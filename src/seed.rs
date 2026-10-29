use crate::ytmd;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

pub struct State {
    inner: Mutex<Inner>,
}

struct Inner {
    active: bool,
    seed_ids: HashSet<String>,
    requests: Vec<String>,
    /// Video ids that actually landed in the queue for each playlist.
    playlists: HashMap<String, Vec<String>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                active: false,
                seed_ids: HashSet::new(),
                requests: Vec::new(),
                playlists: HashMap::new(),
            }),
        }
    }

    pub fn register_enqueue(&self, playlist_id: &str, video_ids: &[String]) {
        let mut s = self.inner.lock().expect("seed");
        let playlist_key = normalize_id(playlist_id);
        for video_id in video_ids {
            let normalized = normalize_id(video_id);
            if normalized.is_empty() {
                continue;
            }
            if !playlist_key.is_empty() {
                let list = s.playlists.entry(playlist_key.clone()).or_default();
                if !list.contains(&normalized) {
                    list.push(normalized.clone());
                }
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
        for list in s.playlists.values_mut() {
            list.retain(|id| id != &normalized);
        }
        s.playlists.retain(|_, list| !list.is_empty());
        s.requests.retain(|id| id != &normalized);
        s.requests.push(normalized);
        s.active = true;
    }

    pub fn snapshot(&self) -> (bool, usize, usize) {
        let s = self.inner.lock().expect("seed");
        (s.active, s.seed_ids.len(), s.playlists.len())
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

    pub fn video_ids_for_playlist(&self, playlist_id: &str) -> HashSet<String> {
        let s = self.inner.lock().expect("seed");
        s.playlists
            .get(&normalize_id(playlist_id))
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default()
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

    /// First upcoming row that is not where the book says it belongs: `(from, to)`.
    pub fn first_move(&self, queue_payload: &Value, current_video_id: &str) -> Option<(i32, i32)> {
        let s = self.inner.lock().expect("seed");
        let ids = ordered_queue_ids(queue_payload);
        let current = ytmd::resolve_current_queue_index(queue_payload, current_video_id);
        first_misplaced(&ids, current, &s.requests)
    }

    pub fn sync_from_queue(&self, queue_payload: &Value, current_video_id: &str) {
        let mut s = self.inner.lock().expect("seed");
        sync_from_queue_locked(&mut s, queue_payload, current_video_id);
    }
}

/// Upcoming order JB2 wants: request FIFO, then every other upcoming id unchanged.
pub fn desired_upcoming(
    queue_ids: &[String],
    current_index: i32,
    requests: &[String],
) -> Vec<String> {
    if current_index < 0 {
        return Vec::new();
    }
    let start = (current_index as usize).saturating_add(1);
    if start >= queue_ids.len() {
        return Vec::new();
    }
    let upcoming = &queue_ids[start..];
    let mut desired = Vec::new();
    let mut used = vec![false; upcoming.len()];
    for request_id in requests {
        if request_id.is_empty() {
            continue;
        }
        if let Some(index) = upcoming
            .iter()
            .enumerate()
            .position(|(index, id)| !used[index] && id == request_id)
        {
            used[index] = true;
            desired.push(request_id.clone());
        }
    }
    for (index, id) in upcoming.iter().enumerate() {
        if !used[index] {
            desired.push(id.clone());
        }
    }
    desired
}

pub fn first_misplaced(
    queue_ids: &[String],
    current_index: i32,
    requests: &[String],
) -> Option<(i32, i32)> {
    if current_index < 0 {
        return None;
    }
    let desired = desired_upcoming(queue_ids, current_index, requests);
    let start = current_index + 1;
    for (offset, want) in desired.iter().enumerate() {
        if want.is_empty() {
            continue;
        }
        let to = start + offset as i32;
        if queue_ids.get(to as usize) == Some(want) {
            continue;
        }
        let Some(from) = (start as usize..queue_ids.len()).find(|&index| queue_ids[index] == *want)
        else {
            continue;
        };
        let from = from as i32;
        if from == to {
            continue;
        }
        // YTMD splices the row out first. Moving it forward lands one slot early
        // unless the destination is the post-removal index.
        let to = if from < to { to - 1 } else { to };
        return Some((from, to));
    }
    None
}

/// Highest queue index whose video id is in `targets` and not in `skip`.
pub fn highest_matching_index(
    queue_ids: &[String],
    targets: &HashSet<String>,
    skip: &HashSet<String>,
) -> Option<(i32, String)> {
    let mut best: Option<(i32, String)> = None;
    for (index, id) in queue_ids.iter().enumerate() {
        if id.is_empty() || skip.contains(id) || !targets.contains(id) {
            continue;
        }
        best = Some((index as i32, id.clone()));
    }
    best
}

pub fn ordered_queue_ids(payload: &Value) -> Vec<String> {
    let items = ytmd::queue_video_ids_by_index(payload);
    if items.is_empty() {
        return Vec::new();
    }
    let max = items.keys().copied().max().unwrap_or(-1);
    (0..=max)
        .map(|index| items.get(&index).cloned().unwrap_or_default())
        .collect()
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
    let ids = ordered_queue_ids(queue_payload);
    let desired = desired_upcoming(&ids, current_index, requests);
    let request_set: HashSet<&String> = requests.iter().collect();
    let prefix = desired
        .iter()
        .take_while(|id| request_set.contains(*id))
        .count();
    current_index + 1 + prefix as i32
}

fn sync_from_queue_locked(s: &mut Inner, queue_payload: &Value, current_video_id: &str) {
    if s.seed_ids.is_empty() && s.requests.is_empty() {
        s.active = false;
        return;
    }
    let items = ytmd::queue_video_ids_by_index(queue_payload);
    // No usable video ids: keep the book. A bad read must not forget the queue JB2 owns.
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
    for list in s.playlists.values_mut() {
        list.retain(|id| s.seed_ids.contains(id));
    }
    s.playlists.retain(|_, list| !list.is_empty());
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
        s.playlists.clear();
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

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|id| (*id).to_string()).collect()
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

    #[test]
    fn request_slot_is_next_not_after_seed_tail() {
        let tail = ids(&["s1", "s2", "s3", "r1"]);
        let requests = ids(&["r1"]);
        assert_eq!(
            desired_upcoming(&tail, 0, &requests),
            ids(&["r1", "s2", "s3"])
        );
        assert_eq!(first_misplaced(&tail, 0, &requests), Some((3, 1)));

        let packed = ids(&["s1", "r1", "s2", "s3"]);
        assert_eq!(
            desired_upcoming(&packed, 0, &requests),
            ids(&["r1", "s2", "s3"])
        );
        assert_eq!(first_misplaced(&packed, 0, &requests), None);

        let both = ids(&["s1", "s2", "r1", "r2"]);
        let requests = ids(&["r1", "r2"]);
        assert_eq!(
            desired_upcoming(&both, 0, &requests),
            ids(&["r1", "r2", "s2"])
        );
        assert_eq!(first_misplaced(&both, 0, &requests), Some((2, 1)));
    }

    #[test]
    fn unreadable_queue_does_not_clear_seed() {
        let s = State::new();
        s.register_enqueue("pl", &["s1".into()]);
        s.sync_from_queue(&json!({"items": [{"title": "no id"}]}), "s1");
        assert!(s.is_active());
        assert_eq!(s.video_id_set().len(), 1);
        s.sync_from_queue(&json!({"items": []}), "s1");
        assert!(s.is_active());
        assert!(s.video_ids_for_playlist("pl").contains("s1"));
    }

    #[test]
    fn playlist_ids_stay_separate() {
        let s = State::new();
        s.register_enqueue("a", &["a1".into(), "a2".into()]);
        s.register_enqueue("B", &["b1".into()]);
        let a = s.video_ids_for_playlist("A");
        let b = s.video_ids_for_playlist("b");
        assert_eq!(a.len(), 2);
        assert!(a.contains("a1") && a.contains("a2"));
        assert_eq!(b.len(), 1);
        assert!(b.contains("b1"));
        assert!(a.is_disjoint(&b));
        let (_, videos, playlists) = s.snapshot();
        assert_eq!(videos, 3);
        assert_eq!(playlists, 2);

        let queue = ids(&["cur", "a2", "b1"]);
        let targets = s.video_ids_for_playlist("a");
        assert_eq!(
            highest_matching_index(&queue, &targets, &HashSet::new()),
            Some((1, "a2".into()))
        );
    }
}
