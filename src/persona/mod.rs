//! TODO: 该文件需重构，当前仅为测试

use std::{collections::HashMap, path::PathBuf, sync::OnceLock, time::SystemTime};

use anyhow::Result;
use tokio::sync::RwLock;

use crate::base_dir;

pub mod handler;
pub(crate) mod manager;
pub use manager::init;

const SOLO_FILENAME: &str = "solo.md";
const MEMORY_FILENAME: &str = "memory.md";

type FileCache = HashMap<PathBuf, (String, SystemTime)>;

static PERSONA_FILE_CACHE: OnceLock<RwLock<FileCache>> = OnceLock::new();

fn ensure_cache() -> &'static RwLock<FileCache> {
    PERSONA_FILE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct Persona {
    pub name: String,
    pub workspace_path: PathBuf,
}

impl Persona {
    pub async fn read_file(&self, filename: &str) -> Result<String> {
        let path = self.workspace_path.join(filename);
        let mtime = tokio::fs::metadata(&path).await?.modified()?;

        {
            let cache = ensure_cache().read().await;
            if let Some((content, cached_mtime)) = cache.get(&path) {
                if *cached_mtime == mtime {
                    return Ok(content.clone());
                }
            }
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut cache = ensure_cache().write().await;
        cache.insert(path, (content.clone(), mtime));
        Ok(content)
    }

    pub async fn read_solo(&self) -> Result<String> {
        self.read_file(SOLO_FILENAME).await
    }

    pub async fn read_memory(&self) -> Result<String> {
        self.read_file(MEMORY_FILENAME).await
    }

    pub async fn create(name: &str) -> Result<Self> {
        let workspace_path = base_dir().join("personas").join(name);
        tokio::fs::create_dir_all(&workspace_path).await?;

        let solo_path = workspace_path.join(SOLO_FILENAME);
        if !tokio::fs::try_exists(&solo_path).await.unwrap_or(false) {
            tokio::fs::write(&solo_path, include_str!("../../assets/solo.md")).await?;
        }

        let memory_path = workspace_path.join(MEMORY_FILENAME);
        if !tokio::fs::try_exists(&memory_path).await.unwrap_or(false) {
            tokio::fs::write(&memory_path, "").await?;
        }

        Ok(Self {
            name: name.to_string(),
            workspace_path,
        })
    }
}
