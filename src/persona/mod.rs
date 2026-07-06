use std::path::PathBuf;

use anyhow::Result;

use crate::base_dir;

pub mod handler;
pub mod manager;

const SOLO_FILENAME: &str = "solo.md";
const MEMORY_FILENAME: &str = "memory.md";

pub struct Persona {
    pub name: String,
    pub workspace_path: PathBuf,
}

// TODO: 纠正处理逻辑，现在这样太慢了，文件你可以缓存啊，没动过就不要再读。而且，不止这两个文件，，算了，以后再改。
impl Persona {
    fn solo_path(&self) -> PathBuf {
        self.workspace_path.join(SOLO_FILENAME)
    }

    fn memory_path(&self) -> PathBuf {
        self.workspace_path.join(MEMORY_FILENAME)
    }

    pub async fn read_solo(&self) -> Result<String> {
        let content = tokio::fs::read_to_string(self.solo_path()).await?;
        Ok(content)
    }

    pub async fn read_memory(&self) -> Result<String> {
        let content = tokio::fs::read_to_string(self.memory_path()).await?;
        Ok(content)
    }

    // 又是default
    pub async fn create_default(name: &str) -> Result<Self> {
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