//! TODO: 该文件需重构，当前仅为测试

use async_trait::async_trait;

use crate::session::{Message, SessionHandler};

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

const PERSONA_FILES: &[&str] = &["solo.md", "memory.md"];

pub struct PersonaHandler;

impl PersonaHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SessionHandler for PersonaHandler {
    async fn handle(&self, session_id: &str, messages: &[Message]) -> anyhow::Result<()> {
        let manager = PersonaManager::get();

        let persona_opt = manager.current_persona.read().await;
        let persona = match persona_opt.as_ref() {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut parts = Vec::new();
        for filename in PERSONA_FILES {
            match persona.read_file(filename).await {
                Ok(content) if !content.is_empty() => {
                    parts.push(format!("# {filename}\n{content}"));
                }
                _ => {}
            }
        }

        let system = parts.join("\n\n");
        let response = manager.llm.chat(&system, messages).await?;
        tracing::info!("[PersonaHandler] session={session_id} response={response}");
        Ok(())
    }
}