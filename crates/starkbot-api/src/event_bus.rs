use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// An internal event flowing through the event bus.
#[derive(Debug, Clone)]
pub struct InternalEvent {
    pub timestamp: String,
    pub kind: String,
    pub payload: String,
}

/// A simple broadcast-based event bus with a ring buffer log.
pub struct EventBus {
    tx: broadcast::Sender<InternalEvent>,
    log: Arc<Mutex<VecDeque<InternalEvent>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            tx,
            log: Arc::new(Mutex::new(VecDeque::with_capacity(200))),
        }
    }

    pub fn emit(&self, kind: &str, payload: &str) {
        let event = InternalEvent {
            timestamp: chrono::Local::now().to_rfc3339(),
            kind: kind.to_string(),
            payload: payload.to_string(),
        };
        // Don't log high-frequency pulse events — they'd drown out useful entries
        if kind != "pulse" {
            let mut log = self.log.lock().unwrap();
            if log.len() >= 200 {
                log.pop_front();
            }
            log.push_back(event.clone());
        }
        // Ignore send errors (no subscribers is fine)
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<InternalEvent> {
        self.tx.subscribe()
    }

    pub fn recent_events(&self) -> Vec<InternalEvent> {
        let log = self.log.lock().unwrap();
        log.iter().cloned().collect()
    }
}
