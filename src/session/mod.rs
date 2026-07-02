use std::{path::PathBuf, str::FromStr};

use anyhow::{Context as _, Ok, Result};
use chrono::Utc;
use db::{Message, Metadata, Schedule};
use sqlx::{Connection as _, SqliteConnection, sqlite::SqliteConnectOptions};

use crate::base_dir;

pub mod db;
pub mod manager;
pub mod markdown;
mod participant;

struct Session {
    metadata: Metadata,
    messages: Vec<Message>,
    schedules: Vec<Schedule>,

    conn: SqliteConnection,
}

fn schedule_path(session_id: &str) -> PathBuf {
    base_dir().join("persona").join(format!("{}_tasks.md", session_id))
}

impl Session {
    async fn load(path: &PathBuf) -> Result<Self> {
        let url = format!("sqlite://{}", path.to_str().unwrap());
        let mut conn = SqliteConnection::connect(&url).await?;

        let metadata = Metadata::get(&mut conn).await?;
        let messages = Message::load_all(&mut conn).await?;

        let md_path = schedule_path(&metadata.session_id);
        let schedules = markdown::load(&md_path)?;

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

        let mut conn = SqliteConnection::connect_with(&options).await?;

        sqlx::migrate!("./migrations")
            .run(&mut conn)
            .await
            .context("Failed to run migrations")?;

        let now = Utc::now();
        let metadata = Metadata {
            session_id: session_id.to_string(),
            creator: creator.to_string(),
            create_time: now,
            archive_at: None,
        };

        sqlx::query(
            "INSERT INTO session_meta (session_id, creator, create_time, archive_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&metadata.session_id)
        .bind(&metadata.creator)
        .bind(metadata.create_time.timestamp())
        .bind::<Option<i64>>(None)
        .execute(&mut conn)
        .await
        .context("Failed to insert session metadata")?;

        let md_path = schedule_path(session_id);
        markdown::save(&md_path, &[])?;

        Ok(Self {
            metadata,
            messages: Vec::new(),
            schedules: Vec::new(),
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
        .bind(&msg.tag)
        .fetch_one(&mut self.conn)
        .await?;

        msg.id = id;
        self.messages.push(msg);

        Ok(id)
    }
}