use async_trait::async_trait;

// 这引用太乱了，模块之前耦合太强了
use crate::session::db::Message;
use crate::session::SessionHandler;

use super::manager::PersonaManager;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, system: &str, messages: &[Message]) -> anyhow::Result<String>;
}

pub struct StubLlm;

#[async_trait]
impl LlmClient for StubLlm {
    async fn chat(&self, _system: &str, _messages: &[Message]) -> anyhow::Result<String> {
        Ok("[Stub LLM response]".to_string())
    }
}

pub struct PersonaHandler;

impl PersonaHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SessionHandler for PersonaHandler {
    async fn handle(&self, session_id: &str, messages: &[Message]) -> anyhow::Result<()> {
        let persona = match PersonaManager::get_default() {
            Some(p) => p,
            None => return Ok(()),
        };

        // 不止这两个文件哦
        let solo = persona.read_solo().await?;
        let memory = persona.read_memory().await?;

        let system = if memory.is_empty() {
            solo
        } else {
            format!("{}\n\n# Memory\n{}", solo, memory)
        };

        let llm = StubLlm;
        let response = llm.chat(&system, messages).await?;
        tracing::info!("[PersonaHandler] session={session_id} response={response}");
        Ok(())
    }
}