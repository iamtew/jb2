use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Pad an SSE frame to one TCP segment.
///
/// ponytail: tiny_http has no set_nodelay. Windows delayed-ACK holds a short
/// write ~200ms. Spaces go on a `:` comment line so EventSource ignores them.
pub fn pad_sse_frame(frame: &str) -> String {
    let mut s = frame.trim_end().to_string();
    s.push_str("\n:");
    if s.len() + 2 < 1600 {
        s.push_str(&" ".repeat(1600 - 2 - s.len()));
    }
    s.push_str("\n\n");
    s
}

/// ponytail: overlay + Stage + a handful of Admin docks.
const MAX_SUBSCRIBERS: usize = 16;

#[derive(Clone)]
pub struct EventBus {
    clients: Arc<Mutex<Vec<Sender<String>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn emit(&self, kind: &str, data: &Value) {
        let frame = format!("event: {kind}\ndata: {data}\n\n");
        let mut clients = self.clients.lock().expect("events");
        clients.retain(|tx| tx.send(frame.clone()).is_ok());
    }

    pub fn subscribe(&self) -> Option<Receiver<String>> {
        let mut clients = self.clients.lock().expect("events");
        clients.retain(|tx| tx.send(": \n\n".into()).is_ok());
        if clients.len() >= MAX_SUBSCRIBERS {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        clients.push(tx);
        Some(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn bus_broadcasts_and_drops_dead() {
        let bus = EventBus::new();
        let rx = bus.subscribe().unwrap();
        bus.emit("queue", &json!({}));
        let frame = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(frame.contains("event: queue"), "{frame}");
        assert!(frame.contains("data: {}"), "{frame}");
        drop(rx);
        bus.emit("queue", &json!({}));
    }

    #[test]
    fn bus_caps_subscribers() {
        let bus = EventBus::new();
        let mut held = Vec::new();
        for _ in 0..MAX_SUBSCRIBERS {
            held.push(bus.subscribe().unwrap());
        }
        assert!(bus.subscribe().is_none());
        drop(held);
        assert!(bus.subscribe().is_some());
    }

    #[test]
    fn bus_keeps_overlay_frames_when_slow() {
        let bus = EventBus::new();
        let rx = bus.subscribe().unwrap();
        for i in 0..32 {
            bus.emit("overlay", &json!({ "x": i }));
        }
        let mut n = 0;
        while rx.recv_timeout(Duration::from_millis(10)).is_ok() {
            n += 1;
        }
        assert_eq!(n, 32);
    }

    #[test]
    fn pad_sse_frame_comment_and_size() {
        let padded = pad_sse_frame("event: overlay\ndata: {\"x\":1}\n\n");
        assert!(padded.contains("event: overlay"), "{padded}");
        assert!(padded.contains("data: {\"x\":1}"), "{padded}");
        assert!(padded.contains("\n:"), "{padded}");
        assert!(padded.len() >= 1600, "{}", padded.len());
        assert!(padded.ends_with("\n\n"));
        let data_line = padded
            .lines()
            .find(|line| line.starts_with("data: "))
            .unwrap();
        assert_eq!(data_line, "data: {\"x\":1}");
    }
}
