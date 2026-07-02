use std::{path::PathBuf, str::FromStr as _};

use anyhow::{Context as _, Ok, Result};
use chrono::Utc;
use db::{Message, Metadata, Schedule};
use sqlx::{Connection as _, SqliteConnection, sqlite::SqliteConnectOptions};

use crate::base_dir;

pub mod db;
pub mod manager;
mod participant;

struct Session {
    metadata: Metadata,
    messages: Vec<Message>,
    schedules: Vec<Schedule>,

    conn: SqliteConnection,
}

impl Session {
    async fn load(path: &PathBuf) -> Result<Self> {
        let url = format!("sqlite://{}", path.to_str().unwrap());
        let mut conn = SqliteConnection::connect(&url).await?;

        let metadata = Metadata::get(&mut conn).await?;
        let messages = Message::load_all(&mut conn).await?;
        let schedules = Schedule::load_all(&mut conn).await?;

        Ok(Self {
            metadata,
            messages,
            schedules,
            conn,
        })
    }

    pub async fn new(session_id: &str, creator: &str) -> Result<Self> {
        let file_path = base_dir()
            .join("sessions")
            .join(format!("{}.sqlite", session_id));
        if file_path.exists() {
            anyhow::bail!("Session database already exists: {}", file_path.display());
        }

        let url = format!("sqlite://{}", file_path.to_str().unwrap());
        let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);

        // 4. 建立连接
        let mut conn = SqliteConnection::connect_with(&options).await?;

        // 5. 运行迁移（建表）
        sqlx::migrate!("./migrations")
            .run(&mut conn)
            .await
            .context("Failed to run migrations")?;

        // 6. 构建初始 metadata
        let now = Utc::now();
        let metadata = Metadata {
            session_id: session_id.to_string(),
            creator: creator.to_string(),
            create_time: now,
            archive_at: None,
        };

        // 7. 插入 metadata 到数据库
        sqlx::query(
            "INSERT INTO session_meta (session_id, creator, create_time, archive_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&metadata.session_id)
        .bind(&metadata.creator)
        .bind(metadata.create_time.timestamp()) // DateTime -> i64 (Unix 秒)
        .bind::<Option<i64>>(None) // archive_at = NULL
        .execute(&mut conn)
        .await
        .context("Failed to insert session metadata")?;

        // 8. 构造 Session 实例
        Ok(Self {
            metadata,
            messages: Vec::new(),  // 新会话无消息
            schedules: Vec::new(), // 新会话无计划
            conn,
        })
    }

    pub async fn insert_message(&mut self, mut msg: Message) -> Result<i64> {
        anyhow::ensure!(msg.id == -1, "msg.id should == -1");

        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (timestamp, content, role, tag) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(&msg.timestamp)
        .bind(&msg.content)
        .bind(&msg.role)
        .bind(&msg.tag) // Option<String> 自动映射为 NULL
        .fetch_one(&mut self.conn)
        .await?;

        // Set right id and push new message
        msg.id = id;
        self.messages.push(msg);

        Ok(id)
    }
}
