use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, Serialize, FromRow, Clone, crudly::IntoRow, crudly::Schema)]
#[crudly(table = "session_meta")]
pub struct Metadata {
    pub session_id: String,
    pub creator: String,
    pub created_at: i64,
    pub archive_at: Option<i64>,
}

impl Metadata {
    pub async fn read_from(pool: &sqlx::SqlitePool) -> Result<Self, sqlx::Error> {
        use crudly::SelectAllNoId;
        let all: Vec<Self> = Self::select_all(pool).await?;
        if all.len() != 1 {
            return Err(sqlx::Error::Protocol(
                format!("session_meta must contain exactly 1 row, found {}", all.len()).into(),
            ));
        }
        Ok(all.into_iter().next().unwrap())
    }
}

impl crudly::CrudlyDefault for Metadata {} 

#[derive(Debug, FromRow, Clone, crudly::IntoRow, crudly::Schema)]
pub struct Message {
    #[crudly(id)]
    pub id: i64,
    pub timestamp: i64,
    pub content: String,
    pub role: String,
    pub tag: Option<String>,
}

impl crudly::CrudlyDefault for Message {}

#[derive(Debug, FromRow, crudly::Schema)]
pub struct Schedule {
    #[crudly(id)]
    pub id: i64,
    pub message: String,
    pub next_run_at: i64,
    pub interval_seconds: Option<i64>,
    pub status: String,
    pub created_at: i64,
}

impl crudly::CrudlyDefault for Schedule {}
