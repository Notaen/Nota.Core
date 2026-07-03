use std::{collections::HashMap, path::PathBuf, sync::RwLock};

use anyhow::Ok;
use chrono::{DateTime, Utc};
use sqlx::Connection;
use tokio::fs;

use crate::{base_dir, session::Session, session::db::Metadata};

static SM: RwLock<Option<SessionManager>> = RwLock::new(None);

pub struct SessionManager {
    pub session_map: HashMap<String, Session>,
}

impl SessionManager {
    pub async fn new_session(creator: String) -> anyhow::Result<String> {
        let sid = uuid::Uuid::new_v4().to_string();

        let res: anyhow::Result<String> = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");

            rt.block_on(async move {
                let new_session = Session::new(&sid, &creator).await?;
                let mut guard = SM.write().unwrap();
                guard
                    .as_mut()
                    .unwrap()
                    .session_map
                    .insert(sid.clone(), new_session);
                Ok(sid)
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

        Ok(res?)
    }

    async fn archive_session(session_id: &str, session: Session) -> anyhow::Result<()> {
        session.conn.close().await?;

        let src = base_dir()
            .join("sessions")
            .join(format!("{}.sqlite", session_id));
        let dst_dir = base_dir().join("sessions").join("archive");
        fs::create_dir_all(&dst_dir).await?;
        let dst = dst_dir.join(format!("{}.sqlite", session_id));
        fs::rename(&src, &dst).await?;

        tracing::info!("Archived session {session_id}");
        Ok(())
    }

    pub async fn archive_expired_sessions() {
        let expired_ids = {
            let guard = SM.read().unwrap();
            let manager = guard.as_ref().unwrap();

            let now = Utc::now();
            manager
                .session_map
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
            let session = {
                let mut guard = SM.write().unwrap();
                let manager = guard.as_mut().unwrap();
                manager.session_map.remove(session_id)
            };

            if let Some(session) = session {
                if let Err(e) = Self::archive_session(session_id, session).await {
                    tracing::error!("Failed to archive session {session_id}: {e:?}");
                }
            }
        }
    }

    pub fn list_metadata() -> Vec<Metadata> {
        let guard = SM.read().unwrap();
        let Some(manager) = guard.as_ref() else {
            return Vec::new();
        };

        manager
            .session_map
            .values()
            .map(|s| Metadata {
                session_id: s.metadata.session_id.clone(),
                creator: s.metadata.creator.clone(),
                created_at: s.metadata.created_at,
                archive_at: s.metadata.archive_at,
            })
            .collect()
    }

    pub fn get_archive_at(session_id: &str) -> Option<Option<DateTime<Utc>>> {
        let guard = SM.read().unwrap();
        let Some(manager) = guard.as_ref() else {
            return None;
        };
        manager
            .session_map
            .get(session_id)
            .map(|s| s.metadata.archive_at)
    }

    pub async fn set_archive_at(
        session_id: &str,
        archive_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        let session_id = session_id.to_string();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");

            rt.block_on(async move {
                let mut guard = SM.write().unwrap();
                let Some(manager) = guard.as_mut() else {
                    anyhow::bail!("SessionManager not initialized");
                };
                let Some(session) = manager.session_map.get_mut(&session_id) else {
                    anyhow::bail!("Session not found: {session_id}");
                };

                session.metadata.archive_at = archive_at;

                let ts = archive_at.map(|dt| dt.timestamp());
                sqlx::query("UPDATE session_meta SET archive_at = ? WHERE session_id = ?")
                    .bind::<Option<i64>>(ts)
                    .bind(&session_id)
                    .execute(&mut session.conn)
                    .await?;

                Ok(())
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?
    }
}

pub async fn load() -> anyhow::Result<()> {
    let sqlite_paths = find_sqlites();

    // 1. 收集所有异步加载Future
    let load_futures = sqlite_paths
        .into_iter()
        .map(async move |path| Session::load(&path).await);

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

    let mut guard = SM.write().unwrap();

    *guard = Some(SessionManager { session_map });
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
