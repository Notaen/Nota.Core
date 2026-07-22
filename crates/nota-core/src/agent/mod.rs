use std::sync::Arc;

use anyhow::Result;

use crate::llm::{ChatMessage, LlmClient, ToolCall, ToolDef};
use crate::tool::{ToolContext, ToolRegistry};

const MAX_ITERATIONS: usize = 16;

pub struct AgentRunner {
    llm: Arc<dyn LlmClient>,
    registry: Arc<dyn ToolRegistry>,
}

impl AgentRunner {
    pub fn new(llm: Arc<dyn LlmClient>, registry: Arc<dyn ToolRegistry>) -> Self {
        Self { llm, registry }
    }

    pub async fn run(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tool_ctx: ToolContext,
    ) -> Result<Vec<ChatMessage>> {
        let mut conversation: Vec<ChatMessage> = messages.to_vec();
        let mut new_messages: Vec<ChatMessage> = Vec::new();
        let tool_defs = self.build_tool_defs();

        for _iteration in 0..MAX_ITERATIONS {
            let response = self
                .llm
                .chat(system, &conversation, &tool_defs)
                .await?;

            if !response.tool_calls.is_empty() {
                let tc_msg = ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(response.tool_calls.clone()),
                    tool_call_id: None,
                };
                conversation.push(tc_msg.clone());
                new_messages.push(tc_msg);

                for tc in &response.tool_calls {
                    match self.execute_tool(tc, &tool_ctx).await {
                        Ok(result) => {
                            let tr_msg = ChatMessage {
                                role: "tool".to_string(),
                                content: Some(result),
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                            };
                            conversation.push(tr_msg.clone());
                            new_messages.push(tr_msg);
                        }
                        Err(e) => {
                            let err_msg = ChatMessage {
                                role: "tool".to_string(),
                                content: Some(format!("tool error: {e}")),
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                            };
                            conversation.push(err_msg.clone());
                            new_messages.push(err_msg);
                        }
                    }
                }
                continue;
            }

            if let Some(content) = response.content {
                let assistant_msg = ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: None,
                };
                new_messages.push(assistant_msg);
                return Ok(new_messages);
            }

            return Ok(new_messages);
        }

        anyhow::bail!("agent loop exceeded max iterations ({MAX_ITERATIONS})");
    }

    fn build_tool_defs(&self) -> Vec<ToolDef> {
        self.registry
            .list()
            .iter()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }

    async fn execute_tool(
        &self,
        tc: &ToolCall,
        ctx: &ToolContext,
    ) -> Result<String> {
        let tool = self
            .registry
            .get(&tc.name)
            .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", tc.name))?;
        tool.run(&tc.arguments, ctx.clone()).await
    }
}
