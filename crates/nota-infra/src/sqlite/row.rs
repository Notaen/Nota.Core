use nota_core::session::{Message, Metadata, Schedule, SessionSnapshot};
use serde::Serialize;
use sqlx::prelude::FromRow;

// Row structs bound to SQLite via crudly/sqlx. These live in the infrastructure
// layer only; the core domain types stay free of persistence derives, and
// conversions below bridge the two.

#[derive(Debug, Serialize, FromRow, Clone, crudly::IntoRow, crudly::Schema)]
#[crudly(table = "session_meta")]
pub struct MetadataRow {
    pub session_id: String,
    pub creator: String,
    pub created_at: i64,
    pub archive_at: Option<i64>,
}

impl MetadataRow {
    pub async fn read_from(pool: &sqlx::SqlitePool) -> Result<Self, sqlx::Error> {
        use crudly::SelectAllNoId;
        let all: Vec<Self> = Self::select_all(pool).await?;
        if all.len() != 1 {
            return Err(sqlx::Error::Protocol(
                format!("session_meta must contain exactly 1 row, found {}", all.len()),
            ));
        }
        Ok(all.into_iter().next().unwrap())
    }
}

impl crudly::CrudlyDefault for MetadataRow {}

impl From<MetadataRow> for Metadata {
    fn from(r: MetadataRow) -> Self {
        Self {
            session_id: r.session_id,
            creator: r.creator,
            created_at: r.created_at,
            archive_at: r.archive_at,
        }
    }
}

#[derive(Debug, FromRow, Clone, crudly::IntoRow, crudly::Schema)]
#[crudly(table = "messages")]
pub struct MessageRow {
    #[crudly(id)]
    pub id: i64,
    pub timestamp: i64,
    pub content: String,
    pub role: String,
    pub tag: Option<String>,
}

impl crudly::CrudlyDefault for MessageRow {}

impl From<Message> for MessageRow {
    fn from(m: Message) -> Self {
        Self {
            id: m.id,
            timestamp: m.timestamp,
            content: m.content,
            role: m.role,
            tag: m.tag,
        }
    }
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            timestamp: r.timestamp,
            content: r.content,
            role: r.role,
            tag: r.tag,
        }
    }
}

#[derive(Debug, FromRow, crudly::Schema)]
#[crudly(table = "schedules")]
pub struct ScheduleRow {
    #[crudly(id)]
    pub id: i64,
    pub message: String,
    pub next_run_at: i64,
    pub interval_seconds: Option<i64>,
    pub status: String,
    pub created_at: i64,
}

impl crudly::CrudlyDefault for ScheduleRow {}

impl From<ScheduleRow> for Schedule {
    fn from(r: ScheduleRow) -> Self {
        Self {
            id: r.id,
            message: r.message,
            next_run_at: r.next_run_at,
            interval_seconds: r.interval_seconds,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

/// Read a full session snapshot from an already-open pool.
pub async fn read_snapshot(pool: &sqlx::SqlitePool) -> anyhow::Result<SessionSnapshot> {
    use crudly::SelectAll;
    let metadata: Metadata = MetadataRow::read_from(pool).await?.into();
    let messages: Vec<Message> = MessageRow::select_all(pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    let schedules: Vec<Schedule> = ScheduleRow::select_all(pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(SessionSnapshot {
        metadata,
        messages,
        schedules,
    })
}
