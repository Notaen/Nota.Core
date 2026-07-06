use std::sync::RwLock;

use anyhow::Result;

use super::handler::{LlmClient, StubLlm};
use super::Persona;

// 你RwLock套Option是何意味呢？嫌代码太短了？？用OnceLock不好吗？
static PM: RwLock<Option<PersonaManager>> = RwLock::new(None);

// 别写死，这项目只是个框架，你咋给人家定义成常量了？？
const DEFAULT_PERSONA: &str = "Nota";

pub struct PersonaManager {
    pub default: Persona,
    pub llm: Box<dyn LlmClient>,
}

impl PersonaManager {
    // 这里也是，default都写我脸上来了！！我再说以一遍，没有任何默认Persona，想要默认，你放到用户一定能看到的地方，经过用户同意，才能创建，哪怕用户只是点了一下回车
    pub fn get_default() -> Option<Persona> {
        let guard = PM.read().ok()?;
        let manager = guard.as_ref()?;
        Some(Persona {
            name: manager.default.name.clone(),
            workspace_path: manager.default.workspace_path.clone(),
        })
    }
}
// 你咋这样写？以后咋做拓展了？
pub async fn init() -> Result<()> {
    let persona = Persona::create_default(DEFAULT_PERSONA).await?;
    let manager = PersonaManager {
        default: persona,
        llm: Box::new(StubLlm),
    };
    *PM.write().unwrap() = Some(manager);
    tracing::info!("Persona initialized: {DEFAULT_PERSONA}");
    Ok(())
}