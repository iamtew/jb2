use crate::envelope::Envelope;
use crate::jsonutil::{as_bool, as_f64, as_string, nested_map};
use crate::ytmd::Client;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Default)]
pub struct CurrentSong {
    pub title: String,
    pub artist: String,
    pub state: String,
}

#[derive(Clone, Debug, Default)]
pub struct SongLookup {
    pub title: String,
    pub artist: String,
    pub duration: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    #[serde(rename = "videoId")]
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminQueueItem {
    pub title: String,
    pub artist: String,
    #[serde(rename = "videoId", skip_serializing_if = "String::is_empty")]
    pub video_id: String,
    #[serde(rename = "queueIndex")]
    pub queue_index: i32,
    pub kind: String,
}

#[derive(Clone, Debug, Default)]
pub struct QueueEntrySummary {
    pub title: String,
    pub artist: String,
    pub video_id: String,
    pub duration_seconds: i32,
    pub duration_label: String,
    pub display_text: String,
}

#[derive(Clone, Debug)]
pub struct TrackLookup {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration: i32,
}

#[derive(Clone, Debug)]
pub struct PlaylistLookup {
    pub id: String,
    pub name: String,
    pub tracks: Vec<TrackLookup>,
}

pub fn flatten_song_payload(song_payload: &Value) -> Value {
    let mut song_data = song_payload.clone();
    if !song_data.is_object() {
        song_data = json!({});
    }
    let nested_data = nested_map(&song_data, "data").cloned();
    if let Some(nested) = nested_data {
        merge_obj(&mut song_data, &nested);
    }
    let nested_song = nested_map(&song_data, "song").cloned();
    if let Some(nested) = nested_song {
        merge_obj(&mut song_data, &nested);
    }
    song_data
}

fn merge_obj(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            d.insert(k.clone(), v.clone());
        }
    }
}

pub fn parse_current_song(song_payload: &Value) -> CurrentSong {
    let song_data = flatten_song_payload(song_payload);
    let mut title = as_string(song_data.get("title").unwrap_or(&Value::Null));
    let mut artist = as_string(song_data.get("artist").unwrap_or(&Value::Null));
    if title.is_empty() {
        title = as_string(song_data.get("name").unwrap_or(&Value::Null));
    }
    if artist.is_empty() {
        artist = as_string(song_data.get("artistName").unwrap_or(&Value::Null));
    }
    let mut is_paused = as_bool(song_data.get("isPaused").unwrap_or(&Value::Null)).unwrap_or(false);
    if !is_paused {
        if let Some(playing) = as_bool(song_data.get("isPlaying").unwrap_or(&Value::Null)) {
            is_paused = !playing;
        }
    }
    if !is_paused {
        if as_string(song_data.get("state").unwrap_or(&Value::Null)).eq_ignore_ascii_case("paused")
        {
            is_paused = true;
        }
    }
    let mut state = if is_paused { "paused" } else { "playing" };
    if title.is_empty() && artist.is_empty() {
        state = "";
    }
    CurrentSong {
        title,
        artist,
        state: state.into(),
    }
}

pub fn build_song_info_response(song_payload: &Value) -> Envelope {
    let song = parse_current_song(song_payload);
    if song.title.is_empty() && song.artist.is_empty() {
        return Envelope::fail_data(
            "Song unavailable. No current track information was returned by the player.",
            json!({"reason": "no_song"}),
        );
    }
    Envelope::ok_msg(
        format!(
            "Song: {} — {}. Playback state: {}.",
            song.title, song.artist, song.state
        ),
        json!({"title": song.title, "artist": song.artist, "state": song.state}),
    )
}

pub fn current_song_video_id(song_payload: &Value) -> String {
    let song_data = flatten_song_payload(song_payload);
    let mut id = as_string(song_data.get("videoId").unwrap_or(&Value::Null));
    if id.is_empty() {
        id = as_string(song_data.get("video_id").unwrap_or(&Value::Null));
    }
    id
}

pub fn search_songs(client: &Client, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
    let payload = client.post_json("/api/v1/search", json!({"query": query}))?;
    Ok(list_search_results(&payload, limit))
}

pub fn list_search_results(payload: &Value, limit: usize) -> Vec<SearchHit> {
    if limit == 0 {
        return Vec::new();
    }
    let entries = collect_search_results(payload, limit * 2);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let id = entry.video_id.trim().to_string();
        if id.is_empty() {
            continue;
        }
        if !seen.insert(id.to_lowercase()) {
            continue;
        }
        out.push(SearchHit {
            video_id: id,
            title: entry.title,
            artist: entry.artist,
            duration: entry.duration_seconds,
        });
        if out.len() >= limit {
            break;
        }
    }
    out
}

pub fn lookup_song_by_query(client: &Client, query: &str) -> Result<(String, SongLookup), String> {
    let payload = client.post_json("/api/v1/search", json!({"query": query}))?;
    let entry = find_best_search_result(&payload, query)
        .ok_or_else(|| format!("no matching search results for \"{query}\""))?;
    if entry.video_id.is_empty() {
        return Err(format!("no matching search results for \"{query}\""));
    }
    Ok((
        entry.video_id,
        SongLookup {
            title: entry.title,
            artist: entry.artist,
            duration: entry.duration_seconds,
        },
    ))
}

pub fn lookup_song_by_video_id(client: &Client, video_id: &str) -> Result<SongLookup, String> {
    let payload = client.post_json(
        "/api/v1/search",
        json!({"query": format!("https://www.youtube.com/watch?v={video_id}")}),
    )?;
    let entry = find_queue_entry_for_video_id(&payload, video_id)
        .ok_or_else(|| format!("song metadata not found for video ID \"{video_id}\""))?;
    if entry.title.is_empty() && entry.artist.is_empty() {
        return Err(format!(
            "song metadata not found for video ID \"{video_id}\""
        ));
    }
    Ok(SongLookup {
        title: entry.title,
        artist: entry.artist,
        duration: entry.duration_seconds,
    })
}

fn find_queue_entry_for_video_id(payload: &Value, video_id: &str) -> Option<QueueEntrySummary> {
    let mut best = QueueEntrySummary::default();
    let mut best_score = -1;
    walk_values(payload, &mut |value| {
        if let Value::Object(_) = value {
            if !map_references_video_id(value, video_id) {
                return;
            }
            let mut entry = enrich_search_entry(value, summarize_queue_entry(value));
            if entry.video_id.is_empty() {
                entry.video_id = video_id.to_string();
            }
            let score = song_lookup_score(&entry);
            if score > best_score {
                best = entry;
                best_score = score;
            }
        }
    });
    if best_score >= 0 {
        Some(best)
    } else {
        None
    }
}

fn find_best_search_result(payload: &Value, query: &str) -> Option<QueueEntrySummary> {
    let entries = collect_search_results(payload, 40);
    if entries.is_empty() {
        return None;
    }
    let mut matched = Vec::new();
    let mut has_duration = false;
    for (i, entry) in entries.iter().enumerate() {
        if let Some(score) = query_relevance_score(query, &entry.title, &entry.artist) {
            matched.push((i, score));
            if entry.duration_seconds > 0 {
                has_duration = true;
            }
        }
    }
    if matched.is_empty() {
        return None;
    }
    let mut best_idx = 0usize;
    let mut best_score = -1;
    for (i, score) in matched {
        if has_duration && entries[i].duration_seconds <= 0 {
            continue;
        }
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    Some(entries[best_idx].clone())
}

fn collect_search_results(payload: &Value, limit: usize) -> Vec<QueueEntrySummary> {
    if limit == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    collect_search_walk(payload, limit, &mut out);
    out
}

fn collect_search_walk(value: &Value, limit: usize, out: &mut Vec<QueueEntrySummary>) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(shelf) = map.get("musicCardShelfRenderer") {
                let wrap = json!({"musicCardShelfRenderer": shelf});
                let entry = enrich_search_entry(&wrap, summarize_queue_entry(&wrap));
                if !entry.video_id.is_empty() {
                    out.push(entry);
                    if out.len() >= limit {
                        return true;
                    }
                }
            }
            if let Some(item) = map.get("musicResponsiveListItemRenderer") {
                let wrap = json!({"musicResponsiveListItemRenderer": item});
                let entry = enrich_search_entry(&wrap, summarize_queue_entry(&wrap));
                if !entry.video_id.is_empty() {
                    out.push(entry);
                    if out.len() >= limit {
                        return true;
                    }
                }
            }
            for nested in map.values() {
                if collect_search_walk(nested, limit, out) {
                    return true;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                if collect_search_walk(item, limit, out) {
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

fn query_relevance_score(query: &str, title: &str, artist: &str) -> Option<i32> {
    let query_norm = normalize_match_text(query);
    let haystack = normalize_match_text(&format!("{title} {artist}"));
    if query_norm.is_empty() || haystack.is_empty() {
        return None;
    }
    if query_norm == haystack || haystack.contains(&query_norm) {
        return Some(1000 + query_norm.len() as i32);
    }
    let tokens = significant_query_tokens(&query_norm);
    if tokens.is_empty() {
        return None;
    }
    let mut matched_count = 0;
    let mut score = 0;
    let title_norm = normalize_match_text(title);
    for token in &tokens {
        if haystack.contains(token) {
            matched_count += 1;
            if title_norm.contains(token) {
                score += 15;
            } else {
                score += 5;
            }
        }
    }
    let mut min_required = (tokens.len() + 1) / 2;
    if tokens.len() <= 2 {
        min_required = tokens.len();
    }
    if matched_count < min_required {
        return None;
    }
    score += matched_count as i32 * 100;
    score += (matched_count as i32 * 100) / tokens.len() as i32;
    Some(score)
}

fn significant_query_tokens(normalized_query: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for part in normalized_query.split_whitespace() {
        if is_match_stopword(part) {
            continue;
        }
        if part.chars().count() < 2 {
            continue;
        }
        if seen.insert(part.to_string()) {
            out.push(part.to_string());
        }
    }
    out
}

fn normalize_match_text(value: &str) -> String {
    let value = value.trim().to_lowercase();
    if value.is_empty() {
        return String::new();
    }
    let mut b = String::new();
    let mut last_space = true;
    for r in value.chars() {
        if r.is_ascii_alphanumeric() {
            b.push(r);
            last_space = false;
        } else if !last_space {
            b.push(' ');
            last_space = true;
        }
    }
    b.trim().to_string()
}

fn is_match_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "the"
            | "and"
            | "or"
            | "of"
            | "to"
            | "for"
            | "in"
            | "on"
            | "at"
            | "by"
            | "ft"
            | "feat"
            | "featuring"
            | "with"
            | "vs"
            | "versus"
            | "official"
            | "video"
            | "audio"
            | "lyrics"
            | "lyric"
            | "hd"
            | "hq"
            | "mv"
    )
}

fn enrich_search_entry(value: &Value, mut entry: QueueEntrySummary) -> QueueEntrySummary {
    if let Some(shelf) = nested_map(value, "musicCardShelfRenderer") {
        if entry.title.is_empty() {
            entry.title = extract_text_from_runs(shelf.get("title").unwrap_or(&Value::Null));
        }
        let subtitle = extract_text_from_runs(shelf.get("subtitle").unwrap_or(&Value::Null));
        if entry.artist.is_empty() {
            entry.artist = extract_artist_from_search_subtitle(&subtitle);
        }
        if entry.duration_seconds == 0 {
            entry.duration_seconds =
                parse_duration_seconds(&extract_duration_token_from_subtitle(&subtitle));
        }
        if entry.video_id.is_empty() {
            entry.video_id = extract_video_id_from_value(shelf);
        }
    }
    if let Some(renderer) = extract_queue_renderer_candidate(value) {
        let (flex_title, flex_artist) = extract_flex_column_metadata(renderer);
        if entry.title.is_empty() {
            entry.title = flex_title;
        }
        if entry.artist.is_empty() {
            entry.artist = flex_artist;
        }
        if entry.duration_seconds == 0 {
            entry.duration_seconds = extract_fixed_column_duration(renderer);
        }
        if entry.video_id.is_empty() {
            entry.video_id = extract_video_id_from_value(renderer);
        }
    }
    if entry.duration_seconds == 0 {
        if let Some(d) = as_f64(value.get("songDuration").unwrap_or(&Value::Null)) {
            entry.duration_seconds = d as i32;
        }
    }
    if entry.duration_seconds == 0 {
        if let Some(d) = as_f64(value.get("lengthSeconds").unwrap_or(&Value::Null)) {
            entry.duration_seconds = d as i32;
        }
    }
    entry
}

fn extract_flex_column_metadata(renderer: &Value) -> (String, String) {
    let Some(columns) = renderer.get("flexColumns").and_then(|v| v.as_array()) else {
        return (String::new(), String::new());
    };
    let title = columns
        .first()
        .map(extract_flex_column_text)
        .unwrap_or_default();
    let artist = columns
        .get(1)
        .map(|c| normalize_byline_artist(&Value::String(extract_flex_column_text(c))))
        .unwrap_or_default();
    (title, artist)
}

fn extract_flex_column_text(column: &Value) -> String {
    if let Some(flex) = nested_map(column, "musicResponsiveListItemFlexColumnRenderer") {
        return extract_text_from_runs(flex.get("text").unwrap_or(&Value::Null));
    }
    extract_text_from_runs(column.get("text").unwrap_or(&Value::Null))
}

fn extract_fixed_column_duration(renderer: &Value) -> i32 {
    let Some(columns) = renderer.get("fixedColumns").and_then(|v| v.as_array()) else {
        return 0;
    };
    for column in columns {
        if let Some(flex) = nested_map(column, "musicResponsiveListItemFixedColumnRenderer") {
            let d = parse_duration_seconds(&extract_text_from_runs(
                flex.get("text").unwrap_or(&Value::Null),
            ));
            if d > 0 {
                return d;
            }
        }
    }
    0
}

fn extract_artist_from_search_subtitle(subtitle: &str) -> String {
    let subtitle = subtitle.trim();
    if subtitle.is_empty() {
        return String::new();
    }
    for part in subtitle.split('•') {
        let candidate = part.trim();
        if candidate.is_empty() {
            continue;
        }
        let lower = candidate.to_lowercase();
        if matches!(
            lower.as_str(),
            "video" | "song" | "music" | "single" | "ep" | "album"
        ) {
            continue;
        }
        if lower.contains(" view") {
            continue;
        }
        if parse_duration_seconds(candidate) > 0 {
            continue;
        }
        return candidate.to_string();
    }
    normalize_byline_artist(&Value::String(subtitle.to_string()))
}

fn extract_duration_token_from_subtitle(subtitle: &str) -> String {
    let parts: Vec<&str> = subtitle.split('•').collect();
    for part in parts.iter().rev() {
        let candidate = part.trim();
        if parse_duration_seconds(candidate) > 0 {
            return candidate.to_string();
        }
    }
    String::new()
}

fn extract_video_id_from_value(value: &Value) -> String {
    let id = as_string(value.get("videoId").unwrap_or(&Value::Null));
    if !id.is_empty() {
        return id;
    }
    if let Some(nested) = nested_map(value, "onTap") {
        let id = extract_video_id_from_watch_endpoint(
            nested.get("watchEndpoint").unwrap_or(&Value::Null),
        );
        if !id.is_empty() {
            return id;
        }
    }
    if let Some(nested) = nested_map(value, "navigationEndpoint") {
        let id = extract_video_id_from_watch_endpoint(
            nested.get("watchEndpoint").unwrap_or(&Value::Null),
        );
        if !id.is_empty() {
            return id;
        }
    }
    let id =
        extract_video_id_from_watch_endpoint(value.get("watchEndpoint").unwrap_or(&Value::Null));
    if !id.is_empty() {
        return id;
    }
    extract_video_id_deep(value, 0)
}

fn extract_video_id_deep(value: &Value, depth: i32) -> String {
    if depth > 10 {
        return String::new();
    }
    match value {
        Value::Object(map) => {
            let id = extract_video_id_from_watch_endpoint(
                map.get("watchEndpoint").unwrap_or(&Value::Null),
            );
            if !id.is_empty() {
                return id;
            }
            if let Some(nested) = map.get("playNavigationEndpoint") {
                let id = extract_video_id_deep(nested, depth + 1);
                if !id.is_empty() {
                    return id;
                }
            }
            for nested in map.values() {
                let id = extract_video_id_deep(nested, depth + 1);
                if !id.is_empty() {
                    return id;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                let id = extract_video_id_deep(item, depth + 1);
                if !id.is_empty() {
                    return id;
                }
            }
        }
        _ => {}
    }
    String::new()
}

fn extract_video_id_from_watch_endpoint(value: &Value) -> String {
    as_string(value.get("videoId").unwrap_or(&Value::Null))
}

fn map_references_video_id(value: &Value, video_id: &str) -> bool {
    if as_string(value.get("videoId").unwrap_or(&Value::Null)).eq_ignore_ascii_case(video_id) {
        return true;
    }
    if as_string(value.get("id").unwrap_or(&Value::Null)).eq_ignore_ascii_case(video_id) {
        return true;
    }
    if extract_video_id_from_value(value).eq_ignore_ascii_case(video_id) {
        return true;
    }
    if let Some(shelf) = nested_map(value, "musicCardShelfRenderer") {
        if extract_video_id_from_value(shelf).eq_ignore_ascii_case(video_id) {
            return true;
        }
    }
    if let Some(item) = nested_map(value, "musicResponsiveListItemRenderer") {
        if as_string(item.get("videoId").unwrap_or(&Value::Null)).eq_ignore_ascii_case(video_id)
            || extract_video_id_from_value(item).eq_ignore_ascii_case(video_id)
        {
            return true;
        }
    }
    if let Some(nested) = nested_map(value, "watchEndpoint") {
        if as_string(nested.get("videoId").unwrap_or(&Value::Null)).eq_ignore_ascii_case(video_id) {
            return true;
        }
    }
    if is_queue_entry_like(value) {
        return summarize_queue_entry(value)
            .video_id
            .eq_ignore_ascii_case(video_id);
    }
    false
}

fn song_lookup_score(entry: &QueueEntrySummary) -> i32 {
    let mut score = 0;
    if !entry.title.is_empty() {
        score += 1;
    }
    if !entry.artist.is_empty() {
        score += 1;
    }
    if entry.duration_seconds > 0 {
        score += 1;
    }
    score
}

pub fn build_queue_info_response(song_payload: &Value, queue_payload: &Value) -> Envelope {
    let song_data = flatten_song_payload(song_payload);
    let mut queue_data = if queue_payload.is_object() {
        queue_payload.clone()
    } else {
        json!({})
    };
    if let Some(nested) = nested_map(&queue_data, "data") {
        queue_data = nested.clone();
    }
    let mut is_paused = as_bool(song_data.get("isPaused").unwrap_or(&Value::Null)).unwrap_or(false);
    if !is_paused {
        if let Some(playing) = as_bool(song_data.get("isPlaying").unwrap_or(&Value::Null)) {
            is_paused = !playing;
        }
    }
    if is_paused {
        return Envelope::fail_data(
            "Queue unavailable while the player is paused. Playback must be active before a queue summary can be generated.",
            json!({"reason": "player_paused"}),
        );
    }
    let mut entries = collect_queue_entries_for_command(&queue_data);
    entries = reorder_queue_entries_from_current_song(entries, &song_data);
    if entries.is_empty() {
        return Envelope::fail_data(
            "Queue unavailable. No queue items were returned by the player.",
            json!({"reason": "empty_queue"}),
        );
    }
    let summary = summarize_queue_entries(&entries);
    let items_json: Vec<Value> = summary
        .items
        .iter()
        .map(|e| {
            json!({
                "Title": e.title,
                "Artist": e.artist,
                "VideoID": e.video_id,
                "DurationSeconds": e.duration_seconds,
                "DurationLabel": e.duration_label,
                "DisplayText": e.display_text,
            })
        })
        .collect();
    // Go exported struct fields as Title/Artist because encoding/json uses those names
    // unless tagged. queueEntrySummary has no json tags so Go serializes as Title, Artist...
    Envelope::ok_msg(
        format!(
            "Queue: {}. {} songs, {} total.",
            summary.display,
            summary.items.len(),
            format_minutes(summary.total_duration_seconds)
        ),
        json!({
            "songs": items_json,
            "totalSeconds": summary.total_duration_seconds,
            "display": summary.display,
            "count": summary.items.len()
        }),
    )
}

pub fn build_song_request_response(
    video_id: &str,
    queue_payload: &Value,
    lookup: &SongLookup,
) -> Envelope {
    let mut data = json!({"videoId": video_id});
    let mut title = lookup.title.clone();
    let mut artist = lookup.artist.clone();
    if let Some(obj) = queue_payload.as_object() {
        if !obj.is_empty() {
            let (mut payload_title, mut payload_artist) = extract_song_metadata(queue_payload);
            if let Some(nested) = nested_map(queue_payload, "data") {
                let (t, a) = extract_song_metadata(nested);
                if payload_title.is_empty() {
                    payload_title = t;
                }
                if payload_artist.is_empty() {
                    payload_artist = a;
                }
            }
            if let Some(nested) = nested_map(queue_payload, "song") {
                let (t, a) = extract_song_metadata(nested);
                if payload_title.is_empty() {
                    payload_title = t;
                }
                if payload_artist.is_empty() {
                    payload_artist = a;
                }
            }
            if !payload_title.is_empty() {
                title = payload_title;
            }
            if !payload_artist.is_empty() {
                artist = payload_artist;
            }
        }
    }
    if !title.is_empty() {
        data["title"] = json!(title);
    }
    if !artist.is_empty() {
        data["artist"] = json!(artist);
    }
    let message = if !title.is_empty() && !artist.is_empty() {
        format!("Added to queue: {title} — {artist}.")
    } else if !title.is_empty() {
        format!("Added to queue: {title}.")
    } else {
        format!("Added to queue: {video_id}.")
    };
    Envelope::ok_msg(message, data)
}

fn extract_song_metadata(payload: &Value) -> (String, String) {
    let mut title = as_string(payload.get("title").unwrap_or(&Value::Null));
    if title.is_empty() {
        title = as_string(payload.get("name").unwrap_or(&Value::Null));
    }
    let mut artist = as_string(payload.get("artist").unwrap_or(&Value::Null));
    if artist.is_empty() {
        artist = as_string(payload.get("artistName").unwrap_or(&Value::Null));
    }
    (title, artist)
}

pub fn queue_contains_video_id(payload: &Value, video_id: &str) -> bool {
    if let Some(items) = ordered_queue_items_array(payload) {
        for item in items {
            let entry = summarize_queue_entry(item);
            if entry.video_id.eq_ignore_ascii_case(video_id)
                || extract_video_id_from_value(item).eq_ignore_ascii_case(video_id)
            {
                return true;
            }
        }
    }
    false
}

pub fn normalize_admin_queue(
    song_payload: &Value,
    queue_payload: &Value,
) -> (Vec<AdminQueueItem>, String) {
    let Some(ordered) = ordered_queue_items_array(queue_payload) else {
        return (Vec::new(), "error".into());
    };
    if ordered.is_empty() {
        return (Vec::new(), "empty".into());
    }
    let current = resolve_current_queue_index(queue_payload, &current_song_video_id(song_payload));
    let mut items = Vec::new();
    for (index, raw) in ordered.iter().enumerate() {
        let entry = summarize_queue_entry(raw);
        if entry.title.is_empty() && entry.artist.is_empty() && entry.video_id.is_empty() {
            continue;
        }
        let kind = if current >= 0 {
            if index as i32 == current {
                "current"
            } else if (index as i32) < current {
                "previous"
            } else {
                "next"
            }
        } else {
            "next"
        };
        items.push(AdminQueueItem {
            title: entry.title,
            artist: entry.artist,
            video_id: entry.video_id,
            queue_index: index as i32,
            kind: kind.into(),
        });
    }
    if items.is_empty() {
        return (items, "empty".into());
    }
    (items, "ok".into())
}

fn collect_queue_entries_for_command(payload: &Value) -> Vec<QueueEntrySummary> {
    if payload.is_null() {
        return Vec::new();
    }
    if let Some(ordered) = ordered_queue_items_array(payload) {
        let mut collected = Vec::new();
        for item in ordered {
            if !is_queue_entry_like(item) {
                continue;
            }
            let entry = summarize_queue_entry(item);
            if entry.title.is_empty() && entry.artist.is_empty() && entry.video_id.is_empty() {
                continue;
            }
            collected.push(entry);
        }
        return collected;
    }
    // Innertube browse: root `contents` is an object, not a queue array.
    collect_queue_entries_walk(payload)
}

fn collect_queue_entries_walk(payload: &Value) -> Vec<QueueEntrySummary> {
    let mut collected = Vec::new();
    let mut seen = std::collections::HashSet::new();
    walk_queue_entry_nodes(payload, &mut collected, &mut seen);
    collected
}

fn walk_queue_entry_nodes(
    value: &Value,
    collected: &mut Vec<QueueEntrySummary>,
    seen: &mut std::collections::HashSet<String>,
) {
    match value {
        Value::Array(arr) => {
            for item in arr {
                walk_queue_entry_nodes(item, collected, seen);
            }
        }
        Value::Object(obj) => {
            if is_queue_entry_like(value) {
                let entry = summarize_queue_entry(value);
                if !(entry.title.is_empty() && entry.artist.is_empty() && entry.video_id.is_empty())
                {
                    let key = entry.video_id.trim().to_lowercase();
                    if !key.is_empty() && seen.insert(key) {
                        collected.push(entry);
                    }
                }
            }
            for key in ["items", "entries", "contents", "content", "queue"] {
                if let Some(nested) = obj.get(key) {
                    walk_queue_entry_nodes(nested, collected, seen);
                }
            }
            for (key, nested) in obj {
                if matches!(
                    key.as_str(),
                    "items" | "entries" | "contents" | "content" | "queue"
                ) {
                    continue;
                }
                walk_queue_entry_nodes(nested, collected, seen);
            }
        }
        _ => {}
    }
}

pub fn ordered_queue_items_array(payload: &Value) -> Option<&Vec<Value>> {
    match payload {
        Value::Array(arr) => Some(arr),
        Value::Object(map) => {
            if let Some(nested) = map.get("data") {
                if let Some(items) = ordered_queue_items_array(nested) {
                    return Some(items);
                }
            }
            for key in ["items", "entries", "contents", "content", "queue"] {
                if let Some(Value::Array(items)) = map.get(key) {
                    return Some(items);
                }
            }
            None
        }
        _ => None,
    }
}

struct QueueSummary {
    items: Vec<QueueEntrySummary>,
    display: String,
    total_duration_seconds: i32,
}

fn reorder_queue_entries_from_current_song(
    entries: Vec<QueueEntrySummary>,
    song_data: &Value,
) -> Vec<QueueEntrySummary> {
    if entries.len() <= 1 {
        return entries;
    }
    let mut current_title = as_string(song_data.get("title").unwrap_or(&Value::Null));
    if current_title.is_empty() {
        current_title = as_string(song_data.get("name").unwrap_or(&Value::Null));
    }
    let mut current_artist = as_string(song_data.get("artist").unwrap_or(&Value::Null));
    if current_artist.is_empty() {
        current_artist = as_string(song_data.get("artistName").unwrap_or(&Value::Null));
    }
    let mut current_video_id = as_string(song_data.get("videoId").unwrap_or(&Value::Null));
    if current_video_id.is_empty() {
        current_video_id = as_string(song_data.get("video_id").unwrap_or(&Value::Null));
    }
    if current_title.is_empty() && current_artist.is_empty() && current_video_id.is_empty() {
        return entries;
    }
    let mut current_index = -1i32;
    if !current_video_id.is_empty() {
        for (idx, entry) in entries.iter().enumerate().rev() {
            if entry.video_id.eq_ignore_ascii_case(&current_video_id) {
                current_index = idx as i32;
                break;
            }
        }
    }
    if current_index < 0 && !current_title.is_empty() && !current_artist.is_empty() {
        for (idx, entry) in entries.iter().enumerate() {
            if entry.title.eq_ignore_ascii_case(&current_title)
                && entry.artist.eq_ignore_ascii_case(&current_artist)
            {
                current_index = idx as i32;
                break;
            }
        }
    }
    if current_index < 0 && !current_title.is_empty() && current_artist.is_empty() {
        for (idx, entry) in entries.iter().enumerate() {
            if entry.title.eq_ignore_ascii_case(&current_title) {
                current_index = idx as i32;
                break;
            }
        }
    }
    if current_index <= 0 {
        return entries;
    }
    entries[current_index as usize..].to_vec()
}

fn summarize_queue_entries(entries: &[QueueEntrySummary]) -> QueueSummary {
    let items = entries.to_vec();
    let visible = if items.len() > 3 {
        &items[..3]
    } else {
        &items[..]
    };
    let mut display_parts = Vec::new();
    for entry in visible {
        if !entry.artist.is_empty() && !entry.title.is_empty() {
            display_parts.push(format!("{} - {}", entry.artist, entry.title));
        } else {
            display_parts.push(entry.display_text.clone());
        }
    }
    let mut display = display_parts.join(", ");
    if items.len() > visible.len() {
        display = format!("{}, +{} more", display, items.len() - visible.len());
    }
    let total: i32 = items.iter().map(|e| e.duration_seconds).sum();
    QueueSummary {
        items,
        display,
        total_duration_seconds: total,
    }
}

fn summarize_queue_entry(value: &Value) -> QueueEntrySummary {
    let mut title;
    let mut artist;
    let mut video_id;
    let mut duration_seconds;
    if let Some(renderer) = extract_queue_renderer_candidate(value) {
        title = extract_text_from_runs(renderer.get("title").unwrap_or(&Value::Null));
        artist = normalize_byline_artist(renderer.get("longBylineText").unwrap_or(&Value::Null));
        video_id = as_string(renderer.get("videoId").unwrap_or(&Value::Null));
        duration_seconds = parse_duration_seconds(&extract_text_from_runs(
            renderer.get("lengthText").unwrap_or(&Value::Null),
        ));
    } else {
        title = extract_text_from_runs(value.get("title").unwrap_or(&Value::Null));
        artist = normalize_byline_artist(value.get("longBylineText").unwrap_or(&Value::Null));
        video_id = as_string(value.get("videoId").unwrap_or(&Value::Null));
        duration_seconds = parse_duration_seconds(&extract_text_from_runs(
            value.get("lengthText").unwrap_or(&Value::Null),
        ));
    }
    if title.is_empty() {
        title = as_string(value.get("title").unwrap_or(&Value::Null));
    }
    if artist.is_empty() {
        artist = as_string(value.get("artist").unwrap_or(&Value::Null));
    }
    if video_id.is_empty() {
        video_id = as_string(value.get("id").unwrap_or(&Value::Null));
    }
    if duration_seconds == 0 {
        duration_seconds =
            parse_duration_seconds(&as_string(value.get("duration").unwrap_or(&Value::Null)));
    }
    let mut display_text = title.clone();
    if !artist.is_empty() {
        display_text = format!("{artist} - {title}");
    }
    let mut entry = QueueEntrySummary {
        title,
        artist,
        video_id,
        duration_seconds,
        duration_label: format_minutes(duration_seconds),
        display_text,
    };
    entry = enrich_search_entry(value, entry);
    if entry.display_text.is_empty() {
        entry.display_text = if !entry.artist.is_empty() && !entry.title.is_empty() {
            format!("{} - {}", entry.artist, entry.title)
        } else {
            entry.title.clone()
        };
    }
    if entry.duration_label.is_empty() && entry.duration_seconds > 0 {
        entry.duration_label = format_minutes(entry.duration_seconds);
    }
    entry
}

fn extract_queue_renderer_candidate(value: &Value) -> Option<&Value> {
    let obj = value.as_object()?;
    for key in [
        "playlistPanelVideoRenderer",
        "videoRenderer",
        "musicResponsiveListItemRenderer",
        "playlistPanelRenderer",
    ] {
        if let Some(v) = obj.get(key) {
            if v.is_object() {
                return Some(v);
            }
        }
    }
    for key in ["primaryRenderer", "renderer"] {
        if let Some(nested) = obj.get(key) {
            if let Some(r) = extract_queue_renderer_candidate(nested) {
                return Some(r);
            }
        }
    }
    if let Some(nested) = obj.get("playlistPanelVideoWrapperRenderer") {
        if let Some(r) = extract_queue_renderer_candidate(nested) {
            return Some(r);
        }
    }
    for candidate in obj.values() {
        if candidate.is_object() {
            if let Some(r) = extract_queue_renderer_candidate(candidate) {
                return Some(r);
            }
        }
    }
    None
}

fn is_queue_entry_like(value: &Value) -> bool {
    if extract_queue_renderer_candidate(value).is_some() {
        return true;
    }
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.contains_key("playlistPanelVideoRenderer")
        || obj.contains_key("videoRenderer")
        || obj.contains_key("musicResponsiveListItemRenderer")
        || obj.contains_key("title")
        || obj.contains_key("longBylineText")
        || obj.contains_key("videoId")
        || obj.contains_key("id")
}

fn extract_text_from_runs(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.trim().to_string(),
        Value::Array(arr) => arr
            .iter()
            .map(extract_text_from_runs)
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(map) => {
            if let Some(Value::Array(runs)) = map.get("runs") {
                return runs
                    .iter()
                    .map(extract_text_from_runs)
                    .collect::<Vec<_>>()
                    .join("");
            }
            if let Some(Value::String(t)) = map.get("text") {
                return t.trim().to_string();
            }
            if let Some(Value::String(t)) = map.get("simpleText") {
                return t.trim().to_string();
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn normalize_byline_artist(value: &Value) -> String {
    let text = extract_text_from_runs(value).trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    for sep in [" • ", "•", " - ", " – ", " / ", " | ", " — "] {
        if text.contains(sep) {
            for part in text.split(sep) {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    text
}

fn parse_duration_seconds(value: &str) -> i32 {
    let value = value.trim();
    if value.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() == 2 {
        if let (Ok(m), Ok(s)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            return m * 60 + s;
        }
    }
    0
}

fn format_minutes(total_seconds: i32) -> String {
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

pub fn queue_item_video_ids(payload: &Value) -> Vec<String> {
    let Some(items) = ordered_queue_items_array(payload) else {
        return Vec::new();
    };
    items
        .iter()
        .map(summarize_queue_entry)
        .filter(|e| !e.video_id.is_empty())
        .map(|e| e.video_id)
        .collect()
}

pub fn queue_indices_for_video_ids(
    payload: &Value,
    video_ids: &std::collections::HashSet<String>,
) -> Vec<i32> {
    let Some(items) = ordered_queue_items_array(payload) else {
        return Vec::new();
    };
    let mut indices = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mut video_id = summarize_queue_entry(item).video_id.trim().to_lowercase();
        if video_id.is_empty() {
            video_id = extract_video_id_from_value(item).trim().to_lowercase();
        }
        if video_id.is_empty() {
            continue;
        }
        if video_ids.contains(&video_id) {
            indices.push(index as i32);
        }
    }
    indices
}

pub fn current_queue_index(payload: &Value) -> i32 {
    let Some(items) = ordered_queue_items_array(payload) else {
        return -1;
    };
    for (index, item) in items.iter().enumerate() {
        if as_bool(item.get("selected").unwrap_or(&Value::Null)).unwrap_or(false) {
            return index as i32;
        }
        if let Some(renderer) = extract_queue_renderer_candidate(item) {
            if as_bool(renderer.get("selected").unwrap_or(&Value::Null)).unwrap_or(false) {
                return index as i32;
            }
        }
    }
    -1
}

pub fn resolve_current_queue_index(payload: &Value, current_video_id: &str) -> i32 {
    let index = current_queue_index(payload);
    if index >= 0 {
        return index;
    }
    queue_index_for_video_id(payload, current_video_id, true)
}

pub fn queue_index_for_video_id(payload: &Value, video_id: &str, prefer_last: bool) -> i32 {
    let video_id = video_id.trim().to_lowercase();
    if video_id.is_empty() {
        return -1;
    }
    let Some(items) = ordered_queue_items_array(payload) else {
        return -1;
    };
    if prefer_last {
        for index in (0..items.len()).rev() {
            if queue_item_video_id(&items[index]).eq_ignore_ascii_case(&video_id) {
                return index as i32;
            }
        }
        return -1;
    }
    for (index, item) in items.iter().enumerate() {
        if queue_item_video_id(item).eq_ignore_ascii_case(&video_id) {
            return index as i32;
        }
    }
    -1
}

fn queue_item_video_id(item: &Value) -> String {
    let entry = summarize_queue_entry(item);
    if !entry.video_id.trim().is_empty() {
        return entry.video_id;
    }
    extract_video_id_from_value(item).trim().to_string()
}

pub fn queue_video_ids_by_index(payload: &Value) -> std::collections::HashMap<i32, String> {
    let mut result = std::collections::HashMap::new();
    let Some(items) = ordered_queue_items_array(payload) else {
        return result;
    };
    for (index, item) in items.iter().enumerate() {
        let video_id = queue_item_video_id(item).trim().to_lowercase();
        if !video_id.is_empty() {
            result.insert(index as i32, video_id);
        }
    }
    result
}

pub fn parse_playlist_payload(playlist_id: &str, payload: &Value) -> Option<PlaylistLookup> {
    let mut name = extract_playlist_name(payload);
    if name.is_empty() {
        name = playlist_id.to_string();
    }
    let entries = collect_queue_entries_for_command(payload);
    let mut tracks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let video_id = entry.video_id.trim().to_string();
        if video_id.is_empty() {
            continue;
        }
        if !seen.insert(video_id.to_lowercase()) {
            continue;
        }
        tracks.push(TrackLookup {
            video_id,
            title: entry.title,
            artist: entry.artist,
            duration: entry.duration_seconds,
        });
    }
    if tracks.is_empty() {
        return None;
    }
    Some(PlaylistLookup {
        id: playlist_id.to_string(),
        name,
        tracks,
    })
}

pub fn extract_playlist_name(payload: &Value) -> String {
    let mut found = String::new();
    walk_values(payload, &mut |value| {
        if !found.is_empty() {
            return;
        }
        let Some(obj) = value.as_object() else {
            return;
        };
        if let Some(header) = obj.get("header") {
            for key in [
                "musicResponsiveHeaderRenderer",
                "musicDetailHeaderRenderer",
                "musicEditablePlaylistDetailHeaderRenderer",
                "musicImmersiveHeaderRenderer",
            ] {
                if let Some(renderer) = nested_map(header, key) {
                    let title =
                        extract_text_from_runs(renderer.get("title").unwrap_or(&Value::Null));
                    if !title.is_empty() {
                        found = title;
                        return;
                    }
                }
            }
        }
        for key in ["musicResponsiveHeaderRenderer", "musicDetailHeaderRenderer"] {
            if let Some(header) = nested_map(value, key) {
                let title = extract_text_from_runs(header.get("title").unwrap_or(&Value::Null));
                if !title.is_empty() {
                    found = title;
                    return;
                }
            }
        }
        if let Some(shelf) = nested_map(value, "musicPlaylistShelfRenderer") {
            let title = extract_text_from_runs(shelf.get("title").unwrap_or(&Value::Null));
            if !title.is_empty() {
                found = title;
                return;
            }
        }
        if let Some(micro) = nested_map(value, "microformatDataRenderer") {
            let title = extract_text_from_runs(micro.get("title").unwrap_or(&Value::Null));
            if !title.is_empty() {
                found = title;
                return;
            }
        }
        let title = extract_text_from_runs(value.get("title").unwrap_or(&Value::Null));
        if !title.is_empty() && obj.contains_key("contents") {
            found = title;
        }
    });
    found
}

pub fn extract_album_browse_id(payload: &Value) -> Option<String> {
    let mut found = None;
    walk_values(payload, &mut |value| {
        if found.is_some() {
            return;
        }
        if let Some(id) = value.get("browseId").and_then(|v| v.as_str()) {
            let id = id.trim();
            if id.starts_with("MPREb_") {
                found = Some(id.to_string());
            }
        }
    });
    found
}

fn walk_values(value: &Value, f: &mut dyn FnMut(&Value)) {
    f(value);
    match value {
        Value::Object(map) => {
            for nested in map.values() {
                walk_values(nested, f);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                walk_values(item, f);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_item(id: &str, title: &str, artist: &str) -> Value {
        json!({
            "musicResponsiveListItemRenderer": {
                "videoId": id,
                "title": { "runs": [{ "text": title }] },
                "longBylineText": { "runs": [{ "text": artist }] }
            }
        })
    }

    #[test]
    fn parse_playlist_from_innertube_shelf() {
        let payload = json!({
            "header": {
                "musicDetailHeaderRenderer": {
                    "title": { "runs": [{ "text": "Mix" }] }
                }
            },
            "contents": {
                "twoColumnBrowseResultsRenderer": {
                    "secondaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "musicPlaylistShelfRenderer": {
                                    "contents": [
                                        list_item("vidA", "Track A", "Artist A"),
                                        list_item("vidB", "Track B", "Artist B")
                                    ]
                                }
                            }]
                        }
                    }
                }
            }
        });
        let lookup = parse_playlist_payload("PL1", &payload).expect("tracks");
        assert_eq!(lookup.name, "Mix");
        assert_eq!(
            lookup
                .tracks
                .iter()
                .map(|t| t.video_id.as_str())
                .collect::<Vec<_>>(),
            ["vidA", "vidB"]
        );
    }

    #[test]
    fn parse_playlist_from_queue_items_array() {
        let payload = json!({
            "items": [
                { "videoId": "q1", "title": "One", "artist": "A" },
                { "videoId": "q2", "title": "Two", "artist": "B" }
            ]
        });
        let lookup = parse_playlist_payload("PL1", &payload).expect("tracks");
        assert_eq!(
            lookup
                .tracks
                .iter()
                .map(|t| t.video_id.as_str())
                .collect::<Vec<_>>(),
            ["q1", "q2"]
        );
    }
}
