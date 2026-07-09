//! TODO: 该文件需重构，当前仅为测试

use std::sync::OnceLock;

use anyhow::Result;
use tokio::sync::RwLock;

use super::handler::{LlmClient, StubLlm};
use super::Persona;

static PM: OnceLock<PersonaManager> = OnceLock::new();

pub struct PersonaManager {
    pub llm: Box<dyn LlmClient>,
    pub current_persona: RwLock<Option<Persona>>,
}

impl PersonaManager {
    pub fn get() -> &'static PersonaManager {
        PM.get().unwrap()
    }
}

pub async fn init() -> Result<()> {
    let manager = PersonaManager {
        llm: Box::new(StubLlm),
        current_persona: RwLock::new(None),
    };
    PM.set(manager)
        .map_err(|_| anyhow::anyhow!("PersonaManager already initialized"))?;
    tracing::info!("PersonaManager initialized");
    Ok(())
}