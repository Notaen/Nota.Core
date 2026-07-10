use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

mod entity;
mod repository;

pub use entity::{Message, Metadata, Schedule, Session, SessionSnapshot};
pub use repository::SessionRepository;

/// Hook invoked after a message is persisted into a session.
///
/// Implementations (e.g. `PersonaManager`) react to new messages without
/// coupling to the persistence layer.
#[async_trait]
pub trait SessionHandler: Send + Sync {
    async fn handle(&self, session_id: &str, messages: &[Message]) -> Result<()>;
}

/// In-memory orchestrator for sessions.
///
/// Holds no global state: a [`SessionRepository`] port is injected at
/// construction, so persistence details (SQLite, crudly, pools) live entirely
/// in the infrastructure layer. Sessions are cached in memory and synced to the
/// repository on every mutating operation.
pub struct SessionManager {
    repo: Arc<dyn SessionRepository>,
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new(repo: Arc<dyn SessionRepository>) -> Self {
        Self {
            repo,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Cold start: load every session snapshot from the repository and cache it
    /// in memory without any handlers attached.
    pub async fn load_all(&self) -> Result<()> {
        let snapshots = self.repo.load_all().await?;
        let mut sessions = self.sessions.write().await;
        for snap in snapshots {
            sessions.insert(snap.metadata.session_id.clone(), Session::from_snapshot(snap));
        }
        log::info!("SessionManager loaded");
        Ok(())
    }

    /// Attach a handler to a single session.
    pub async fn register_handler(&self, session_id: &str, handler: Arc<dyn SessionHandler>) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.handlers.push(handler);
        Ok(())
    }

    /// Attach the same handler to every loaded session (used during cold start).
    pub async fn register_handler_all(&self, handler: Arc<dyn SessionHandler>) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        for session in sessions.values_mut() {
            session.handlers.push(handler.clone());
        }
        log::info!("Default handlers injected into all sessions");
        Ok(())
    }

    pub async fn new_session(&self, creator: String) -> Result<String> {
        // The repository assigns the session id (e.g. a DB-friendly UUID),
        // keeping id generation an infrastructure concern.
        let (sid, metadata) = self.repo.create_session(creator).await?;
        let session = Session::new(metadata);
        self.sessions.write().await.insert(sid.clone(), session);
        Ok(sid)
    }

    pub async fn insert_message(&self, session_id: &str, msg: Message) -> Result<i64> {
        let id = self.repo.insert_message(session_id, msg.clone()).await?;

        // Fire session handlers outside the write lock to avoid reentrant
        // deadlocks (a handler may itself touch the manager).
        let (messages_clone, handlers): (Vec<Message>, Vec<Arc<dyn SessionHandler>>) = {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            session.messages.push(Message { id, ..msg });
            (session.messages.clone(), session.handlers.clone())
        };

        for handler in handlers {
            if let Err(e) = handler.handle(session_id, &messages_clone).await {
                log::error!("Session handler error (session={session_id}): {e:?}");
            }
        }

        Ok(id)
    }

    pub async fn set_archive_at(&self, session_id: &str, at: Option<i64>) -> Result<()> {
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            session.metadata.archive_at = at;
        }
        self.repo.set_archive_at(session_id, at).await?;

        // 这有bug —— 原实现在持有 session_map.write() 时回调
        // SessionManager::get().archive_expired_sessions()，造成重入死锁。
        // DI 重构后不再走全局单例，重入消失；归档扫描由调用方/调度器显式触发。
        Ok(())
    }

    pub async fn archive_expired_sessions(&self) {
        // TODO：现在的逻辑有问题，调用后会卡住
        // （DI 后重入死锁已消除，但归档调度策略仍待重做 —— 本次不在范围内。）
        let expired_ids = {
            let sessions = self.sessions.read().await;
            let now = chrono::Utc::now().timestamp();
            sessions
                .iter()
                .filter_map(|(id, session)| {
                    let archive_at = session.metadata.archive_at.as_ref()?;
                    if *archive_at <= now {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        for session_id in &expired_ids {
            match self.repo.archive_session(session_id).await {
                Ok(()) => {
                    self.sessions.write().await.remove(session_id);
                    log::info!("Archived session {session_id}");
                }
                Err(e) => log::error!("Failed to archive session {session_id}: {e:?}"),
            }
        }
    }

    pub async fn list_metadata(&self) -> Vec<Metadata> {
        self.sessions.read().await.values().map(|s| s.metadata.clone()).collect()
    }

    pub async fn get_archive_at(&self, session_id: &str) -> Option<Option<i64>> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .map(|s| s.metadata.archive_at)
    }
}
