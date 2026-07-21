use anyhow::Result;
use async_trait::async_trait;
use nota_core::persona::LlmClient;
use nota_core::session::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

/// OpenAI-compatible chat-completion client.
pub struct OpenAiLlm {
    api_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiLlm {
    pub fn new(api_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiLlm {
    async fn chat(&self, system: &str, messages: &[Message]) -> Result<String> {
        let mut chat_messages: Vec<ChatMessage> = Vec::new();

        if !system.is_empty() {
            chat_messages.push(ChatMessage {
                role: "system",
                content: system,
            });
        }

        for msg in messages {
            chat_messages.push(ChatMessage {
                role: &msg.role,
                content: &msg.content,
            });
        }

        let req = ChatRequest {
            model: &self.model,
            messages: chat_messages,
        };

        let url = format!("{}/chat/completions", self.api_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error ({}): {}", status, body);
        }

        let chat_resp: ChatResponse = resp.json().await?;
        let content = chat_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }
}
