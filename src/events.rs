use serde_json::Value;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

fn keep_client(result: Result<(), TrySendError<String>>) -> bool {
    !matches!(result, Err(TrySendError::Disconnected(_)))
}

/// ponytail: overlay + Stage + a handful of Admin docks.
const MAX_SUBSCRIBERS: usize = 16;
const CLIENT_BUF: usize = 8;

#[derive(Clone)]
pub struct EventBus {
    clients: Arc<Mutex<Vec<SyncSender<String>>>>,
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
        clients.retain(|tx| keep_client(tx.try_send(frame.clone())));
    }

    pub fn subscribe(&self) -> Option<Receiver<String>> {
        let mut clients = self.clients.lock().expect("events");
        clients.retain(|tx| keep_client(tx.try_send(": \n\n".into())));
        if clients.len() >= MAX_SUBSCRIBERS {
            return None;
        }
        let (tx, rx) = mpsc::sync_channel(CLIENT_BUF);
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
}
