use crate::jsonutil::as_string;
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;

mod parse;
pub use parse::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
/// Public web key the YouTube Music site itself ships; not an operator secret.
const INNERTUBE_BROWSE: &str =
    "https://music.youtube.com/youtubei/v1/browse?alt=json&key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";
pub const INNERTUBE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct Client {
    pub base: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(host: &str, port: &str) -> Self {
        let agent = ureq::AgentBuilder::new().timeout(DEFAULT_TIMEOUT).build();
        Self {
            base: format!("http://{host}:{port}"),
            agent,
        }
    }

    pub fn fetch_json_retry(
        &self,
        endpoint: &str,
        attempts: usize,
        delay: Duration,
    ) -> Result<Value, String> {
        let attempts = attempts.max(1);
        let mut last = "no attempts".to_string();
        for attempt in 0..attempts {
            match self.get_json(endpoint) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last = e;
                    if attempt + 1 < attempts {
                        std::thread::sleep(delay);
                    }
                }
            }
        }
        Err(last)
    }

    fn get_json(&self, endpoint: &str) -> Result<Value, String> {
        let url = format!("{}{endpoint}", self.base);
        let resp = self.agent.get(&url).call().map_err(|e| e.to_string())?;
        if resp.status() >= 400 {
            return Err(format!("endpoint returned {}", resp.status()));
        }
        resp.into_json()
            .map_err(|e| format!("failed to decode payload: {e}"))
    }

    pub fn post_json(&self, endpoint: &str, body: Value) -> Result<Value, String> {
        self.request_json("POST", endpoint, Some(body))
    }

    pub fn request_json(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let url = format!("{}{endpoint}", self.base);
        let req = self.agent.request(method, &url);
        let resp = if let Some(body) = body {
            req.set("Content-Type", "application/json")
                .send_json(body)
                .map_err(|e| e.to_string())?
        } else {
            req.call().map_err(|e| e.to_string())?
        };
        if resp.status() >= 400 {
            let status = resp.status();
            let text = resp.into_string().unwrap_or_default();
            if text.is_empty() {
                return Err(format!("endpoint returned {status}"));
            }
            return Err(format!("endpoint returned {status}: {text}"));
        }
        let text = resp.into_string().map_err(|e| e.to_string())?;
        if text.trim().is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).map_err(|e| format!("failed to decode payload: {e}"))
    }

    pub fn proxy(
        &self,
        method: &str,
        path: &str,
        query: &str,
        body: Option<Vec<u8>>,
    ) -> Result<(u16, Vec<u8>), String> {
        let mut url = format!("{}{path}", self.base);
        if !query.is_empty() {
            url.push('?');
            url.push_str(query);
        }
        let mut req = self.agent.request(method, &url);
        let resp = if let Some(body) = body {
            if method != "GET" {
                req = req.set("Content-Type", "application/json");
            }
            req.send_bytes(&body).map_err(|e| e.to_string())?
        } else {
            req.call().map_err(|e| e.to_string())?
        };
        let status = resp.status();
        let mut buf = Vec::new();
        resp.into_reader()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        Ok((status, buf))
    }
}

pub fn route_for_path(path: &str) -> String {
    let mut path = path.strip_prefix("/cmd/ytmd").unwrap_or(path).to_string();
    if path.ends_with('/') {
        path.pop();
    }
    if path.is_empty() {
        return "/api/v1/".into();
    }
    let mut segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.len() > 1 {
        let last = segments[segments.len() - 1].to_ascii_lowercase();
        if is_supported_method(&last) {
            segments.pop();
        }
    }
    let trimmed = format!("/{}", segments.join("/"));
    if trimmed == "/" {
        return "/api/v1/".into();
    }
    if let Some(rest) = trimmed.strip_prefix("/queue/") {
        return format!("/api/v1/queue/{rest}");
    }
    format!("/api/v1{trimmed}")
}

pub fn resolve_upstream_method(request_method: &str, path: &str) -> String {
    let mut command = path.strip_prefix("/cmd/ytmd").unwrap_or(path).to_string();
    if command.ends_with('/') {
        command.pop();
    }
    if command.is_empty() {
        return request_method.to_string();
    }
    let segments: Vec<&str> = command.trim_start_matches('/').split('/').collect();
    if let Some(last) = segments.last() {
        let last = last.to_ascii_lowercase();
        if is_supported_method(&last) {
            let method = last.to_ascii_uppercase();
            if method == "GET" {
                return request_method.to_string();
            }
            return method;
        }
    }
    if command == "/queue/{index}" || command == "/queue/index" {
        if request_method.eq_ignore_ascii_case("GET") {
            return "PATCH".into();
        }
        return request_method.to_string();
    }
    match command.as_str() {
        "/play" | "/pause" | "/toggle-play" | "/previous" | "/next" | "/seek-to" | "/go-back"
        | "/go-forward" | "/toggle-mute" | "/switch-repeat" | "/like" | "/dislike" | "/volume"
        | "/fullscreen" => "POST".into(),
        "/shuffle" => {
            if request_method.eq_ignore_ascii_case("POST") {
                "POST".into()
            } else {
                "GET".into()
            }
        }
        "/queue" => {
            if matches!(
                request_method.to_ascii_uppercase().as_str(),
                "POST" | "PATCH" | "DELETE"
            ) {
                request_method.to_ascii_uppercase()
            } else {
                "GET".into()
            }
        }
        _ => request_method.to_string(),
    }
}

fn is_supported_method(segment: &str) -> bool {
    matches!(
        segment.to_ascii_lowercase().as_str(),
        "get" | "post" | "put" | "patch" | "delete"
    )
}

pub fn coerce_query_value(value: &str) -> Value {
    if value.is_empty() {
        return Value::String(value.into());
    }
    if value.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if value.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    if let Ok(n) = value.parse::<i64>() {
        return json!(n);
    }
    if let Ok(n) = value.parse::<f64>() {
        return json!(n);
    }
    Value::String(value.into())
}

pub fn build_upstream_body(
    method: &str,
    query: &[(String, String)],
    body: &[u8],
) -> Option<Vec<u8>> {
    let mutating = matches!(
        method.to_ascii_uppercase().as_str(),
        "POST" | "PUT" | "PATCH" | "DELETE"
    );
    if !mutating {
        if body.is_empty() {
            return None;
        }
        return Some(body.to_vec());
    }
    if !body.iter().all(|b| b.is_ascii_whitespace()) {
        return Some(body.to_vec());
    }
    if query.is_empty() {
        return None;
    }
    let mut payload = serde_json::Map::new();
    let mut grouped: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (k, v) in query {
        grouped.entry(k.clone()).or_default().push(v.clone());
    }
    for (k, values) in grouped {
        if values.len() == 1 {
            payload.insert(k, coerce_query_value(&values[0]));
        } else {
            payload.insert(k, json!(values));
        }
    }
    Some(Value::Object(payload).to_string().into_bytes())
}

#[derive(Clone, Debug, Default)]
pub struct SpotifyOembed {
    pub query: String,
    pub title: String,
    pub artist: String,
    pub thumbnail: String,
}

pub fn lookup_spotify_track_query(track_url: &str) -> Result<SpotifyOembed, String> {
    let oembed = format!(
        "https://open.spotify.com/oembed?url={}",
        url::form_urlencoded::byte_serialize(track_url.as_bytes()).collect::<String>()
    );
    let resp = ureq::get(&oembed)
        .set("Accept", "application/json")
        .set("User-Agent", INNERTUBE_UA)
        .timeout(DEFAULT_TIMEOUT)
        .call()
        .map_err(|e| format!("spotify oEmbed: {e}"))?;
    if resp.status() != 200 {
        return Err(format!("spotify oEmbed: HTTP {}", resp.status()));
    }
    let payload: Value = resp
        .into_json()
        .map_err(|e| format!("spotify oEmbed: {e}"))?;
    let title = as_string(payload.get("title").unwrap_or(&Value::Null));
    let author = as_string(payload.get("author_name").unwrap_or(&Value::Null));
    let thumbnail = as_string(payload.get("thumbnail_url").unwrap_or(&Value::Null));
    let query = format!("{title} {author}").trim().to_string();
    if query.is_empty() {
        return Err("spotify oEmbed: empty title and artist".into());
    }
    Ok(SpotifyOembed {
        query,
        title,
        artist: author,
        thumbnail,
    })
}

fn innertube_client_version() -> String {
    let now = chrono_lite_ymd();
    format!("1.{now}.01.00")
}

fn chrono_lite_ymd() -> String {
    // YYYYMMDD in UTC without extra crate
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // civil date from unix days (proleptic Gregorian)
    let z = (secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}")
}

fn browse_id_for_playlist(playlist_id: &str) -> String {
    if playlist_id.starts_with("VL") {
        playlist_id.to_string()
    } else {
        format!("VL{playlist_id}")
    }
}

pub fn post_innertube_browse(browse_id: &str) -> Result<Value, String> {
    let body = json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": innertube_client_version(),
                "hl": "en",
                "gl": "US"
            },
            "user": {}
        },
        "browseId": browse_id
    });
    let resp = ureq::post(INNERTUBE_BROWSE)
        .set("Content-Type", "application/json")
        .set("User-Agent", INNERTUBE_UA)
        .set("Origin", "https://music.youtube.com")
        .set("X-Goog-Api-Format-Version", "2")
        .timeout(Duration::from_secs(15))
        .send_json(body)
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        let status = resp.status();
        let text = resp.into_string().unwrap_or_default();
        if text.is_empty() {
            return Err(format!("innertube browse returned {status}"));
        }
        return Err(format!("innertube browse returned {status}: {text}"));
    }
    resp.into_json()
        .map_err(|e| format!("failed to decode innertube response: {e}"))
}

pub fn lookup_playlist_by_id(client: &Client, playlist_id: &str) -> Result<PlaylistLookup, String> {
    let mut attempts = Vec::new();
    match lookup_playlist_via_innertube(playlist_id) {
        Ok(v) => return Ok(v),
        Err(e) => attempts.push(e),
    }
    match lookup_playlist_via_ytmd_search(client, playlist_id) {
        Ok(v) => return Ok(v),
        Err(e) => attempts.push(e),
    }
    let detail = if attempts.is_empty() {
        "playlist must be public or viewable in YTMD".into()
    } else {
        attempts.join("; ")
    };
    Err(format!(
        "no tracks found for playlist \"{playlist_id}\" ({detail})"
    ))
}

fn lookup_playlist_via_innertube(playlist_id: &str) -> Result<PlaylistLookup, String> {
    let payload = post_innertube_browse(&browse_id_for_playlist(playlist_id))?;
    let mut lookup = parse_playlist_payload(playlist_id, &payload)
        .ok_or_else(|| format!("innertube returned no tracks for playlist \"{playlist_id}\""))?;
    if lookup.tracks.is_empty() {
        return Err(format!(
            "innertube returned no tracks for playlist \"{playlist_id}\""
        ));
    }
    if lookup.name.trim().eq_ignore_ascii_case(playlist_id) {
        if let Some(album_id) = extract_album_browse_id(&payload) {
            if let Ok(album_payload) = post_innertube_browse(&album_id) {
                let name = extract_playlist_name(&album_payload);
                if !name.trim().is_empty() {
                    lookup.name = name;
                }
            }
        }
    }
    Ok(lookup)
}

fn lookup_playlist_via_ytmd_search(
    client: &Client,
    playlist_id: &str,
) -> Result<PlaylistLookup, String> {
    let queries = [
        crate::videoid::playlist_url(playlist_id),
        playlist_id.to_string(),
    ];
    let mut last_err = None;
    for query in queries {
        match client.post_json("/api/v1/search", json!({"query": query})) {
            Ok(payload) => {
                if let Some(lookup) = parse_playlist_payload(playlist_id, &payload) {
                    if !lookup.tracks.is_empty() {
                        return Ok(lookup);
                    }
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    if let Some(e) = last_err {
        return Err(e);
    }
    Err(format!(
        "search returned no tracks for playlist \"{playlist_id}\""
    ))
}
