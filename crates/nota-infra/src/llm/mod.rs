use anyhow::Result;
use async_trait::async_trait;
use nota_core::llm::{ChatMessage, LlmClient, LlmResponse, ToolCall, ToolDef};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Serialize)]
struct WireMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<WireToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: WireToolCallFunction,
}

#[derive(Serialize)]
struct WireToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct ApiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ApiToolFunction,
}

#[derive(Serialize)]
struct ApiToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
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
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatToolCall>,
}

#[derive(Deserialize)]
struct ChatToolCall {
    id: String,
    function: ToolCallFunction,
}

#[derive(Deserialize)]
struct ToolCallFunction {
    name: String,
    arguments: String,
}

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
    async fn chat(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<LlmResponse> {
        let mut chat_messages: Vec<ChatMessage> = Vec::new();

        if !system.is_empty() {
            chat_messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        chat_messages.extend(messages.iter().cloned());

        let api_tools: Vec<ApiTool> = tools
            .iter()
            .map(|t| ApiTool {
                tool_type: "function".to_string(),
                function: ApiToolFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: serde_json::to_value(&t.parameters)
                        .unwrap_or(serde_json::Value::Null),
                },
            })
            .collect();

        let wire_messages: Vec<WireMessage> = chat_messages
            .iter()
            .map(to_wire_message)
            .collect();

        let req = ChatRequest {
            model: self.model.clone(),
            messages: wire_messages,
            tools: if api_tools.is_empty() { None } else { Some(api_tools) },
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
        let choice = chat_resp.choices.into_iter().next();
        let msg = choice.map(|c| c.message);

        let content = msg.as_ref().and_then(|m| m.content.clone());
        let tool_calls = msg
            .map(|m| {
                m.tool_calls
                    .into_iter()
                    .map(|tc| ToolCall {
                        id: tc.id,
                        name: tc.function.name,
                        arguments: tc.function.arguments,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(LlmResponse { content, tool_calls })
    }
}

fn to_wire_message(msg: &ChatMessage) -> WireMessage {
    WireMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
        tool_calls: msg.tool_calls.as_ref().map(|tcs| {
            tcs.iter()
                .map(|tc| WireToolCall {
                    id: tc.id.clone(),
                    tool_type: "function".to_string(),
                    function: WireToolCallFunction {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    },
                })
                .collect()
        }),
        tool_call_id: msg.tool_call_id.clone(),
    }
}
