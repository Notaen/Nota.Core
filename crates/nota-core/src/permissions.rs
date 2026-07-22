use std::collections::HashMap;

use tokio::sync::{RwLock, oneshot};

pub struct PermissionRegistry {
    pending: RwLock<HashMap<String, oneshot::Sender<bool>>>,
}

impl PermissionRegistry {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self) -> (String, oneshot::Receiver<bool>) {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id.clone(), tx);
        (id, rx)
    }

    pub async fn resolve(&self, id: &str, approved: bool) -> bool {
        let mut pending = self.pending.write().await;
        if let Some(tx) = pending.remove(id) {
            tx.send(approved).is_ok()
        } else {
            false
        }
    }
}

impl Default for PermissionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
