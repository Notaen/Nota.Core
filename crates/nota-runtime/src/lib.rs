use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use nota_core::tool::ToolRegistry;

mod instance;
mod types;

use instance::PluginInstance;
use types::PluginManifest;

pub struct PluginManager {
    plugins_dir: PathBuf,
    #[allow(dead_code)]
    registry: Arc<dyn ToolRegistry>,
    instances: RwLock<HashMap<String, PluginInstance>>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf, registry: Arc<dyn ToolRegistry>) -> Self {
        Self {
            plugins_dir,
            registry,
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub async fn scan_and_load_all(&self) -> Result<()> {
        if !tokio::fs::try_exists(&self.plugins_dir).await.unwrap_or(false) {
            log::info!("plugins dir not found, skipping");
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&self.plugins_dir)
            .into_iter()
            .flatten()
        {
            if entry.file_name() == "plugin.json" {
                let manifest_path = entry.path().to_path_buf();
                match self.load_from_manifest(&manifest_path).await {
                    Ok(()) => {}
                    Err(e) => {
                        log::warn!("failed to load plugin at {}: {e}", manifest_path.display());
                    }
                }
            }
        }
        Ok(())
    }

    async fn load_from_manifest(&self, manifest_path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;

        let plugin_dir = manifest_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("plugin dir not found"))?;
        let entry_path = plugin_dir.join(&manifest.entry);

        if !tokio::fs::try_exists(&entry_path).await.unwrap_or(false) {
            anyhow::bail!("entry file not found: {}", entry_path.display());
        }

        let instance = PluginInstance::load(manifest.clone(), &entry_path).await?;

        let registered = instance.list_tools();
        log::info!(
            "plugin '{}' loaded ({} tools declared: {:?})",
            manifest.name,
            registered.len(),
            registered.iter().map(|t| &t.name).collect::<Vec<_>>(),
        );

        let name = manifest.name.clone();
        self.instances.write().await.insert(name, instance);
        Ok(())
    }

    pub async fn load_embedded(&self, manifest_json: &str, entry_code: &str) -> Result<()> {
        let manifest: PluginManifest = serde_json::from_str(manifest_json)?;
        let instance = PluginInstance::load_from_memory(manifest.clone(), entry_code)?;

        let registered = instance.list_tools();
        log::info!(
            "embedded plugin '{}' loaded ({} tools declared: {:?})",
            manifest.name,
            registered.len(),
            registered.iter().map(|t| &t.name).collect::<Vec<_>>(),
        );

        self.instances
            .write()
            .await
            .insert(manifest.name.clone(), instance);
        Ok(())
    }

    pub async fn reload(&self, name: &str) -> Result<()> {
        let old = {
            let mut instances = self.instances.write().await;
            instances.remove(name)
        };

        let old = match old {
            Some(inst) => inst,
            None => anyhow::bail!("plugin '{}' not found", name),
        };
        drop(old);

        let manifest_path = self.plugins_dir.join(name).join("plugin.json");
        self.load_from_manifest(&manifest_path).await
    }

    pub async fn list(&self) -> Vec<String> {
        self.instances.read().await.keys().cloned().collect()
    }
}
