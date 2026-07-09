use std::{path::PathBuf, str::FromStr as _};

use anyhow::{Context as _, Ok, Result};
use async_trait::async_trait;
use chrono::Utc;
use db::{Message, Metadata, Schedule};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::BASE_DIR;

// 还是不要暴露太多module，耦合太强了。但我还没想好怎么做
pub mod db;
pub mod manager;
mod participant;

pub use manager::SM;

pub struct Session {
    pub metadata: Metadata,
    pub messages: Vec<Message>,
    pub schedules: Vec<Schedule>,
    pub handlers: Vec<Box<dyn SessionHandler>>,
    pub db_pool: SqlitePool,
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
    async fn load_from(path: &PathBuf) -> Result<Self> {
        let db_url = format!("sqlite://{}", path.to_str().unwrap());
        let pool = SqlitePool::connect(db_url.as_str()).await?;

        use crudly::SelectAll;

        let metadata = Metadata::read_from(&pool).await?;
        let messages = Message::select_all(&pool).await?;
        let schedules = Schedule::select_all(&pool).await?;

        Ok(Self {
            metadata,
            messages,
            schedules,
            handlers: Vec::new(),
            db_pool: pool,
        })
    }

    pub async fn new(session_id: String, creator: String) -> Result<Self> {
        let db_path = BASE_DIR
            .join("sessions")
            .join(format!("{}.sqlite", &session_id));
        if db_path.exists() {
            anyhow::bail!("Session database already exists: {}", db_path.display());
        }

        let db_url = format!("sqlite://{}", db_path.to_string_lossy());

        // create and open the db
        let options = SqliteConnectOptions::from_str(db_url.as_str())?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        // 5. 运行迁移（建表）
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        // 6. 构建初始 metadata
        let metadata = Metadata {
            session_id,
            creator,
            created_at: Utc::now().timestamp(),
            archive_at: None,
        };

        // 7. 插入 metadata 到数据库
        use crudly::InsertNoId;
        metadata.clone().insert(&pool).await?;

        // 8. 构造 Session 实例
        Ok(Self {
            metadata,
            messages: Vec::new(),
            schedules: Vec::new(),
            handlers: Vec::new(),
            db_pool: pool,
        })
    }

    pub async fn insert_message(&mut self, msg: Message) -> Result<i64> {
        use crudly::InsertWithoutId;
        let id = msg.clone().insert(&self.db_pool).await?;
        self.messages.push(Message { id, ..msg });

        // Fire session handlers
        let session_id = self.metadata.session_id.clone();
        let handlers: Vec<_> = self.handlers.iter().collect();
        for handler in handlers {
            if let Err(e) = handler.handle(&session_id, &self.messages).await {
                tracing::error!("Session handler error (session={session_id}): {e:?}");
            }
        }

        Ok(id)
    }

    pub async fn set_archive_at(&mut self, at: Option<i64>) {
        self.metadata.archive_at = at;
        SM.get().unwrap().archive_expired_sessions().await;
    }
}

#[async_trait]
pub trait SessionHandler: Send + Sync {
    async fn handle(&self, session_id: &str, messages: &[Message]) -> anyhow::Result<()>;
}
