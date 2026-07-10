use anyhow::Result;
use async_trait::async_trait;
use nota_core::persona::LlmClient;
use nota_core::session::Message;

/// No-op [`LlmClient`] used until a real provider is wired in.
pub struct StubLlm;

#[async_trait]
impl LlmClient for StubLlm {
    async fn chat(&self, _system: &str, _messages: &[Message]) -> Result<String> {
        Ok("[Stub LLM response]".to_string())
    }
}
