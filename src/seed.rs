use crate::ytmd;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct State {
    inner: Mutex<Inner>,
}

struct Inner {
    active: bool,
    seed_ids: HashSet<String>,
    requests: Vec<String>,
    playlist_ids: HashSet<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                active: false,
                seed_ids: HashSet::new(),
                requests: Vec::new(),
                playlist_ids: HashSet::new(),
            }),
        }
    }

    pub fn register_enqueue(&self, playlist_id: &str, video_ids: &[String]) {
        let mut s = self.inner.lock().expect("seed");
        if !playlist_id.is_empty() {
            s.playlist_ids.insert(playlist_id.trim().to_lowercase());
        }
        for video_id in video_ids {
            let normalized = normalize_id(video_id);
            if normalized.is_empty() {
                continue;
            }
            s.seed_ids.insert(normalized.clone());
            s.requests.retain(|id| id != &normalized);
        }
        if !s.seed_ids.is_empty() {
            s.active = true;
        }
    }

    pub fn append_request(&self, video_id: &str) {
        let mut s = self.inner.lock().expect("seed");
        let normalized = normalize_id(video_id);
        if normalized.is_empty() {
            return;
        }
        s.seed_ids.remove(&normalized);
        s.requests.retain(|id| id != &normalized);
        s.requests.push(normalized);
        s.active = true;
    }

    pub fn snapshot(&self) -> (bool, usize, usize) {
        let s = self.inner.lock().expect("seed");
        (s.active, s.seed_ids.len(), s.playlist_ids.len())
    }

    pub fn request_count(&self) -> usize {
        self.inner.lock().expect("seed").requests.len()
    }

    pub fn is_active(&self) -> bool {
        self.inner.lock().expect("seed").active
    }

    pub fn video_id_set(&self) -> HashSet<String> {
        self.inner.lock().expect("seed").seed_ids.clone()
    }

    pub fn request_ids(&self) -> Vec<String> {
        self.inner.lock().expect("seed").requests.clone()
    }

    pub fn classify_video_id(&self, video_id: &str) -> &'static str {
        let s = self.inner.lock().expect("seed");
        let normalized = normalize_id(video_id);
        if normalized.is_empty() {
            return "unknown";
        }
        if s.requests.iter().any(|id| id == &normalized) {
            return "request";
        }
        if s.seed_ids.contains(&normalized) {
            return "seed";
        }
        "other"
    }

    pub fn request_insert_index(&self, queue_payload: &Value, current_video_id: &str) -> i32 {
        let s = self.inner.lock().expect("seed");
        request_insert_index_locked(queue_payload, current_video_id, &s.requests)
    }

    pub fn sync_from_queue(&self, queue_payload: &Value, current_video_id: &str) {
        let mut s = self.inner.lock().expect("seed");
        sync_from_queue_locked(&mut s, queue_payload, current_video_id);
    }
}

fn request_insert_index_locked(
    queue_payload: &Value,
    current_video_id: &str,
    requests: &[String],
) -> i32 {
    let current_index = ytmd::resolve_current_queue_index(queue_payload, current_video_id);
    if current_index < 0 {
        return -1;
    }
    let request_set: HashSet<&String> = requests.iter().collect();
    let items = ytmd::queue_video_ids_by_index(queue_payload);
    let mut insert_after = current_index;
    let mut index = current_index + 1;
    loop {
        match items.get(&index) {
            Some(video_id) if request_set.contains(video_id) => {
                insert_after = index;
                index += 1;
            }
            _ => break,
        }
    }
    insert_after + 1
}

fn sync_from_queue_locked(s: &mut Inner, queue_payload: &Value, current_video_id: &str) {
    if s.seed_ids.is_empty() && s.requests.is_empty() {
        s.active = false;
        return;
    }
    let items = ytmd::queue_video_ids_by_index(queue_payload);
    if items.is_empty() {
        return;
    }
    let mut present = HashSet::new();
    let mut max_index = -1;
    for (index, video_id) in &items {
        present.insert(video_id.clone());
        if *index > max_index {
            max_index = *index;
        }
    }
    let current_id = normalize_id(current_video_id);
    let current_index = ytmd::resolve_current_queue_index(queue_payload, current_video_id);
    if s.active && !current_id.is_empty() {
        let is_tracked_request = s.requests.iter().any(|id| id == &current_id);
        if !is_tracked_request {
            s.seed_ids.insert(current_id);
        }
    }
    s.seed_ids.retain(|id| present.contains(id));
    let request_set: HashSet<String> = s.requests.iter().cloned().collect();
    let mut rebuilt = Vec::new();
    let mut seen = HashSet::new();
    for index in 0..=max_index {
        let Some(video_id) = items.get(&index) else {
            continue;
        };
        if video_id.is_empty() || !request_set.contains(video_id) {
            continue;
        }
        if seen.insert(video_id.clone()) {
            rebuilt.push(video_id.clone());
        }
    }
    for id in &s.requests {
        if seen.contains(id) {
            continue;
        }
        if present.contains(id) {
            continue;
        }
        rebuilt.push(id.clone());
        seen.insert(id.clone());
    }
    s.requests = rebuilt;
    let mut seed_present = s.seed_ids.iter().any(|id| present.contains(id));
    if !seed_present && current_index >= 0 {
        if let Some(video_id) = items.get(&current_index) {
            if s.seed_ids.contains(video_id) {
                seed_present = true;
            }
        }
    }
    let request_present = !s.requests.is_empty();
    s.active = seed_present || request_present;
    if !s.active {
        s.seed_ids.clear();
        s.requests.clear();
        s.playlist_ids.clear();
    }
}

fn normalize_id(video_id: &str) -> String {
    video_id.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn queue(ids: &[&str]) -> Value {
        json!({
            "items": ids.iter().map(|id| json!({"videoId": id, "title": id})).collect::<Vec<_>>()
        })
    }

    #[test]
    fn insert_after_current_seed_then_after_requests() {
        let s = State::new();
        s.register_enqueue("pl", &["s1".into(), "s2".into(), "s3".into()]);
        let q = queue(&["s1", "s2", "s3"]);
        assert_eq!(s.request_insert_index(&q, "s1"), 1);

        s.append_request("r1");
        let q = queue(&["s1", "r1", "s2", "s3"]);
        assert_eq!(s.request_insert_index(&q, "s1"), 2);

        s.append_request("r2");
        let q = queue(&["s1", "r1", "r2", "s2", "s3"]);
        assert_eq!(s.request_insert_index(&q, "s1"), 3);
    }

    #[test]
    fn insert_after_playing_request_block() {
        let s = State::new();
        s.append_request("r1");
        s.append_request("r2");
        let q = queue(&["r1", "r2", "s1"]);
        assert_eq!(s.request_insert_index(&q, "r1"), 2);
    }
}
