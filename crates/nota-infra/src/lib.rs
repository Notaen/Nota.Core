//! Infrastructure adapters for Nota.
//!
//! Implements the ports declared in `nota-core` against concrete technologies
//! (SQLite + crudly, axum, the filesystem, a stub LLM). The CLI wires these
//! together and injects them into the core.

pub mod config;
pub mod http;
pub mod llm;
pub mod persona_store;
pub mod tool;

pub use config::{
    Config, ConfigStore,
    provider_default_model, provider_ids, provider_name, provider_url,
};
pub use http::{router as http_router, serve as http_serve, chat::run_dispatcher};
pub use llm::OpenAiLlm;
pub use persona_store::FilePersonaStore;
pub use tool::{ToolRegistryImpl, builtin::register_builtin_tools};
