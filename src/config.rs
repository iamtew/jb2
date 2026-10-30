use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const TWITCH_COMMAND_IDS: [&str; 4] = ["sr", "q", "si", "jb"];
const COOLDOWN_CAP_SECS: i32 = 86400;

fn default_true() -> bool {
    true
}

fn default_twitch_commands() -> BTreeMap<String, CommandCooldown> {
    filled_twitch_commands(BTreeMap::new())
}

fn filled_twitch_commands(
    mut src: BTreeMap<String, CommandCooldown>,
) -> BTreeMap<String, CommandCooldown> {
    let mut out = BTreeMap::new();
    for id in TWITCH_COMMAND_IDS {
        let mut cd = src.remove(id).unwrap_or_default();
        if cd.gcd < 0 || cd.gcd > COOLDOWN_CAP_SECS {
            cd.gcd = 0;
        }
        if cd.ucd < 0 || cd.ucd > COOLDOWN_CAP_SECS {
            cd.ucd = 0;
        }
        out.insert((*id).to_string(), cd);
    }
    out
}

pub const DEFAULT_BLACKLIST_ENTRY: &str = "Rick Astley";
pub const DEFAULT_MAX_DURATION: i32 = 600;
const MAX_DURATION_CAP: i32 = 86400;
const KV_CONFIG: &str = "config";
pub const OVERLAY_CANVAS_1080: (i32, i32) = (1920, 1080);
pub const OVERLAY_CANVAS_720: (i32, i32) = (1280, 720);

fn default_canvas_w() -> i32 {
    OVERLAY_CANVAS_1080.0
}

fn default_canvas_h() -> i32 {
    OVERLAY_CANVAS_1080.1
}

fn default_overlay_theme() -> String {
    "lcd-glass".to_string()
}

fn default_seek_style() -> String {
    "sesh-sofa".to_string()
}

fn default_playing_opacity() -> f32 {
    1.0
}

fn default_paused_opacity() -> f32 {
    0.25
}

fn default_panel_w() -> i32 {
    500
}

fn default_overlay_pulse() -> i32 {
    3
}

fn default_overlay_grid() -> String {
    "center,thirds".to_string()
}

fn default_viz_y() -> i32 {
    800
}

fn default_viz_w() -> i32 {
    OVERLAY_CANVAS_1080.0
}

fn default_viz_h() -> i32 {
    280
}

fn default_layers() -> String {
    "overlay,viz".to_string()
}

fn default_viz_device() -> String {
    "default".to_string()
}

fn default_viz_bands() -> i32 {
    16
}

fn default_viz_hz() -> i32 {
    30
}

fn default_viz_smoothing() -> f32 {
    0.35
}

fn default_viz_style() -> String {
    "bands".to_string()
}

fn default_viz_color_dir() -> String {
    "lr".to_string()
}

fn default_viz_peak_fall() -> f32 {
    0.35
}

fn default_bass_hz() -> [i32; 2] {
    [20, 250]
}

fn default_mid_hz() -> [i32; 2] {
    [250, 2000]
}

fn default_treble_hz() -> [i32; 2] {
    [2000, 16000]
}

/// Loopback analysis for the overlay canvas. Off until Stage or a config write turns it on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioVisualizer {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_viz_device")]
    pub device: String,
    #[serde(rename = "spectrumBands", default = "default_viz_bands")]
    pub spectrum_bands: i32,
    #[serde(rename = "updateHz", default = "default_viz_hz")]
    pub update_hz: i32,
    #[serde(default = "default_viz_smoothing")]
    pub smoothing: f32,
    #[serde(rename = "bassHz", default = "default_bass_hz")]
    pub bass_hz: [i32; 2],
    #[serde(rename = "midHz", default = "default_mid_hz")]
    pub mid_hz: [i32; 2],
    #[serde(rename = "trebleHz", default = "default_treble_hz")]
    pub treble_hz: [i32; 2],
    /// `bands`, `wave`, `dots`, or `radial`.
    #[serde(default = "default_viz_style")]
    pub style: String,
    /// Seek gradient id. Empty follows the live seek style.
    #[serde(default)]
    pub color: String,
    /// `lr`, `rl`, `tb`, or `bt`.
    #[serde(rename = "colorDir", default = "default_viz_color_dir")]
    pub color_dir: String,
    #[serde(default)]
    pub mirror: bool,
    /// 0 hides peak caps. Higher drops faster.
    #[serde(rename = "peakFall", default = "default_viz_peak_fall")]
    pub peak_fall: f32,
}

impl Default for AudioVisualizer {
    fn default() -> Self {
        Self {
            enabled: false,
            device: default_viz_device(),
            spectrum_bands: default_viz_bands(),
            update_hz: default_viz_hz(),
            smoothing: default_viz_smoothing(),
            bass_hz: default_bass_hz(),
            mid_hz: default_mid_hz(),
            treble_hz: default_treble_hz(),
            style: default_viz_style(),
            color: String::new(),
            color_dir: default_viz_color_dir(),
            mirror: false,
            peak_fall: default_viz_peak_fall(),
        }
    }
}

pub fn normalize_audio_visualizer(mut viz: AudioVisualizer) -> AudioVisualizer {
    viz.device = {
        let name = viz.device.trim();
        if name.is_empty() {
            default_viz_device()
        } else {
            name.to_string()
        }
    };
    viz.spectrum_bands = viz.spectrum_bands.clamp(1, 32);
    viz.style = normalize_viz_style(&viz.style);
    viz.color = normalize_viz_color(&viz.color);
    viz.color_dir = normalize_viz_color_dir(&viz.color_dir);
    if !viz.peak_fall.is_finite() {
        viz.peak_fall = default_viz_peak_fall();
    } else {
        viz.peak_fall = viz.peak_fall.clamp(0.0, 1.0);
    }
    viz.update_hz = match viz.update_hz {
        45 => 45,
        60 => 60,
        _ => 30,
    };
    if !viz.smoothing.is_finite() {
        viz.smoothing = default_viz_smoothing();
    } else {
        viz.smoothing = viz.smoothing.clamp(0.0, 1.0);
    }
    viz.bass_hz = normalize_hz_pair(viz.bass_hz, default_bass_hz());
    viz.mid_hz = normalize_hz_pair(viz.mid_hz, default_mid_hz());
    viz.treble_hz = normalize_hz_pair(viz.treble_hz, default_treble_hz());
    viz
}

fn normalize_viz_style(style: &str) -> String {
    match style.trim() {
        "bands" | "wave" | "dots" | "radial" => style.trim().to_string(),
        _ => default_viz_style(),
    }
}

fn normalize_viz_color(color: &str) -> String {
    match color.trim() {
        "" => String::new(),
        "sesh-sofa" | "ice" | "magenta" | "white" | "sunset" | "forest" | "gold" | "lava"
        | "ocean" | "candy" | "ultraviolet" | "acid" | "copper" | "cherry" | "vapor"
        | "graphite" | "honey" => color.trim().to_string(),
        _ => String::new(),
    }
}

fn normalize_viz_color_dir(dir: &str) -> String {
    match dir.trim() {
        "lr" | "rl" | "tb" | "bt" => dir.trim().to_string(),
        _ => default_viz_color_dir(),
    }
}

fn normalize_hz_pair(pair: [i32; 2], fallback: [i32; 2]) -> [i32; 2] {
    let lo = pair[0];
    let hi = pair[1];
    if lo < 20 || hi > 20000 || lo >= hi {
        fallback
    } else {
        [lo, hi]
    }
}

const OVERLAY_THEMES: [&str; 10] = [
    "lcd-glass",
    "lower-third",
    "amber-terminal",
    "crypt",
    "fraktur",
    "konstrukt",
    "street",
    "drop",
    "capsule",
    "poster",
];
const OVERLAY_SEEK_STYLES: [&str; 17] = [
    "sesh-sofa",
    "ice",
    "magenta",
    "white",
    "sunset",
    "forest",
    "gold",
    "lava",
    "ocean",
    "candy",
    "ultraviolet",
    "acid",
    "copper",
    "cherry",
    "vapor",
    "graphite",
    "honey",
];
const OVERLAY_GRID_PARTS: [&str; 5] = ["center", "golden", "quad", "safe", "thirds"];
const OVERLAY_PANEL_W_MIN: i32 = 280;
const OVERLAY_PANEL_H_MIN: i32 = 80;
const OVERLAY_VIZ_MIN: i32 = 80;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedPlaylist {
    pub id: String,
    pub name: String,
    #[serde(rename = "trackCount")]
    pub track_count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandCooldown {
    #[serde(default)]
    pub gcd: i32,
    #[serde(default)]
    pub ucd: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayLayout {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(rename = "canvasW", default = "default_canvas_w")]
    pub canvas_w: i32,
    #[serde(rename = "canvasH", default = "default_canvas_h")]
    pub canvas_h: i32,
    #[serde(default = "default_overlay_theme")]
    pub theme: String,
    #[serde(rename = "seekStyle", default = "default_seek_style")]
    pub seek_style: String,
    #[serde(rename = "playingOpacity", default = "default_playing_opacity")]
    pub playing_opacity: f32,
    #[serde(rename = "pausedOpacity", default = "default_paused_opacity")]
    pub paused_opacity: f32,
    #[serde(rename = "panelW", default = "default_panel_w")]
    pub panel_w: i32,
    #[serde(rename = "panelH", default)]
    pub panel_h: i32,
    #[serde(default = "default_overlay_pulse")]
    pub pulse: i32,
    #[serde(default = "default_true")]
    pub glass: bool,
    #[serde(default = "default_overlay_grid")]
    pub grid: String,
    /// Stage-only. OBS ignores this.
    #[serde(default = "default_true")]
    pub snap: bool,
    #[serde(rename = "vizX", default)]
    pub viz_x: i32,
    #[serde(rename = "vizY", default = "default_viz_y")]
    pub viz_y: i32,
    #[serde(rename = "vizW", default = "default_viz_w")]
    pub viz_w: i32,
    #[serde(rename = "vizH", default = "default_viz_h")]
    pub viz_h: i32,
    /// Front to back. First id paints above the other.
    #[serde(default = "default_layers")]
    pub layers: String,
}

impl Default for OverlayLayout {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            canvas_w: OVERLAY_CANVAS_1080.0,
            canvas_h: OVERLAY_CANVAS_1080.1,
            theme: default_overlay_theme(),
            seek_style: default_seek_style(),
            playing_opacity: default_playing_opacity(),
            paused_opacity: default_paused_opacity(),
            panel_w: default_panel_w(),
            panel_h: 0,
            pulse: default_overlay_pulse(),
            glass: true,
            grid: default_overlay_grid(),
            snap: true,
            viz_x: 0,
            viz_y: default_viz_y(),
            viz_w: default_viz_w(),
            viz_h: default_viz_h(),
            layers: default_layers(),
        }
    }
}

impl OverlayLayout {
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "x": self.x,
            "y": self.y,
            "canvasW": self.canvas_w,
            "canvasH": self.canvas_h,
            "theme": self.theme,
            "seekStyle": self.seek_style,
            "playingOpacity": self.playing_opacity,
            "pausedOpacity": self.paused_opacity,
            "panelW": self.panel_w,
            "panelH": self.panel_h,
            "pulse": self.pulse,
            "glass": self.glass,
            "grid": self.grid,
            "snap": self.snap,
            "vizX": self.viz_x,
            "vizY": self.viz_y,
            "vizW": self.viz_w,
            "vizH": self.viz_h,
            "layers": self.layers,
        })
    }
}

fn pick_allowed(value: &str, allowed: &[&str], fallback: &str) -> String {
    if allowed.iter().any(|id| *id == value) {
        value.to_string()
    } else {
        fallback.to_string()
    }
}

fn normalize_overlay_grid(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "off" {
        return "off".to_string();
    }
    let mapped = if trimmed == "thirds-center" {
        "center,thirds"
    } else {
        trimmed
    };
    let mut parts: Vec<&str> = mapped
        .split(',')
        .map(str::trim)
        .filter(|id| OVERLAY_GRID_PARTS.contains(id))
        .collect();
    parts.sort_unstable();
    parts.dedup();
    if parts.is_empty() {
        default_overlay_grid()
    } else {
        parts.join(",")
    }
}

pub fn normalize_overlay_layout(mut layout: OverlayLayout) -> OverlayLayout {
    let is_720 = layout.canvas_w == OVERLAY_CANVAS_720.0 && layout.canvas_h == OVERLAY_CANVAS_720.1;
    let is_1080 =
        layout.canvas_w == OVERLAY_CANVAS_1080.0 && layout.canvas_h == OVERLAY_CANVAS_1080.1;
    if !is_720 && !is_1080 {
        if layout.canvas_w == OVERLAY_CANVAS_720.0 || layout.canvas_h == OVERLAY_CANVAS_720.1 {
            layout.canvas_w = OVERLAY_CANVAS_720.0;
            layout.canvas_h = OVERLAY_CANVAS_720.1;
        } else {
            layout.canvas_w = OVERLAY_CANVAS_1080.0;
            layout.canvas_h = OVERLAY_CANVAS_1080.1;
        }
    }
    layout.theme = pick_allowed(&layout.theme, &OVERLAY_THEMES, "lcd-glass");
    if layout.seek_style == "sesh-sofs" {
        layout.seek_style = "sesh-sofa".to_string();
    }
    layout.seek_style = pick_allowed(&layout.seek_style, &OVERLAY_SEEK_STYLES, "sesh-sofa");
    layout.grid = normalize_overlay_grid(&layout.grid);
    if !layout.playing_opacity.is_finite() {
        layout.playing_opacity = 1.0;
    }
    if !layout.paused_opacity.is_finite() {
        layout.paused_opacity = 0.25;
    }
    layout.playing_opacity = layout.playing_opacity.clamp(0.0, 1.0);
    layout.paused_opacity = layout.paused_opacity.clamp(0.0, 1.0);
    layout.pulse = layout.pulse.clamp(0, 10);
    layout.panel_w = layout.panel_w.clamp(OVERLAY_PANEL_W_MIN, layout.canvas_w);
    if layout.panel_h < 0 {
        layout.panel_h = 0;
    } else if layout.panel_h > 0 {
        layout.panel_h = layout.panel_h.clamp(OVERLAY_PANEL_H_MIN, layout.canvas_h);
    }
    let max_x = (layout.canvas_w - layout.panel_w).max(0);
    layout.x = layout.x.clamp(0, max_x);
    let panel_h = if layout.panel_h > 0 {
        layout.panel_h
    } else {
        OVERLAY_PANEL_H_MIN
    };
    let max_y = (layout.canvas_h - panel_h).max(0);
    layout.y = layout.y.clamp(0, max_y);
    layout.viz_w = layout.viz_w.clamp(OVERLAY_VIZ_MIN, layout.canvas_w);
    layout.viz_h = layout.viz_h.clamp(OVERLAY_VIZ_MIN, layout.canvas_h);
    layout.viz_x = layout
        .viz_x
        .clamp(0, (layout.canvas_w - layout.viz_w).max(0));
    layout.viz_y = layout
        .viz_y
        .clamp(0, (layout.canvas_h - layout.viz_h).max(0));
    layout.layers = normalize_layers(&layout.layers);
    layout
}

fn normalize_layers(value: &str) -> String {
    let mut out = Vec::new();
    for id in value.split(',') {
        let id = id.trim();
        if (id == "overlay" || id == "viz") && !out.contains(&id) {
            out.push(id);
        }
    }
    for id in ["overlay", "viz"] {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out.join(",")
}

fn json_f32(v: &Value) -> Option<f32> {
    v.as_f64().map(|n| n as f32)
}

pub fn overlay_layout_from_value(base: OverlayLayout, parsed: &Value) -> OverlayLayout {
    let mut layout = base;
    if let Some(n) = parsed.get("x").and_then(|v| v.as_i64()) {
        layout.x = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("y").and_then(|v| v.as_i64()) {
        layout.y = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("canvasW").and_then(|v| v.as_i64()) {
        layout.canvas_w = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("canvasH").and_then(|v| v.as_i64()) {
        layout.canvas_h = i32::try_from(n).unwrap_or(0);
    }
    if let Some(s) = parsed.get("theme").and_then(|v| v.as_str()) {
        layout.theme = s.to_string();
    }
    if let Some(s) = parsed.get("seekStyle").and_then(|v| v.as_str()) {
        layout.seek_style = s.to_string();
    }
    if let Some(n) = parsed.get("playingOpacity").and_then(json_f32) {
        layout.playing_opacity = n;
    }
    if let Some(n) = parsed.get("pausedOpacity").and_then(json_f32) {
        layout.paused_opacity = n;
    }
    if let Some(n) = parsed.get("panelW").and_then(|v| v.as_i64()) {
        layout.panel_w = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("panelH").and_then(|v| v.as_i64()) {
        layout.panel_h = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("pulse").and_then(|v| v.as_i64()) {
        layout.pulse = i32::try_from(n).unwrap_or(0);
    }
    if let Some(b) = parsed.get("glass").and_then(|v| v.as_bool()) {
        layout.glass = b;
    }
    if let Some(s) = parsed.get("grid").and_then(|v| v.as_str()) {
        layout.grid = s.to_string();
    }
    if let Some(b) = parsed.get("snap").and_then(|v| v.as_bool()) {
        layout.snap = b;
    }
    if let Some(n) = parsed.get("vizX").and_then(|v| v.as_i64()) {
        layout.viz_x = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("vizY").and_then(|v| v.as_i64()) {
        layout.viz_y = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("vizW").and_then(|v| v.as_i64()) {
        layout.viz_w = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = parsed.get("vizH").and_then(|v| v.as_i64()) {
        layout.viz_h = i32::try_from(n).unwrap_or(0);
    }
    if let Some(s) = parsed.get("layers").and_then(|v| v.as_str()) {
        layout.layers = s.to_string();
    }
    normalize_overlay_layout(layout)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub blacklist: Vec<String>,
    #[serde(rename = "maxDuration")]
    pub max_duration: i32,
    #[serde(rename = "seedPlaylists", default)]
    pub seed_playlists: Vec<SeedPlaylist>,
    #[serde(rename = "clearQueueOnRequest", default)]
    pub clear_queue_on_request: bool,
    #[serde(rename = "twitchChannel", default)]
    pub twitch_channel: String,
    #[serde(rename = "twitchOAuth", default)]
    pub twitch_oauth: String,
    #[serde(rename = "twitchNick", default)]
    pub twitch_nick: String,
    #[serde(rename = "twitchClientId", default)]
    pub twitch_client_id: String,
    #[serde(rename = "twitchClientSecret", default)]
    pub twitch_client_secret: String,
    #[serde(rename = "twitchRefreshToken", default)]
    pub twitch_refresh_token: String,
    #[serde(rename = "twitchOAuthExpiresAt", default)]
    pub twitch_oauth_expires_at: i64,
    #[serde(rename = "twitchCommandsEnabled", default = "default_true")]
    pub twitch_commands_enabled: bool,
    #[serde(rename = "twitchCommands", default = "default_twitch_commands")]
    pub twitch_commands: BTreeMap<String, CommandCooldown>,
    #[serde(rename = "overlayLayout", default)]
    pub overlay_layout: OverlayLayout,
    #[serde(rename = "audioVisualizer", default)]
    pub audio_visualizer: AudioVisualizer,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blacklist: vec![DEFAULT_BLACKLIST_ENTRY.to_string()],
            max_duration: DEFAULT_MAX_DURATION,
            seed_playlists: Vec::new(),
            clear_queue_on_request: false,
            twitch_channel: String::new(),
            twitch_oauth: String::new(),
            twitch_nick: String::new(),
            twitch_client_id: String::new(),
            twitch_client_secret: String::new(),
            twitch_refresh_token: String::new(),
            twitch_oauth_expires_at: 0,
            twitch_commands_enabled: true,
            twitch_commands: default_twitch_commands(),
            overlay_layout: OverlayLayout::default(),
            audio_visualizer: AudioVisualizer::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportSource {
    Jb1,
    Jb2,
}

impl ImportSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jb1 => "jb1",
            Self::Jb2 => "jb2",
        }
    }
}

pub fn normalize(mut cfg: Config) -> Config {
    if cfg.blacklist.is_empty() {
        cfg.blacklist = vec![DEFAULT_BLACKLIST_ENTRY.to_string()];
    } else {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for entry in cfg.blacklist {
            let trimmed = entry.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            let key = trimmed.to_lowercase();
            if seen.insert(key) {
                out.push(trimmed);
            }
        }
        cfg.blacklist = if out.is_empty() {
            vec![DEFAULT_BLACKLIST_ENTRY.to_string()]
        } else {
            out
        };
    }
    if cfg.max_duration <= 0 || cfg.max_duration > MAX_DURATION_CAP {
        cfg.max_duration = DEFAULT_MAX_DURATION;
    }
    let mut seen_seed = std::collections::HashSet::new();
    let mut seeds = Vec::new();
    for playlist in cfg.seed_playlists {
        let id = playlist.id.trim().to_string();
        if id.is_empty() {
            continue;
        }
        let key = id.to_lowercase();
        if !seen_seed.insert(key) {
            continue;
        }
        let mut name = playlist.name.trim().to_string();
        if name.is_empty() {
            name = id.clone();
        }
        seeds.push(SeedPlaylist {
            id,
            name,
            track_count: playlist.track_count.max(0),
        });
    }
    cfg.seed_playlists = seeds;
    cfg.twitch_commands = filled_twitch_commands(cfg.twitch_commands);
    cfg.overlay_layout = normalize_overlay_layout(cfg.overlay_layout);
    cfg.audio_visualizer = normalize_audio_visualizer(cfg.audio_visualizer);
    cfg
}

/// Hide live secrets in Admin JSON dumps. `***` is also ignored on save.
pub fn public_json(cfg: &Config) -> serde_json::Value {
    let mut v = serde_json::to_value(cfg).unwrap_or(serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        for key in ["twitchOAuth", "twitchClientSecret", "twitchRefreshToken"] {
            let nonempty = obj
                .get(key)
                .and_then(|x| x.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if nonempty {
                obj.insert(key.to_string(), serde_json::json!("***"));
            }
        }
    }
    v
}

pub fn load_json(path: &Path) -> Result<Config, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Ok(normalize(Config::default()));
    }
    let cfg: Config = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(normalize(cfg))
}

pub fn detect_import(parsed: &Value) -> Result<ImportSource, String> {
    let obj = parsed
        .as_object()
        .ok_or_else(|| "import must be a JSON object".to_string())?;
    if obj.keys().any(|k| k.starts_with("twitch")) {
        return Ok(ImportSource::Jb2);
    }
    if [
        "blacklist",
        "maxDuration",
        "seedPlaylists",
        "clearQueueOnRequest",
    ]
    .iter()
    .any(|k| obj.contains_key(*k))
    {
        return Ok(ImportSource::Jb1);
    }
    Err("unrecognized config file (need JB1 jukeboks.json or JB2 jb2.json)".into())
}

/// Empty / `***` means keep the stored secret so a blank Admin field or a JB1 dump cannot wipe it.
fn patch_secret(dst: &mut String, parsed: &Value, key: &str) {
    let Some(s) = parsed.get(key).and_then(|v| v.as_str()) else {
        return;
    };
    let token = s.trim();
    if token.is_empty() || token == "***" {
        return;
    }
    *dst = token.to_string();
}

/// Overwrite only the fields present in `parsed` (Admin save and JB1/JB2 import share this).
/// The store normalizes on persist, so out-of-range values fall back to defaults there.
pub fn apply_patch(cfg: &mut Config, parsed: &Value) {
    if let Some(list) = parsed.get("blacklist").and_then(|v| v.as_array()) {
        cfg.blacklist = list
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(n) = parsed.get("maxDuration").and_then(|v| v.as_i64()) {
        // Out-of-range becomes 0, which normalize resets to the default instead of wrapping.
        cfg.max_duration = i32::try_from(n).unwrap_or(0);
    }
    if let Some(list) = parsed.get("seedPlaylists").and_then(|v| v.as_array()) {
        cfg.seed_playlists = list
            .iter()
            .filter_map(|v| serde_json::from_value::<SeedPlaylist>(v.clone()).ok())
            .collect();
    }
    if let Some(b) = parsed.get("clearQueueOnRequest").and_then(|v| v.as_bool()) {
        cfg.clear_queue_on_request = b;
    }
    if let Some(s) = parsed.get("twitchChannel").and_then(|v| v.as_str()) {
        cfg.twitch_channel = s.trim().trim_start_matches('#').to_string();
    }
    if let Some(s) = parsed.get("twitchNick").and_then(|v| v.as_str()) {
        cfg.twitch_nick = s.trim().to_string();
    }
    if let Some(s) = parsed.get("twitchClientId").and_then(|v| v.as_str()) {
        cfg.twitch_client_id = s.trim().to_string();
    }
    patch_secret(&mut cfg.twitch_oauth, parsed, "twitchOAuth");
    patch_secret(&mut cfg.twitch_client_secret, parsed, "twitchClientSecret");
    patch_secret(&mut cfg.twitch_refresh_token, parsed, "twitchRefreshToken");
    if let Some(n) = parsed.get("twitchOAuthExpiresAt").and_then(|v| v.as_i64()) {
        cfg.twitch_oauth_expires_at = n;
    }
    if let Some(b) = parsed
        .get("twitchCommandsEnabled")
        .and_then(|v| v.as_bool())
    {
        cfg.twitch_commands_enabled = b;
    }
    if let Some(obj) = parsed.get("twitchCommands").and_then(|v| v.as_object()) {
        for id in TWITCH_COMMAND_IDS {
            let Some(v) = obj.get(id) else {
                continue;
            };
            let mut cd = cfg.twitch_commands.get(id).cloned().unwrap_or_default();
            if let Some(n) = v.get("gcd").and_then(|x| x.as_i64()) {
                cd.gcd = i32::try_from(n).unwrap_or(0);
            }
            if let Some(n) = v.get("ucd").and_then(|x| x.as_i64()) {
                cd.ucd = i32::try_from(n).unwrap_or(0);
            }
            cfg.twitch_commands.insert((*id).to_string(), cd);
        }
    }
    if let Some(v) = parsed.get("overlayLayout") {
        cfg.overlay_layout = overlay_layout_from_value(cfg.overlay_layout.clone(), v);
    }
    if let Some(v) = parsed.get("audioVisualizer") {
        apply_audio_patch(&mut cfg.audio_visualizer, v);
    }
}

fn apply_audio_patch(dst: &mut AudioVisualizer, parsed: &Value) {
    let Some(obj) = parsed.as_object() else {
        return;
    };
    if let Some(b) = obj.get("enabled").and_then(|v| v.as_bool()) {
        dst.enabled = b;
    }
    if let Some(s) = obj.get("device").and_then(|v| v.as_str()) {
        dst.device = s.to_string();
    }
    if let Some(n) = obj.get("spectrumBands").and_then(|v| v.as_i64()) {
        dst.spectrum_bands = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = obj.get("updateHz").and_then(|v| v.as_i64()) {
        dst.update_hz = i32::try_from(n).unwrap_or(0);
    }
    if let Some(n) = obj.get("smoothing").and_then(json_f32) {
        dst.smoothing = n;
    }
    if let Some(pair) = obj.get("bassHz").and_then(json_hz_pair) {
        dst.bass_hz = pair;
    }
    if let Some(pair) = obj.get("midHz").and_then(json_hz_pair) {
        dst.mid_hz = pair;
    }
    if let Some(pair) = obj.get("trebleHz").and_then(json_hz_pair) {
        dst.treble_hz = pair;
    }
    if let Some(s) = obj.get("style").and_then(|v| v.as_str()) {
        dst.style = s.to_string();
    }
    if let Some(s) = obj.get("color").and_then(|v| v.as_str()) {
        dst.color = s.to_string();
    }
    if let Some(s) = obj.get("colorDir").and_then(|v| v.as_str()) {
        dst.color_dir = s.to_string();
    }
    if let Some(b) = obj.get("mirror").and_then(|v| v.as_bool()) {
        dst.mirror = b;
    }
    if let Some(n) = obj.get("peakFall").and_then(json_f32) {
        dst.peak_fall = n;
    }
}

fn json_hz_pair(v: &Value) -> Option<[i32; 2]> {
    let arr = v.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    let lo = i32::try_from(arr[0].as_i64()?).ok()?;
    let hi = i32::try_from(arr[1].as_i64()?).ok()?;
    Some([lo, hi])
}

/// True when a patch should bounce the IRC socket (channel / nick / tokens only).
pub fn touches_twitch(parsed: &Value) -> bool {
    parsed
        .as_object()
        .map(|o| {
            [
                "twitchChannel",
                "twitchOAuth",
                "twitchNick",
                "twitchClientId",
                "twitchClientSecret",
                "twitchRefreshToken",
                "twitchOAuthExpiresAt",
            ]
            .iter()
            .any(|k| o.contains_key(*k))
        })
        .unwrap_or(false)
}

fn open_db(path: &Path) -> Result<Connection, String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv (k TEXT PRIMARY KEY NOT NULL, v TEXT NOT NULL)",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn kv_get(conn: &Connection, k: &str) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("SELECT v FROM kv WHERE k = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![k]).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(row.get(0).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

fn kv_set(conn: &Connection, k: &str, v: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO kv(k, v) VALUES (?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
        params![k, v],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn persist(conn: &Connection, cfg: &Config) -> Result<Config, String> {
    let cfg = normalize(cfg.clone());
    let data = serde_json::to_string(&cfg).map_err(|e| e.to_string())?;
    kv_set(conn, KV_CONFIG, &data)?;
    Ok(cfg)
}

fn parse_stored(s: &str) -> Result<Config, String> {
    let cfg: Config = serde_json::from_str(s).map_err(|e| e.to_string())?;
    Ok(normalize(cfg))
}

struct Inner {
    conn: Connection,
    cfg: Config,
}

pub struct Store {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl Store {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let conn = open_db(&path)?;
        let cfg = match kv_get(&conn, KV_CONFIG)? {
            Some(s) if !s.trim().is_empty() => parse_stored(&s)?,
            _ => {
                let sibling = path.with_file_name("jb2.json");
                let cfg = if sibling.is_file() {
                    load_json(&sibling)
                        .map_err(|e| format!("failed to import {}: {e}", sibling.display()))?
                } else {
                    Config::default()
                };
                persist(&conn, &cfg)?
            }
        };
        Ok(Self {
            path,
            inner: Mutex::new(Inner { conn, cfg }),
        })
    }

    pub fn get(&self) -> Result<Config, String> {
        let guard = self.inner.lock().expect("config lock");
        Ok(guard.cfg.clone())
    }

    /// Read-modify-write under the lock. Always patch the latest config here instead of
    /// `get` + edit + save: a stale copy held across a network call would wipe other edits.
    /// Do slow work (HTTP) before calling this; the closure should only assign fields.
    pub fn update<F>(&self, f: F) -> Result<Config, String>
    where
        F: FnOnce(&mut Config) -> Result<(), String>,
    {
        let mut guard = self.inner.lock().expect("config lock");
        let mut cfg = guard.cfg.clone();
        f(&mut cfg)?;
        let saved =
            persist(&guard.conn, &cfg).map_err(|e| format!("{}: {e}", self.path.display()))?;
        guard.cfg = saved.clone();
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn import_and_sqlite_roundtrip() {
        let mut cfg = Config {
            twitch_oauth: "keep-me".into(),
            twitch_channel: "streamer".into(),
            ..Config::default()
        };

        let jb1 = json!({
            "blacklist": ["Metallica"],
            "maxDuration": 120,
            "seedPlaylists": [{"id": "PL1", "name": "Mix", "trackCount": 3}],
            "clearQueueOnRequest": true
        });
        assert_eq!(detect_import(&jb1).unwrap(), ImportSource::Jb1);
        apply_patch(&mut cfg, &jb1);
        assert_eq!(cfg.blacklist, vec!["Metallica"]);
        assert_eq!(cfg.max_duration, 120);
        assert_eq!(cfg.seed_playlists[0].id, "PL1");
        assert!(cfg.clear_queue_on_request);
        assert_eq!(cfg.twitch_oauth, "keep-me");
        assert_eq!(cfg.twitch_channel, "streamer");

        let jb2 = json!({
            "twitchChannel": "other",
            "twitchOAuth": "***",
            "twitchClientSecret": "",
            "blacklist": ["Foo"],
            "maxDuration": 4294967416i64
        });
        assert_eq!(detect_import(&jb2).unwrap(), ImportSource::Jb2);
        apply_patch(&mut cfg, &jb2);
        let cfg2 = normalize(cfg);
        assert_eq!(cfg2.twitch_channel, "other");
        assert_eq!(cfg2.twitch_oauth, "keep-me");
        assert_eq!(cfg2.blacklist, vec!["Foo"]);
        // i64 that would wrap to 120 as i32 must fall back to the default instead.
        assert_eq!(cfg2.max_duration, DEFAULT_MAX_DURATION);

        let path = std::env::temp_dir().join(format!(
            "jb2-test-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&path);
        let store = Store::new(path.clone()).expect("open sqlite");
        store
            .update(|c| {
                *c = cfg2;
                Ok(())
            })
            .expect("save");

        // Lost-update guard: an Admin edit landing while a token refresh is in flight
        // must survive the refresh writing its own fields afterwards.
        let _stale = store.get().expect("get");
        store
            .update(|c| {
                c.blacklist = vec!["Admin edit".into()];
                Ok(())
            })
            .expect("admin edit");
        store
            .update(|c| {
                c.twitch_oauth = "fresh-token".into();
                Ok(())
            })
            .expect("token refresh");
        drop(store);

        let store2 = Store::new(path.clone()).expect("reopen sqlite");
        let loaded = store2.get().expect("get");
        assert_eq!(loaded.twitch_channel, "other");
        assert_eq!(loaded.twitch_oauth, "fresh-token");
        assert_eq!(loaded.blacklist, vec!["Admin edit"]);
        drop(store2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_fields_do_not_reconnect_irc() {
        assert!(!touches_twitch(&json!({
            "twitchCommandsEnabled": false,
            "twitchCommands": {"sr": {"gcd": 5, "ucd": 10}}
        })));
        assert!(touches_twitch(&json!({"twitchChannel": "x"})));
    }

    #[test]
    fn twitch_commands_normalize_and_patch() {
        let mut cfg = Config::default();
        assert!(cfg.twitch_commands_enabled);
        assert_eq!(cfg.twitch_commands["sr"].gcd, 0);
        apply_patch(
            &mut cfg,
            &json!({
                "twitchCommandsEnabled": false,
                "twitchCommands": {"sr": {"gcd": 12, "ucd": -3}, "nope": {"gcd": 9}}
            }),
        );
        let cfg = normalize(cfg);
        assert!(!cfg.twitch_commands_enabled);
        assert_eq!(cfg.twitch_commands["sr"].gcd, 12);
        assert_eq!(cfg.twitch_commands["sr"].ucd, 0);
        assert!(!cfg.twitch_commands.contains_key("nope"));
    }

    #[test]
    fn overlay_layout_normalize_clamps_and_fixes_canvas() {
        let bogus = normalize_overlay_layout(OverlayLayout {
            x: -40,
            y: 9000,
            canvas_w: 800,
            canvas_h: 600,
            theme: "nope".into(),
            seek_style: "nope".into(),
            playing_opacity: 4.0,
            paused_opacity: -1.0,
            panel_w: 40,
            pulse: 99,
            grid: "nope".into(),
            ..Default::default()
        });
        assert_eq!(bogus.canvas_w, OVERLAY_CANVAS_1080.0);
        assert_eq!(bogus.canvas_h, OVERLAY_CANVAS_1080.1);
        assert_eq!(bogus.x, 0);
        assert_eq!(bogus.y, OVERLAY_CANVAS_1080.1 - OVERLAY_PANEL_H_MIN);
        assert_eq!(bogus.theme, "lcd-glass");
        assert_eq!(bogus.seek_style, "sesh-sofa");
        assert_eq!(bogus.grid, "center,thirds");
        assert!(bogus.snap);
        assert_eq!(bogus.playing_opacity, 1.0);
        assert_eq!(bogus.paused_opacity, 0.0);
        assert_eq!(bogus.panel_w, OVERLAY_PANEL_W_MIN);
        assert_eq!(bogus.pulse, 10);

        let hinted = normalize_overlay_layout(OverlayLayout {
            x: 10,
            y: 20,
            canvas_w: 1280,
            canvas_h: 1,
            ..Default::default()
        });
        assert_eq!(hinted.canvas_w, OVERLAY_CANVAS_720.0);
        assert_eq!(hinted.canvas_h, OVERLAY_CANVAS_720.1);
        assert_eq!(hinted.x, 10);
        assert_eq!(hinted.y, 20);

        let kept = normalize_overlay_layout(OverlayLayout {
            theme: "crypt".into(),
            seek_style: "vapor".into(),
            ..Default::default()
        });
        assert_eq!(kept.theme, "crypt");
        assert_eq!(kept.seek_style, "vapor");

        let mut cfg = Config::default();
        apply_patch(
            &mut cfg,
            &json!({
                "overlayLayout": {
                    "x": 50,
                    "y": 60,
                    "canvasW": 1280,
                    "canvasH": 720,
                    "theme": "amber-terminal",
                    "seekStyle": "ice",
                    "playingOpacity": 0.8,
                    "pausedOpacity": 0.1,
                    "panelW": 400,
                    "pulse": 0,
                    "glass": false,
                    "grid": "safe"
                }
            }),
        );
        let mut cfg = normalize(cfg);
        assert_eq!(cfg.overlay_layout.x, 50);
        assert_eq!(cfg.overlay_layout.canvas_w, 1280);
        assert_eq!(cfg.overlay_layout.theme, "amber-terminal");
        assert_eq!(cfg.overlay_layout.seek_style, "ice");
        assert!((cfg.overlay_layout.playing_opacity - 0.8).abs() < 0.001);
        assert!((cfg.overlay_layout.paused_opacity - 0.1).abs() < 0.001);
        assert_eq!(cfg.overlay_layout.panel_w, 400);
        assert_eq!(cfg.overlay_layout.pulse, 0);
        assert!(!cfg.overlay_layout.glass);
        assert_eq!(cfg.overlay_layout.grid, "safe");
        assert!(cfg.overlay_layout.snap);

        apply_patch(
            &mut cfg,
            &json!({
                "overlayLayout": { "snap": false }
            }),
        );
        let cfg = normalize(cfg);
        assert!(!cfg.overlay_layout.snap);

        let alias = normalize_overlay_layout(OverlayLayout {
            grid: "thirds-center".into(),
            ..Default::default()
        });
        assert_eq!(alias.grid, "center,thirds");

        let combo = normalize_overlay_layout(OverlayLayout {
            grid: "thirds,golden,nope".into(),
            ..Default::default()
        });
        assert_eq!(combo.grid, "golden,thirds");

        let off = normalize_overlay_layout(OverlayLayout {
            grid: "off".into(),
            ..Default::default()
        });
        assert_eq!(off.grid, "off");

        let stacked = normalize_overlay_layout(OverlayLayout {
            layers: "nope,viz".into(),
            viz_w: 5000,
            viz_h: 10,
            viz_x: -5,
            viz_y: 9000,
            ..Default::default()
        });
        assert_eq!(stacked.layers, "viz,overlay");
        assert_eq!(stacked.viz_w, OVERLAY_CANVAS_1080.0);
        assert_eq!(stacked.viz_h, OVERLAY_VIZ_MIN);
        assert_eq!(stacked.viz_x, 0);
        assert_eq!(stacked.viz_y, OVERLAY_CANVAS_1080.1 - OVERLAY_VIZ_MIN);
    }

    #[test]
    fn audio_visualizer_normalizes_junk() {
        let mut cfg = Config::default();
        assert!(!cfg.audio_visualizer.enabled);
        assert_eq!(cfg.audio_visualizer.spectrum_bands, 16);
        apply_patch(
            &mut cfg,
            &json!({
                "audioVisualizer": {
                    "enabled": true,
                    "device": "  ",
                    "spectrumBands": 7,
                    "updateHz": 99,
                    "smoothing": 4.0,
                    "bassHz": [10, 10],
                    "midHz": [300, 1500]
                }
            }),
        );
        let cfg = normalize(cfg);
        assert!(cfg.audio_visualizer.enabled);
        assert_eq!(cfg.audio_visualizer.device, "default");
        assert_eq!(cfg.audio_visualizer.spectrum_bands, 7);
        assert_eq!(cfg.audio_visualizer.style, "bands");
        assert_eq!(cfg.audio_visualizer.color, "");
        assert_eq!(cfg.audio_visualizer.color_dir, "lr");
        assert!(!cfg.audio_visualizer.mirror);
        assert_eq!(cfg.audio_visualizer.peak_fall, 0.35);
        assert_eq!(cfg.audio_visualizer.update_hz, 30);
        assert_eq!(cfg.audio_visualizer.smoothing, 1.0);
        assert_eq!(cfg.audio_visualizer.bass_hz, [20, 250]);
        assert_eq!(cfg.audio_visualizer.mid_hz, [300, 1500]);
        assert_eq!(cfg.audio_visualizer.treble_hz, [2000, 16000]);

        let mut cfg = cfg;
        apply_patch(
            &mut cfg,
            &json!({
                "audioVisualizer": {
                    "spectrumBands": 0,
                    "style": "nope",
                    "color": "nope",
                    "colorDir": "nope",
                    "mirror": true,
                    "peakFall": 4.0
                }
            }),
        );
        let cfg = normalize(cfg);
        assert_eq!(cfg.audio_visualizer.spectrum_bands, 1);
        assert_eq!(cfg.audio_visualizer.style, "bands");
        assert_eq!(cfg.audio_visualizer.color, "");
        assert_eq!(cfg.audio_visualizer.color_dir, "lr");
        assert!(cfg.audio_visualizer.mirror);
        assert_eq!(cfg.audio_visualizer.peak_fall, 1.0);

        let mut cfg = cfg;
        apply_patch(
            &mut cfg,
            &json!({
                "audioVisualizer": {
                    "spectrumBands": 99,
                    "style": "wave",
                    "color": "ice",
                    "colorDir": "rl",
                    "peakFall": -1.0
                }
            }),
        );
        let cfg = normalize(cfg);
        assert_eq!(cfg.audio_visualizer.spectrum_bands, 32);
        assert_eq!(cfg.audio_visualizer.style, "wave");
        assert_eq!(cfg.audio_visualizer.color, "ice");
        assert_eq!(cfg.audio_visualizer.color_dir, "rl");
        assert_eq!(cfg.audio_visualizer.peak_fall, 0.0);
    }
}
