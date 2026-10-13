use crate::config::Config;
use serde_json::Value;

pub fn check_content(
    cfg: &Config,
    duration_seconds: i32,
    require_duration: bool,
    candidates: &[&str],
) -> Result<(), String> {
    for candidate in candidates {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        for entry in &cfg.blacklist {
            let entry_trim = entry.trim();
            if trimmed.eq_ignore_ascii_case(entry_trim)
                || trimmed.to_lowercase().contains(&entry_trim.to_lowercase())
            {
                return Err(format!("request blocked by blacklist entry \"{entry}\""));
            }
        }
    }
    if require_duration {
        if duration_seconds <= 0 {
            return Err("request blocked because song duration could not be verified".into());
        }
        if duration_seconds > cfg.max_duration {
            return Err(format!(
                "request blocked because duration {duration_seconds} exceeds maxDuration {}",
                cfg.max_duration
            ));
        }
        return Ok(());
    }
    if duration_seconds > cfg.max_duration {
        return Err(format!(
            "request blocked because duration {duration_seconds} exceeds maxDuration {}",
            cfg.max_duration
        ));
    }
    Ok(())
}

pub fn enforce(cfg: &Config, query: &[(String, String)], body: &[u8]) -> Result<(), String> {
    let mut text = Vec::new();
    for key in ["query", "title", "artist", "name", "url"] {
        if let Some(v) = query_get(query, key) {
            text.push(v);
        }
    }
    if !body.iter().all(|b| b.is_ascii_whitespace()) {
        if let Ok(s) = std::str::from_utf8(body) {
            text.push(s.to_string());
        }
        collect_strings(&parse_body(body), &mut text);
    }

    let mut duration = 0;
    for key in ["duration", "seconds", "length"] {
        if let Some(v) = query_get(query, key) {
            if let Ok(n) = v.parse::<i32>() {
                duration = n;
                break;
            }
        }
    }
    if duration == 0 {
        duration = duration_from_json(&parse_body(body));
    }

    let refs: Vec<&str> = text.iter().map(|s| s.as_str()).collect();
    check_content(cfg, duration, false, &refs)
}

fn query_get(query: &[(String, String)], key: &str) -> Option<String> {
    query
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_body(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if !s.trim().is_empty() => out.push(s.clone()),
        Value::Array(arr) => {
            for item in arr {
                collect_strings(item, out);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_strings(item, out);
            }
        }
        _ => {}
    }
}

fn duration_from_json(payload: &Value) -> i32 {
    let Some(obj) = payload.as_object() else {
        return 0;
    };
    for key in ["duration", "seconds", "length"] {
        if let Some(v) = obj.get(key) {
            match v {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        return i as i32;
                    }
                    if let Some(f) = n.as_f64() {
                        return f as i32;
                    }
                }
                Value::String(s) => {
                    if let Ok(n) = s.trim().parse::<i32>() {
                        return n;
                    }
                }
                _ => {}
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn blocks_blacklisted_song() {
        let cfg = Config {
            blacklist: vec!["Rick Astley".into()],
            max_duration: 600,
            ..Config::default()
        };
        assert!(check_content(&cfg, 214, true, &["Never Gonna Give You Up", "Rick Astley"]).is_err());
    }

    #[test]
    fn blocks_overlong() {
        let cfg = Config {
            blacklist: vec!["Rick Astley".into()],
            max_duration: 120,
            ..Config::default()
        };
        assert!(check_content(&cfg, 214, true, &["Safe Artist", "Safe Title"]).is_err());
    }

    #[test]
    fn requires_duration() {
        let cfg = Config {
            blacklist: vec!["Rick Astley".into()],
            max_duration: 600,
            ..Config::default()
        };
        assert!(check_content(&cfg, 0, true, &["Safe Artist", "Safe Title"]).is_err());
    }

    #[test]
    fn allows_normal() {
        let cfg = Config {
            blacklist: vec!["Rick Astley".into()],
            max_duration: 600,
            ..Config::default()
        };
        assert!(check_content(&cfg, 180, true, &["Safe Artist", "Safe Title"]).is_ok());
    }

    #[test]
    fn enforce_query() {
        let cfg = Config {
            blacklist: vec!["rick astley".into(), "never gonna give you up".into()],
            max_duration: 600,
            ..Config::default()
        };
        let q = vec![("query".into(), "Never Gonna Give You Up".into())];
        assert!(enforce(&cfg, &q, b"").is_err());
    }
}
