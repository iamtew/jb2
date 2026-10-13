use url::Url;

pub const MAX_SONG_REQUEST_INPUT_LEN: usize = 2048;

const TRAILING_URL_JUNK: &[char] = &['.', ',', '!', '?', ';', ':', ')', ']', '}', '>', '"', '\''];

const KNOWN_LINK_HOST_PREFIXES: &[&str] = &[
    "youtu.be/",
    "youtu.be?",
    "youtube.com/",
    "youtube.com?",
    "m.youtube.com/",
    "m.youtube.com?",
    "music.youtube.com/",
    "music.youtube.com?",
    "youtube-nocookie.com/",
    "youtube-nocookie.com?",
    "spotify.com/",
    "spotify.com?",
    "open.spotify.com/",
    "open.spotify.com?",
];

fn is_bare_video_id(s: &str) -> bool {
    s.len() == 11
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_spotify_track_id(s: &str) -> bool {
    s.len() == 22 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn classify_song_request_input(input: &str) -> (String, bool) {
    let input = normalize_song_request_input(input);
    if input.is_empty() {
        return (String::new(), false);
    }
    if is_bare_video_id(&input) {
        return (input, false);
    }
    let url_like = has_url_shape(&input);
    let mut candidate = input.clone();
    if url_like {
        candidate = strip_trailing_url_junk(&candidate);
    }
    if let Some(id) = extract_video_id_from_url(&candidate) {
        return (id, true);
    }
    if url_like {
        return (String::new(), true);
    }
    (String::new(), false)
}

pub fn canonical_spotify_track_url(input: &str) -> Option<String> {
    let input = strip_trailing_url_junk(&normalize_song_request_input(input));
    if input.is_empty() {
        return None;
    }
    let lower = input.to_lowercase();
    if let Some(_rest) = lower.strip_prefix("spotify:track:") {
        let orig = &input["spotify:track:".len()..];
        let id = orig.split(['?', '#']).next().unwrap_or(orig);
        if !is_spotify_track_id(id) {
            return None;
        }
        return Some(format!("https://open.spotify.com/track/{id}"));
    }
    let candidate = if input.contains("://") {
        input.clone()
    } else {
        format!("https://{input}")
    };
    let parsed = Url::parse(&candidate).ok()?;
    let mut host = parsed.host_str().unwrap_or("").to_lowercase();
    if let Some(h) = host.strip_prefix("www.") {
        host = h.to_string();
    }
    if host != "open.spotify.com" && host != "spotify.com" {
        return None;
    }
    let id = spotify_track_id_from_path(parsed.path())?;
    Some(format!("https://open.spotify.com/track/{id}"))
}

fn spotify_track_id_from_path(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    for i in 0..segments.len().saturating_sub(1) {
        if segments[i].eq_ignore_ascii_case("track") {
            let id = segments[i + 1];
            if is_spotify_track_id(id) {
                return Some(id.to_string());
            }
            return None;
        }
    }
    None
}

fn normalize_song_request_input(input: &str) -> String {
    let input = input.trim();
    if let Some(stripped) = input.strip_prefix('<') {
        if let Some(end) = stripped.find('>') {
            let inner = stripped[..end].trim();
            let rest = &stripped[end + 1..];
            if rest.is_empty() || strip_trailing_url_junk(rest).is_empty() {
                return inner.to_string();
            }
        }
    }
    input.to_string()
}

fn has_url_shape(input: &str) -> bool {
    let lower = input.to_lowercase();
    if lower.contains("://") || lower.starts_with("www.") {
        return true;
    }
    for prefix in KNOWN_LINK_HOST_PREFIXES {
        if lower.starts_with(prefix) {
            return true;
        }
        let host_only = prefix.trim_end_matches(['/', '?']);
        if lower == host_only {
            return true;
        }
    }
    false
}

fn strip_trailing_url_junk(input: &str) -> String {
    let mut s = input.to_string();
    while let Some(c) = s.chars().last() {
        if TRAILING_URL_JUNK.contains(&c) {
            s.pop();
        } else {
            break;
        }
    }
    s
}

fn extract_video_id_from_url(input: &str) -> Option<String> {
    let candidate = if input.contains("://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let parsed = Url::parse(&candidate).ok()?;
    let mut host = parsed.host_str().unwrap_or("").to_lowercase();
    if let Some(h) = host.strip_prefix("www.") {
        host = h.to_string();
    }
    if let Some(h) = host.strip_prefix("m.") {
        host = h.to_string();
    }
    match host.as_str() {
        "youtu.be" => {
            let id = first_path_segment(parsed.path());
            if is_bare_video_id(&id) {
                return Some(id);
            }
        }
        "youtube.com" | "music.youtube.com" | "youtube-nocookie.com" => {
            if let Some(id) = parsed
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.into_owned())
            {
                if is_bare_video_id(&id) {
                    return Some(id);
                }
            }
            let segments: Vec<&str> = parsed.path().trim_matches('/').split('/').collect();
            if segments.len() >= 2 {
                match segments[0] {
                    "embed" | "v" | "shorts" | "live" if is_bare_video_id(segments[1]) => {
                        return Some(segments[1].to_string());
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    None
}

fn first_path_segment(path: &str) -> String {
    let path = path.trim_matches('/');
    path.split('/').next().unwrap_or("").to_string()
}

pub fn extract_playlist_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty input".into());
    }
    if is_bare_playlist_id(input) {
        return Ok(input.to_string());
    }
    let candidate = if input.contains("://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let parsed = Url::parse(&candidate).map_err(|_| "invalid playlist URL or ID".to_string())?;
    let mut host = parsed.host_str().unwrap_or("").to_lowercase();
    if let Some(h) = host.strip_prefix("www.") {
        host = h.to_string();
    }
    if let Some(h) = host.strip_prefix("m.") {
        host = h.to_string();
    }
    match host.as_str() {
        "youtube.com" | "music.youtube.com" | "youtube-nocookie.com" => {
            if let Some(id) = parsed
                .query_pairs()
                .find(|(k, _)| k == "list")
                .map(|(_, v)| v.into_owned())
            {
                if is_bare_playlist_id(&id) {
                    return Ok(id);
                }
            }
        }
        _ => {}
    }
    Err("invalid playlist URL or ID".into())
}

fn is_bare_playlist_id(input: &str) -> bool {
    let ok_prefix = input.starts_with("PL")
        || input.starts_with("OLAK5uy_")
        || input.starts_with("RD")
        || input.starts_with("LL")
        || input.starts_with("FL");
    ok_prefix
        && input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn playlist_url(id: &str) -> String {
    format!("https://music.youtube.com/playlist?list={id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_inputs() {
        let cases = [
            ("dQw4w9WgXcQ", "dQw4w9WgXcQ", false),
            ("  dQw4w9WgXcQ  ", "dQw4w9WgXcQ", false),
            (
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                "dQw4w9WgXcQ",
                true,
            ),
            ("https://youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ", true),
            ("youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ", true),
            (
                "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
                "dQw4w9WgXcQ",
                true,
            ),
            (
                "https://www.youtube.com/embed/dQw4w9WgXcQ",
                "dQw4w9WgXcQ",
                true,
            ),
            (
                "https://www.youtube.com/shorts/dQw4w9WgXcQ",
                "dQw4w9WgXcQ",
                true,
            ),
            ("<https://youtu.be/dQw4w9WgXcQ>", "dQw4w9WgXcQ", true),
            ("https://youtu.be/dQw4w9WgXcQ.", "dQw4w9WgXcQ", true),
            ("never gonna give you up", "", false),
            ("", "", false),
            ("abc123", "", false),
        ];
        for (input, id, url_like) in cases {
            let (got_id, got_like) = classify_song_request_input(input);
            assert_eq!((got_id.as_str(), got_like), (id, url_like), "{input}");
        }
        let (id, like) = classify_song_request_input("https://open.spotify.com/track/abc");
        assert!(id.is_empty() && like);
        let (id, like) = classify_song_request_input("www.example.com/foo");
        assert!(id.is_empty() && like);
    }

    #[test]
    fn spotify_urls() {
        let id = "11dFghVXANMlKmJXsNCbNl";
        let want = format!("https://open.spotify.com/track/{id}");
        for input in [
            format!("https://open.spotify.com/track/{id}"),
            format!("https://open.spotify.com/track/{id}?si=abc"),
            format!("https://open.spotify.com/intl-en/track/{id}"),
            format!("https://www.open.spotify.com/track/{id}"),
            format!("open.spotify.com/track/{id}"),
            format!("spotify:track:{id}"),
            format!("<https://open.spotify.com/track/{id}>"),
            format!("https://open.spotify.com/track/{id}."),
        ] {
            assert_eq!(
                canonical_spotify_track_url(&input).as_deref(),
                Some(want.as_str()),
                "{input}"
            );
        }
        assert!(canonical_spotify_track_url("https://open.spotify.com/track/abc").is_none());
        assert!(
            canonical_spotify_track_url(&format!("https://open.spotify.com/album/{id}")).is_none()
        );
    }
}
