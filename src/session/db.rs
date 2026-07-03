use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, prelude::FromRow};

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Metadata {
    pub session_id: String,
    pub creator: String,
    pub created_at: DateTime<Utc>,
    pub archive_at: Option<DateTime<Utc>>,
}

impl Metadata {
    pub async fn get(conn: &mut SqliteConnection) -> Result<Self, sqlx::Error> {
        let row: Metadata = sqlx::query_as(
            "SELECT session_id, creator, created_at, archive_at FROM session_meta LIMIT 1",
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
    pub async fn load_all(conn: &mut SqliteConnection) -> Result<Vec<Self>, sqlx::Error> {
        let messages = sqlx::query_as(
            "SELECT id, timestamp, content, role, tag FROM messages ORDER BY timestamp",
        )
        .fetch_all(conn)
        .await?;

        Ok(messages)
    }
}

#[derive(Debug, FromRow)]
pub struct Schedule {
    /// schedule id. Set to -1 only when used for insertion
    pub id: i64,
    pub message: String,
    pub next_run_at: i64,
    pub interval_seconds: Option<i64>, // NULL -> 单次任务
    pub status: String,                // "active", "paused", "completed"
    pub created_at: i64,
}

impl Schedule {
    pub async fn load_all(conn: &mut SqliteConnection) -> Result<Vec<Self>, sqlx::Error> {
        let schedules = sqlx::query_as(
            "SELECT id, message, next_run_at, interval_seconds, status, created_at FROM schedules",
        )
        .fetch_all(conn)
        .await?;

        Ok(schedules)
    }
}
