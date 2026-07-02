use std::{fs, sync::RwLock};

use anyhow::{Ok, Result, anyhow};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{base_dir, config::guide::interactive_config_init};

mod guide;

static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
}

pub fn load() -> Result<()> {
    let config_file = base_dir().join("config.toml");

    let config: Config = if !fs::exists(&config_file)? {
        warn!("The config.toml doesn't exist");
        interactive_config_init()?
    } else {
        let config_str = fs::read_to_string(config_file)?;
        toml::from_str(&config_str)?
    };

    let mut guard = CONFIG.write().unwrap();
    *guard = Some(config);
    
    tracing::info!("Config loaded");
    Ok(())
}

pub fn save() -> Result<()> {
    let guard = CONFIG.read().unwrap();
    let cfg = guard
        .as_ref()
        .ok_or(anyhow!("Should config::load() first"))?;
    _save(cfg)
}

pub(super) fn _save(cfg: &Config) -> Result<()> {
    let config_str = toml::to_string_pretty(cfg)?;

    let config_file = base_dir().join("config.toml");
    fs::write(config_file, config_str)?;
    Ok(())
}
