use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use anyhow::Result;
use async_trait::async_trait;
use nota_core::persona::PersonaStore;
use tokio::sync::RwLock;

const SOLO_FILENAME: &str = "solo.md";
const MEMORY_FILENAME: &str = "memory.md";

type FileCache = HashMap<PathBuf, (String, SystemTime)>;

static PERSONA_FILE_CACHE: OnceLock<RwLock<FileCache>> = OnceLock::new();

fn ensure_cache() -> &'static RwLock<FileCache> {
    PERSONA_FILE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Filesystem-backed [`PersonaStore`].
///
/// Persona workspaces live under `<base_dir>/personas/<name>/`. File contents
/// are cached keyed by path with mtime invalidation (write-through on read).
pub struct FilePersonaStore {
    personas_dir: PathBuf,
}

impl FilePersonaStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            personas_dir: base_dir.join("personas"),
        }
    }

    fn workspace(&self, name: &str) -> PathBuf {
        self.personas_dir.join(name)
    }
}

#[async_trait]
impl PersonaStore for FilePersonaStore {
    async fn read_persona_file(&self, name: &str, filename: &str) -> Result<String> {
        let path = self.workspace(name).join(filename);
        let mtime = tokio::fs::metadata(&path).await?.modified()?;

        {
            let cache = ensure_cache().read().await;
            if let Some((content, cached_mtime)) = cache.get(&path)
                && *cached_mtime == mtime
            {
                return Ok(content.clone());
            }
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut cache = ensure_cache().write().await;
        cache.insert(path, (content.clone(), mtime));
        Ok(content)
    }

    async fn create_persona(&self, name: &str) -> Result<()> {
        let workspace = self.workspace(name);
        tokio::fs::create_dir_all(&workspace).await?;

        let solo_path = workspace.join(SOLO_FILENAME);
        if !tokio::fs::try_exists(&solo_path).await.unwrap_or(false) {
            tokio::fs::write(&solo_path, include_str!("../../assets/solo.md")).await?;
        }

        let memory_path = workspace.join(MEMORY_FILENAME);
        if !tokio::fs::try_exists(&memory_path).await.unwrap_or(false) {
            tokio::fs::write(&memory_path, "").await?;
        }

        Ok(())
    }
}
