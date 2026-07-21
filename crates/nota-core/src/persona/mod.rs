use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::session::{Message, SessionHandler, SessionManager};

const SOLO_FILENAME: &str = "solo.md";
const MEMORY_FILENAME: &str = "memory.md";
const PERSONA_FILES: &[&str] = &[SOLO_FILENAME, MEMORY_FILENAME];

/// A persona is, at the domain level, just a name. Where its files live is an
/// infrastructure concern handled by [`PersonaStore`].
#[derive(Debug, Clone)]
pub struct Persona {
    pub name: String,
}

/// Port for reading/writing persona workspace files.
#[async_trait]
pub trait PersonaStore: Send + Sync {
    /// Read a file (`solo.md`, `memory.md`, ...) belonging to a persona.
    async fn read_persona_file(&self, name: &str, filename: &str) -> Result<String>;

    /// Create a persona workspace (idempotent: seeds default files when absent).
    async fn create_persona(&self, name: &str) -> Result<()>;
}

/// Port for chat completion. Infrastructure supplies real/stub clients.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, system: &str, messages: &[Message]) -> Result<String>;
}

/// Owns the active persona and reacts to session messages by building a system
/// prompt from the persona's files and delegating to an [`LlmClient`].
///
/// Replaces the old split `PersonaManager` + `PersonaHandler`; the handler role
/// is now filled by implementing [`SessionHandler`] directly.
pub struct PersonaManager {
    store: Arc<dyn PersonaStore>,
    llm: Arc<dyn LlmClient>,
    sessions: Arc<SessionManager>,
    current_persona: RwLock<Option<Persona>>,
}

impl PersonaManager {
    pub fn new(
        store: Arc<dyn PersonaStore>,
        llm: Arc<dyn LlmClient>,
        sessions: Arc<SessionManager>,
    ) -> Self {
        Self {
            store,
            llm,
            sessions,
            current_persona: RwLock::new(None),
        }
    }

    pub async fn set_persona(&self, persona: Option<Persona>) {
        *self.current_persona.write().await = persona;
    }

    pub async fn current_persona_name(&self) -> Option<String> {
        self.current_persona.read().await.as_ref().map(|p| p.name.clone())
    }
}

#[async_trait]
impl SessionHandler for PersonaManager {
    async fn handle(&self, session_id: &str, messages: &[Message]) -> Result<()> {
        // 仅响应用户消息，避免对 assistant 回复再次触发 LLM 导致递归
        let last_role = messages.last().map(|m| m.role.as_str());
        if last_role != Some("user") {
            return Ok(());
        }

        let persona_name = match self.current_persona_name().await {
            Some(n) => n,
            None => return Ok(()),
        };

        let mut parts = Vec::new();
        for filename in PERSONA_FILES {
            match self.store.read_persona_file(&persona_name, filename).await {
                Ok(content) if !content.is_empty() => {
                    parts.push(format!("# {filename}\n{content}"));
                }
                _ => {}
            }
        }

        let system = parts.join("\n\n");

        // 把除最后一条（刚插入的用户消息）以外的消息作为上下文传给 LLM，
        // 因为 messages 已经包含了刚入库的用户消息。
        let response = self.llm.chat(&system, messages).await?;

        let now = chrono::Utc::now().timestamp();
        let assistant_msg = Message {
            id: 0, // DB 会分配真正的 id
            timestamp: now,
            content: response,
            role: "assistant".to_string(),
            tag: None,
        };

        self.sessions
            .insert_message(session_id, assistant_msg)
            .await?;

        Ok(())
    }
}
