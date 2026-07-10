use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// Application configuration persisted as `config.toml` under the base dir.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
}

/// Owns the loaded [`Config`] and the path it is read from / written to.
pub struct ConfigStore {
    config_file: PathBuf,
    inner: RwLock<Option<Config>>,
}

impl ConfigStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            config_file: base_dir.join("config.toml"),
            inner: RwLock::new(None),
        }
    }

    pub fn load(&self) -> Result<()> {
        let config: Config = if !std::fs::exists(&self.config_file)? {
            return Err(anyhow!("config.toml not found at {}", self.config_file.display()));
        } else {
            let config_str = std::fs::read_to_string(&self.config_file)?;
            toml::from_str(&config_str)?
        };

        let mut guard = self.inner.write().unwrap();
        *guard = Some(config);
        log::info!("Config loaded");
        Ok(())
    }

    pub fn save(&self, cfg: &Config) -> Result<()> {
        let config_str = toml::to_string_pretty(cfg)?;
        std::fs::write(&self.config_file, config_str)?;
        let mut guard = self.inner.write().unwrap();
        *guard = Some(cfg.clone());
        Ok(())
    }

    /// Take ownership of a config built by an external wizard (e.g. the CLI's
    /// interactive prompt) without first reading it from disk.
    pub fn set(&self, cfg: Config) {
        let mut guard = self.inner.write().unwrap();
        *guard = Some(cfg);
    }
}
