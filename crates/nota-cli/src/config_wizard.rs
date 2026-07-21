use anyhow::Result;
use dialoguer::{Confirm, Input, Password, Select};

use nota_infra::{Config, provider_default_model, provider_ids, provider_name, provider_url};

/// Run the interactive config wizard. If `existing` is provided, its values
/// are used as defaults so the user can edit an existing configuration.
pub fn run_wizard(existing: Option<&Config>) -> Result<Config> {
    println!("==== Nota Configuration Wizard ====\n");

    let builtin_ids = provider_ids();
    let mut menu_items: Vec<String> = builtin_ids
        .iter()
        .map(|id| provider_name(id).unwrap_or(id).to_string())
        .collect();
    menu_items.push("Custom".to_string());

    let default_idx = existing
        .and_then(|cfg| {
            builtin_ids
                .iter()
                .position(|&id| provider_url(id) == Some(&cfg.api_url))
        })
        .unwrap_or(menu_items.len() - 1);

    let selection = Select::new()
        .with_prompt("API Provider")
        .items(&menu_items)
        .default(default_idx)
        .interact()?;

    let (api_url, existing_key) = if selection < builtin_ids.len() {
        let id = builtin_ids[selection];
        let name = provider_name(id).unwrap_or(id);
        let url = provider_url(id).unwrap_or("").to_string();
        let prompt = format!("{} API Key", name);
        let existing_key = existing
            .filter(|cfg| cfg.api_url == url)
            .map(|cfg| cfg.api_key.clone());
        let api_key = prompt_for_key(&prompt, existing_key)?;
        (url, Some(api_key))
    } else {
        let default_url = existing
            .map(|cfg| cfg.api_url.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let existing_key = existing.map(|cfg| cfg.api_key.clone());
        let api_url: String = Input::new()
            .with_prompt("API Base URL")
            .default(default_url)
            .interact_text()?;
        let api_key = prompt_for_key("API Key", existing_key)?;
        (api_url, Some(api_key))
    };

    // api_key 已经在上面通过 prompt_for_key 获取到了
    let api_key = existing_key.unwrap();

    let default_model = existing
        .map(|cfg| cfg.model.clone())
        .or_else(|| {
            if selection < builtin_ids.len() {
                provider_default_model(builtin_ids[selection]).map(|s| s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "gpt-4o".to_string());

    let model: String = Input::new()
        .with_prompt("Model")
        .default(default_model)
        .interact_text()?;

    let cfg = Config {
        api_url,
        api_key,
        model,
    };

    // 展示最终配置，让用户检查
    println!();
    println!("══════════════════════════════════════");
    println!("  Configuration Summary");
    println!("══════════════════════════════════════");
    println!("  API URL : {}", cfg.api_url);
    println!("  API Key : {}", mask_key(&cfg.api_key));
    println!("  Model   : {}", cfg.model);
    println!("══════════════════════════════════════");
    println!();

    let save_confirm = Confirm::new()
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()?;

    if !save_confirm {
        anyhow::bail!("Configuration cancelled by user");
    }

    Ok(cfg)
}

fn prompt_for_key(prompt: &str, existing: Option<String>) -> Result<String> {
    if let Some(key) = existing
        && !key.is_empty()
    {
        let masked = mask_key(&key);
        let display = format!("{prompt} [current: {masked}]");
        let input: String = Password::new()
            .with_prompt(&display)
            .allow_empty_password(true)
            .interact()?;
        if input.is_empty() {
            Ok(key)
        } else {
            Ok(input)
        }
    } else {
        let input: String = Password::new()
            .with_prompt(prompt)
            .allow_empty_password(false)
            .interact()?;
        Ok(input)
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}****{}", &key[..4], &key[key.len() - 4..])
    }
}
