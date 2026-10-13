use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const DEFAULT_BLACKLIST_ENTRY: &str = "Rick Astley";
pub const DEFAULT_MAX_DURATION: i32 = 600;
const MAX_DURATION_CAP: i32 = 86400;
const KV_CONFIG: &str = "config";

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
}

/// True when a patch touches Twitch fields, so the IRC worker should reconnect.
pub fn touches_twitch(parsed: &Value) -> bool {
    parsed
        .as_object()
        .map(|o| o.keys().any(|k| k.starts_with("twitch")))
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
}
