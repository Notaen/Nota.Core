use std::sync::RwLock;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct BusEvent {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub context: String,
    pub request_id: Option<String>,
}

pub struct EventBus {
    senders: RwLock<Vec<UnboundedSender<BusEvent>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(Vec::new()),
        }
    }

    pub fn subscribe(&self) -> UnboundedReceiver<BusEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.senders.write().unwrap().push(tx);
        rx
    }

    pub fn send(&self, event: BusEvent) {
        let senders = self.senders.read().unwrap();
        for tx in senders.iter() {
            let _ = tx.send(event.clone());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
