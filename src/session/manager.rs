use std::{collections::HashMap, path::PathBuf, sync::OnceLock};

use anyhow::Ok;
use chrono::Utc;
use tokio::{fs, sync::RwLock};

use crate::{
    BASE_DIR, base_dir,
    session::{Session, db::Metadata},
};

pub static SM: OnceLock<SessionManager> = OnceLock::new();

#[derive(Debug)]
pub struct SessionManager {
    pub session_map: RwLock<HashMap<String, Session>>,
}

impl SessionManager {
    pub async fn new_session(&self, creator: String) -> anyhow::Result<String> {
        let sid = uuid::Uuid::new_v4().to_string();

        let new_session = Session::new(sid.clone(), creator).await?;
        self.session_map
            .write()
            .await
            .insert(sid.clone(), new_session);
        Ok(sid)
    }

    async fn archive_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut guard = self.session_map.write().await;
        let session = guard
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found in memory: {}", session_id))?;

        let src = session
            .db_url
            .strip_prefix("sqlite://")
            .ok_or_else(|| anyhow::anyhow!("Invalid session db_url: {}", session.db_url))?;
        let dst_dir = BASE_DIR.join("sessions").join("archive");
        let dst = dst_dir.join(format!("{}.sqlite", session_id));
        fs::rename(&src, &dst).await?;

        tracing::info!("Archived session {session_id}");
        Ok(())
    }

    pub async fn archive_expired_sessions(&self) {
        let expired_ids = {
            let session_map = self.session_map.read().await;

            let now = Utc::now();
            session_map
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
            let res = self.archive_session(session_id).await;
            if let Err(e) = res {
                tracing::error!("Failed to archive session {session_id}: {e:?}");
            }
        }
    }

    pub async fn list_metadata(&self) -> Vec<Metadata> {
        let session_map = self.session_map.read().await;

        session_map.values().map(|s| s.metadata.clone()).collect()
    }
}

pub async fn load() -> anyhow::Result<()> {
    let sqlite_paths = find_sqlites();

    // 1. 收集所有异步加载Future
    let load_futures = sqlite_paths
        .into_iter()
        .map(async move |path| Session::load_from(&path).await);

    // 2. 并发等待所有加载完成，得到Vec<Result<Session>>
    let sessions_result = futures_util::future::join_all(load_futures).await;

    // 3. 遍历结果，遇到错误直接向上返回
    let mut sessions = Vec::new();
    for res in sessions_result {
        sessions.push(res?);
    }

    // 4. 构建HashMap，collect不会返回Result，去掉?
    let session_map = sessions
        .into_iter()
        .map(|s| (s.metadata.session_id.clone(), s))
        .collect();

    SM.set(SessionManager {
        session_map: RwLock::new(session_map),
    })
    .unwrap();

    tracing::info!("SessionManager loaded");
    Ok(())
}

fn find_sqlites() -> Vec<PathBuf> {
    let mut sqlites: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(base_dir().join("sessions"))
        .into_iter()
        .flatten()
    {
        let path = entry.path();

        // 仅处理一级目录（depth=1），匹配排除名单则跳过整个目录分支
        if entry.depth() == 1 && path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                if dir_name == "archive" {
                    continue;
                }
            }
        }

        // 筛选后缀为 sqlite 的文件
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sqlite") {
            sqlites.push(path.to_path_buf());
        }
    }
    sqlites
}
