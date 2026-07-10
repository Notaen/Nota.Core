use anyhow::Result;
use async_trait::async_trait;

use super::{Message, Metadata, SessionSnapshot};

/// Persistence port for sessions.
///
/// Infrastructure provides the implementation (e.g. SQLite + crudly). The core
/// never sees a connection pool or SQL: it only talks through this trait.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Create a new session DB and return its id (assigned by the repo) plus
    /// the initial [`Metadata`].
    async fn create_session(&self, creator: String) -> Result<(String, Metadata)>;

    /// Load every active session from disk (cold start).
    async fn load_all(&self) -> Result<Vec<SessionSnapshot>>;

    /// Persist a new message and return its DB-assigned id.
    async fn insert_message(&self, session_id: &str, msg: Message) -> Result<i64>;

    /// Update the archive timestamp of a session.
    async fn set_archive_at(&self, session_id: &str, at: Option<i64>) -> Result<()>;

    /// Move a session's storage to the archive location.
    async fn archive_session(&self, session_id: &str) -> Result<()>;
}
