use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tokio::sync::RwLock;

use nota_core::session::{Message, Metadata, SessionRepository, SessionSnapshot};

use crate::sqlite::row::{MetadataRow, MessageRow, read_snapshot};

mod row;

/// SQLite-backed [`SessionRepository`].
///
/// Each session owns a dedicated `SqlitePool` (max_connections = 1). The repo
/// resolves all paths under the injected `base_dir` (`<base>/sessions` and
/// `<base>/sessions/archive`), keeping path knowledge out of the core.
pub struct SqliteSessionRepository {
    base_dir: PathBuf,
    pools: RwLock<HashMap<String, SqlitePool>>,
}

impl SqliteSessionRepository {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            pools: RwLock::new(HashMap::new()),
        }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.base_dir.join("sessions")
    }

    fn archive_dir(&self) -> PathBuf {
        self.sessions_dir().join("archive")
    }

    fn db_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{}.sqlite", session_id))
    }

    async fn open_pool(&self, path: &Path, create: bool) -> Result<SqlitePool> {
        let db_url = format!("sqlite://{}", path.to_string_lossy());
        let mut options = SqliteConnectOptions::from_str(db_url.as_str())?;
        if create {
            options = options.create_if_missing(true);
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(pool)
    }

    /// Discover every `.sqlite` file directly under the sessions dir (skipping
    /// the `archive` subdirectory) and return their paths.
    fn find_sqlites(&self) -> Vec<PathBuf> {
        let mut sqlites: Vec<PathBuf> = Vec::new();

        for entry in walkdir::WalkDir::new(self.sessions_dir())
            .into_iter()
            .flatten()
        {
            let path = entry.path();

            // 仅处理一级目录（depth=1），匹配排除名单则跳过整个目录分支
            if entry.depth() == 1
                && path.is_dir()
                && let Some(dir_name) = path.file_name().and_then(|s| s.to_str())
                && dir_name == "archive"
            {
                continue;
            }

            // 筛选后缀为 sqlite 的文件
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sqlite") {
                sqlites.push(path.to_path_buf());
            }
        }
        sqlites
    }

    async fn pool_of(&self, session_id: &str) -> Result<SqlitePool> {
        let pools = self.pools.read().await;
        pools
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session pool not found: {}", session_id))
    }
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn create_session(&self, creator: String) -> Result<(String, Metadata)> {
        let sid = uuid::Uuid::new_v4().to_string();
        let db_path = self.db_path(&sid);
        if db_path.exists() {
            anyhow::bail!("Session database already exists: {}", db_path.display());
        }

        let pool = self.open_pool(&db_path, true).await?;

        // 5. 运行迁移（建表）
        sqlx::migrate!("./assets/migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        // 6. 构建初始 metadata
        let metadata = Metadata {
            session_id: sid.clone(),
            creator,
            created_at: Utc::now().timestamp(),
            archive_at: None,
        };

        // 7. 插入 metadata 到数据库
        let row = MetadataRow {
            session_id: metadata.session_id.clone(),
            creator: metadata.creator.clone(),
            created_at: metadata.created_at,
            archive_at: metadata.archive_at,
        };
        use crudly::InsertNoId;
        row.insert(&pool).await?;

        self.pools.write().await.insert(sid.clone(), pool);

        Ok((sid, metadata))
    }

    async fn load_all(&self) -> Result<Vec<SessionSnapshot>> {
        let paths = self.find_sqlites();

        // 1. 收集所有异步加载Future
        let load_futures = paths.into_iter().map(|path| {
            let this = self;
            async move {
                let pool = this.open_pool(&path, false).await?;
                let snap = read_snapshot(&pool).await?;
                Ok::<_, anyhow::Error>((snap.metadata.session_id.clone(), pool, snap))
            }
        });

        // 2. 并发等待所有加载完成
        let results = futures_util::future::join_all(load_futures).await;

        // 3. 遍历结果，遇到错误直接向上返回；同时登记 pool
        let mut snapshots = Vec::new();
        let mut pools = self.pools.write().await;
        for res in results {
            let (sid, pool, snap) = res?;
            pools.insert(sid, pool);
            snapshots.push(snap);
        }

        log::info!("Loaded {} session(s) from disk", snapshots.len());
        Ok(snapshots)
    }

    async fn insert_message(&self, session_id: &str, msg: Message) -> Result<i64> {
        let pool = self.pool_of(session_id).await?;
        use crudly::InsertWithoutId;
        let row: MessageRow = msg.into();
        let id = row.insert(&pool).await?;
        Ok(id)
    }

    async fn set_archive_at(&self, session_id: &str, at: Option<i64>) -> Result<()> {
        let pool = self.pool_of(session_id).await?;
        // crudly has no ergonomic update for the id-less singleton row, so use
        // a plain UPDATE against session_meta.
        sqlx::query("UPDATE session_meta SET archive_at = ?")
            .bind(at)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn archive_session(&self, session_id: &str) -> Result<()> {
        let pool = self.pools
            .write()
            .await
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found in memory: {}", session_id))?;
        pool.close().await;

        let src = self.db_path(session_id);
        let dst = self.archive_dir().join(format!("{}.sqlite", session_id));
        tokio::fs::rename(src, &dst).await?;
        Ok(())
    }
}
