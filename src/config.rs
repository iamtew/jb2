use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

pub const DEFAULT_BLACKLIST_ENTRY: &str = "Rick Astley";
pub const DEFAULT_MAX_DURATION: i32 = 600;
const MAX_DURATION_CAP: i32 = 86400;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedPlaylist {
    pub id: String,
    pub name: String,
    #[serde(rename = "trackCount")]
    pub track_count: i32,
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

pub fn load(path: &Path) -> Result<Config, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Ok(normalize(Config {
            blacklist: vec![DEFAULT_BLACKLIST_ENTRY.to_string()],
            ..Config::default()
        }));
    }
    let cfg: Config = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(normalize(cfg))
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    let cfg = normalize(cfg.clone());
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let mut data = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    data.push('\n');
    fs::write(path, data).map_err(|e| e.to_string())
}

pub fn ensure(path: &Path) -> Result<Config, String> {
    match load(path) {
        Ok(cfg) => Ok(cfg),
        Err(_) if !path.exists() => {
            let cfg = Config::default();
            save(path, &cfg)?;
            Ok(cfg)
        }
        Err(e) => Err(e),
    }
}

pub struct Store {
    path: PathBuf,
    inner: Mutex<(Config, Option<SystemTime>)>,
}

impl Store {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let cfg = ensure(&path)?;
        let mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        Ok(Self {
            path,
            inner: Mutex::new((cfg, mtime)),
        })
    }

    pub fn get(&self) -> Result<Config, String> {
        let mut guard = self.inner.lock().expect("config lock");
        let mtime = fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        if mtime != guard.1 {
            let cfg = load(&self.path)?;
            *guard = (cfg.clone(), mtime);
            return Ok(cfg);
        }
        Ok(guard.0.clone())
    }

    pub fn save(&self, cfg: Config) -> Result<(), String> {
        let mut guard = self.inner.lock().expect("config lock");
        save(&self.path, &cfg)?;
        let loaded = load(&self.path)?;
        let mtime = fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        *guard = (loaded, mtime);
        Ok(())
    }
}
