use std::{path::PathBuf, str::FromStr as _};

use anyhow::{Context as _, Ok, Result};
use chrono::Utc;
use db::{Message, Metadata, Schedule};
use sqlx::{Connection as _, SqliteConnection, sqlite::{SqliteConnectOptions, SqlitePoolOptions}};

use crate::BASE_DIR;

pub use manager::{SessionManager, load, SM};

mod db;
mod manager;
mod participant;

#[derive(Debug)]
pub struct Session {
    pub metadata: Metadata,
    pub messages: Vec<Message>,
    pub schedules: Vec<Schedule>,

    pub db_url: String,
}

impl Session {
    async fn load_from(path: &PathBuf) -> Result<Self> {
        let db_url = format!("sqlite://{}", path.to_str().unwrap());
        let mut conn = SqliteConnection::connect(db_url.as_str()).await?;

        let res = (async || {
            let metadata = Metadata::get(&mut conn).await?;
            let messages = Message::load_all(&mut conn).await?;
            let schedules = Schedule::load_all(&mut conn).await?;

            Ok(Self {
                metadata,
                messages,
                schedules,
                db_url,
            })
        })()
        .await;

        // TODO: add error handling, like struct error
        // if let Err(ref e) = res {
        //     tracing::error!(
        //         "Failed to load session from {}: {e:?}",
        //         path.display()
        //     );
        // }

        res
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
        // TODO: Perhaps it can be persistent
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;

        // 5. 运行迁移（建表）
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        // 6. 构建初始 metadata
        let metadata = Metadata {
            session_id: session_id,
            creator: creator,
            created_at: Utc::now(),
            archive_at: None,
        };

        // 7. 插入 metadata 到数据库
        sqlx::query(
            "INSERT INTO session_meta (session_id, creator, created_at, archive_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&metadata.session_id)
        .bind(&metadata.creator)
        .bind(metadata.created_at.timestamp()) // DateTime -> i64 (Unix 秒)
        .bind(None::<i64>) // archive_at = NULL
        .execute(&pool)
        .await
        .context("Failed to insert session metadata")?;

        // 8. 构造 Session 实例
        Ok(Self {
            metadata,
            messages: Vec::new(),
            schedules: Vec::new(),
            db_url,
        })
    }

    pub async fn insert_message(&mut self, mut msg: Message) -> Result<i64> {
        anyhow::ensure!(msg.id == -1, "msg.id should == -1");

        let mut conn = SqliteConnection::connect(&self.db_url).await?;

        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (timestamp, content, role, tag) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(&msg.timestamp)
        .bind(&msg.content)
        .bind(&msg.role)
        .bind(&msg.tag) // Option<String> 自动映射为 NULL
        .fetch_one(&mut conn)
        .await?;

        // Set right id and push new message
        msg.id = id;
        self.messages.push(msg);

        Ok(id)
    }
}
