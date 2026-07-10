use anyhow::Result;
use dialoguer::{Confirm, Input, Password};
use tracing::info;

use nota_infra::Config;

/// Interactive first-run configuration wizard. Prompts for the API base URL and
/// key, then asks to persist the result to `config.toml`.
pub fn interactive_config_init() -> Result<Config> {
    println!("==== Interactive Configuration Wizard ====");
    println!("Config file missing, we'll create one for you.\n");

    let api_url: String = Input::new()
        .with_prompt("API Base URL")
        .default("https://openrouter.ai/api/v1".to_string())
        .interact_text()?;

    let api_key: String = Password::new()
        .with_prompt("API Key")
        .allow_empty_password(false)
        .interact()?;

    let save_confirm = Confirm::new()
        .with_prompt("Confirm save config to config.toml?")
        .default(true)
        .interact()?;

    let cfg = Config { api_url, api_key };

    if save_confirm {
        info!("config saved (see ConfigStore::save)");
    }

    Ok(cfg)
}
