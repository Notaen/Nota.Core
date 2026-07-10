use std::sync::Arc;

use serde::Serialize;

use super::SessionHandler;

#[derive(Debug, Serialize, Clone)]
pub struct Metadata {
    pub session_id: String,
    pub creator: String,
    pub created_at: i64,
    pub archive_at: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Message {
    pub id: i64,
    pub timestamp: i64,
    pub content: String,
    pub role: String,
    pub tag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub id: i64,
    pub message: String,
    pub next_run_at: i64,
    pub interval_seconds: Option<i64>,
    pub status: String,
    pub created_at: i64,
}

/// A persisted session loaded wholesale from the repository (cold start).
pub struct SessionSnapshot {
    pub metadata: Metadata,
    pub messages: Vec<Message>,
    pub schedules: Vec<Schedule>,
}

/// In-memory session aggregate.
///
/// Unlike the pre-refactor `Session`, this holds no `SqlitePool`: persistence
/// is delegated to the injected [`super::SessionRepository`].
pub struct Session {
    pub metadata: Metadata,
    pub messages: Vec<Message>,
    pub schedules: Vec<Schedule>,
    pub handlers: Vec<Arc<dyn SessionHandler>>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("metadata", &self.metadata)
            .field("messages", &self.messages)
            .field("schedules", &self.schedules)
            .field("handlers", &self.handlers.len())
            .finish()
    }
}

impl Session {
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            messages: Vec::new(),
            schedules: Vec::new(),
            handlers: Vec::new(),
        }
    }

    pub fn from_snapshot(snap: SessionSnapshot) -> Self {
        Self {
            metadata: snap.metadata,
            messages: snap.messages,
            schedules: snap.schedules,
            handlers: Vec::new(),
        }
    }
}
