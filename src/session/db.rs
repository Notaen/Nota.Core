use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{SqliteConnection, prelude::FromRow};

#[derive(Debug, FromRow)]
pub struct Metadata {
    pub session_id: String,
    pub creator: String,
    pub create_time: DateTime<Utc>,
    pub archive_at: Option<DateTime<Utc>>,
}

impl Metadata {
    pub async fn get(conn: &mut SqliteConnection) -> Result<Self> {
        let row: Metadata = sqlx::query_as(
            "SELECT session_id, creator, create_time, archive_at FROM session_meta LIMIT 1",
        )
        .fetch_one(conn)
        .await?;

        Ok(row)
    }
}

#[derive(Debug, FromRow)]
pub struct Message {
    /// message id. Set to -1 only when used for insertion
    pub id: i64,
    pub timestamp: i64,
    pub content: String,
    pub role: String,
    pub tag: Option<String>,
}

impl Message {
    pub async fn load_all(conn: &mut SqliteConnection) -> Result<Vec<Self>> {
        let messages = sqlx::query_as(
            "SELECT id, timestamp, content, role, tag FROM messages ORDER BY timestamp",
        )
        .fetch_all(conn)
        .await?;

        Ok(messages)
    }
}

#[derive(Debug)]
pub struct Schedule {
    pub id: i64,
    pub message: String,
    pub next_run_at: i64,
    pub interval_seconds: Option<i64>,
    pub status: String,
    pub created_at: i64,
}

// scacdcccccccccccccccccccccccccccc

pub async fn run_migrations(
    conn: &mut SqliteConnection,
) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(conn).await?;
    Ok(())
}
