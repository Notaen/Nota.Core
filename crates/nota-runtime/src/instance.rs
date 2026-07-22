use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context as _, Result};
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::op2;

use crate::types::PluginManifest;

#[derive(Debug, Clone)]
pub struct RegisteredToolMeta {
    pub name: String,
    pub description: String,
    pub parameters: String,
}

#[op2(fast)]
fn op_register_tool(
    state: &mut OpState,
    #[string] name: String,
    #[string] description: String,
    #[string] parameters: String,
) {
    let tools: &mut Vec<RegisteredToolMeta> = state.borrow_mut();
    tools.push(RegisteredToolMeta {
        name,
        description,
        parameters,
    });
}

pub struct PluginInstance {
    pub manifest: PluginManifest,
    _runtime: JsRuntime,
    tools: Vec<RegisteredToolMeta>,
}

impl PluginInstance {
    pub async fn load(manifest: PluginManifest, entry_path: &std::path::Path) -> Result<Self> {
        let entry_code = tokio::fs::read_to_string(entry_path).await?;
        Self::load_from_memory(manifest, &entry_code)
    }

    pub fn load_from_memory(manifest: PluginManifest, entry_code: &str) -> Result<Self> {
        let tools: Rc<RefCell<Vec<RegisteredToolMeta>>> = Rc::new(RefCell::new(Vec::new()));

        let op_decl = op_register_tool();
        let ext = deno_core::Extension {
            name: "nota_plugin",
            ops: std::borrow::Cow::Owned(vec![op_decl]),
            ..Default::default()
        };

        let mut runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![ext],
            ..Default::default()
        });

        {
            let op_state = runtime.op_state();
            op_state.borrow_mut().put(tools.clone());
        }

        let bootstrap = format!(
            r#"
const ctx = {{
    tool: {{
        register(opts) {{
            Deno.core.ops.op_register_tool(
                opts.name,
                opts.description,
                JSON.stringify(opts.parameters || {{}})
            );
        }}
    }}
}};
{entry_code}
"#,
        );

        runtime
            .execute_script("bootstrap.js", bootstrap)
            .context("plugin execution failed")?;

        let registered_tools = tools.borrow().clone();

        Ok(Self {
            manifest,
            _runtime: runtime,
            tools: registered_tools,
        })
    }

    pub fn list_tools(&self) -> Vec<RegisteredToolMeta> {
        self.tools.clone()
    }
}
