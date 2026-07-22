use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use nota_core::tool::{PropertyDef, Tool, ToolContext, ToolParams, ToolRegistry};

use super::ToolRegistryImpl;

pub struct FileReadTool {
    personas_dir: PathBuf,
}

impl FileReadTool {
    pub fn new(personas_dir: PathBuf) -> Self {
        Self { personas_dir }
    }

    fn workspace(&self, name: &str) -> PathBuf {
        self.personas_dir.join(name)
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read a file within the persona workspace"
    }

    fn parameters(&self) -> ToolParams {
        let mut props = HashMap::new();
        props.insert(
            "path".to_string(),
            PropertyDef {
                prop_type: "string".to_string(),
                description: "Relative path within the persona workspace".to_string(),
                r#enum: vec![],
            },
        );
        ToolParams::object(props, vec!["path".to_string()])
    }

    async fn run(&self, args: &str, ctx: ToolContext) -> Result<String> {
        let args: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
        let rel = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        let workspace = self.workspace(&ctx.persona_name);
        let resolved = workspace.join(rel);

        let workspace_canonical = tokio::fs::canonicalize(&workspace).await?;
        let canonical = match tokio::fs::canonicalize(&resolved).await {
            Ok(c) => c,
            Err(_) => {
                if !resolved.starts_with(&workspace_canonical) {
                    let prompt = format!(
                        "{} wants to read outside its workspace: {}",
                        ctx.persona_name,
                        resolved.display()
                    );
                    let approved = ctx.request_permission(prompt).await;
                    if !approved {
                        anyhow::bail!("permission denied");
                    }
                }
                tokio::fs::canonicalize(&resolved).await?
            }
        };

        if !canonical.starts_with(&workspace_canonical) {
            let prompt = format!(
                "{} wants to read outside its workspace: {}",
                ctx.persona_name,
                resolved.display()
            );
            let approved = ctx.request_permission(prompt).await;
            if !approved {
                anyhow::bail!("permission denied");
            }
        }

        let content = tokio::fs::read_to_string(&canonical).await?;
        Ok(content)
    }
}

pub struct FileWriteTool {
    personas_dir: PathBuf,
}

impl FileWriteTool {
    pub fn new(personas_dir: PathBuf) -> Self {
        Self { personas_dir }
    }

    fn workspace(&self, name: &str) -> PathBuf {
        self.personas_dir.join(name)
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file within the persona workspace. Creates parent directories if needed."
    }

    fn parameters(&self) -> ToolParams {
        let mut props = HashMap::new();
        props.insert(
            "path".to_string(),
            PropertyDef {
                prop_type: "string".to_string(),
                description: "Relative path within the persona workspace".to_string(),
                r#enum: vec![],
            },
        );
        props.insert(
            "content".to_string(),
            PropertyDef {
                prop_type: "string".to_string(),
                description: "Content to write".to_string(),
                r#enum: vec![],
            },
        );
        ToolParams::object(props, vec!["path".to_string(), "content".to_string()])
    }

    async fn run(&self, args: &str, ctx: ToolContext) -> Result<String> {
        let args: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
        let rel = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        let workspace = self.workspace(&ctx.persona_name);
        let resolved = workspace.join(rel);

        let canonical = tokio::fs::canonicalize(&workspace).await?;
        let target = if let Ok(c) = tokio::fs::canonicalize(&resolved).await {
            c
        } else {
            let mut clean = std::path::PathBuf::new();
            for component in resolved.components() {
                clean.push(component);
            }
            clean
        };

        if !target.starts_with(&canonical) && target != canonical {
            let prompt = format!(
                "{} wants to write outside its workspace: {}",
                ctx.persona_name,
                resolved.display()
            );
            let approved = ctx.request_permission(prompt).await;
            if !approved {
                anyhow::bail!("permission denied");
            }
        }

        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&target, content).await?;
        Ok(format!("ok: wrote {} bytes", content.len()))
    }
}

pub struct ScheduleTool;

#[async_trait]
impl Tool for ScheduleTool {
    fn name(&self) -> &str {
        "schedule"
    }

    fn description(&self) -> &str {
        "Schedule a message to be sent in this session at a future time"
    }

    fn parameters(&self) -> ToolParams {
        let mut props = HashMap::new();
        props.insert(
            "message".to_string(),
            PropertyDef {
                prop_type: "string".to_string(),
                description: "Message content to deliver".to_string(),
                r#enum: vec![],
            },
        );
        props.insert(
            "trigger_at".to_string(),
            PropertyDef {
                prop_type: "string".to_string(),
                description: "ISO 8601 datetime when the message should be delivered".to_string(),
                r#enum: vec![],
            },
        );
        ToolParams::object(
            props,
            vec!["message".to_string(), "trigger_at".to_string()],
        )
    }

    async fn run(&self, args: &str, _ctx: ToolContext) -> Result<String> {
        let _args: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
        Ok("schedule accepted (scheduler not yet implemented)".to_string())
    }
}

pub struct GetVersionTool;

#[async_trait]
impl Tool for GetVersionTool {
    fn name(&self) -> &str {
        "get_version"
    }

    fn description(&self) -> &str {
        "Get the current Nota version"
    }

    fn parameters(&self) -> ToolParams {
        ToolParams::object(HashMap::new(), vec![])
    }

    async fn run(&self, _args: &str, _ctx: ToolContext) -> Result<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }
}

pub fn register_builtin_tools(registry: &ToolRegistryImpl, personas_dir: PathBuf) {
    registry.register(Arc::new(FileReadTool::new(personas_dir.clone())));
    registry.register(Arc::new(FileWriteTool::new(personas_dir)));
    registry.register(Arc::new(ScheduleTool));
    registry.register(Arc::new(GetVersionTool));
}
