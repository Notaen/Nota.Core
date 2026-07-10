//! Infrastructure adapters for Nota.
//!
//! Implements the ports declared in `nota-core` against concrete technologies
//! (SQLite + crudly, axum, the filesystem, a stub LLM). The CLI wires these
//! together and injects them into the core.

pub mod config;
pub mod http;
pub mod llm;
pub mod persona_store;
pub mod sqlite;

pub use config::{Config, ConfigStore};
pub use http::{router as http_router, serve as http_serve};
pub use llm::StubLlm;
pub use persona_store::FilePersonaStore;
pub use sqlite::SqliteSessionRepository;
