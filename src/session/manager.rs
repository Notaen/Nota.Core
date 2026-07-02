use std::{collections::HashMap, path::PathBuf, sync::RwLock};

use anyhow::Ok;

use crate::{base_dir, session::Session};

static SM: RwLock<Option<SessionManager>> = RwLock::new(None);

pub struct SessionManager {
    session_map: HashMap<String, Session>,
}

impl SessionManager {
    pub async fn new_session(creator: &str) -> anyhow::Result<String> {
        let session_id = "".to_string();
        let new_session = Session::new(&session_id, creator).await?;
        let mut _guard = SM.write().unwrap();
        _guard.as_mut().unwrap().session_map.insert(session_id.clone(), new_session);
        Ok(session_id)
    }
}

pub async fn load() -> anyhow::Result<()> {
    let sqlite_paths = find_sqlites();

    // 1. 收集所有异步加载Future
    let load_futures = sqlite_paths
        .into_iter()
        .map(|path| async move { Session::load(&path).await });

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
