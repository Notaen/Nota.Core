# Developer Notes

## Code Modification Rules
- Do NOT delete, modify, or remove existing comments without explicit approval.
- When in doubt about a comment change, ask first.

## Directory Naming
- Use `personas` (plural) for the persona directory under `base_dir()`.
  - `base_dir().join("personas")` not `base_dir().join("persona")`.

## Design Decisions (from Chinese review comments)

### No Default Persona
- `PersonaManager` must NOT auto-create a default persona on init.
- **No hardcoded persona names** (removed `const DEFAULT_PERSONA`).
- The `current_persona` field starts as `None`.
- Persona creation must be explicit — user must opt in (via CLI wizard, config, or API).
- Ref: `src/persona/manager.rs`

### OnceLock over RwLock<Option<T>>
- Use `OnceLock<T>` for singletons that are set once at startup and never unset.
- Do NOT use `RwLock<Option<T>>` — it adds unnecessary complexity and allows invalid states.
- Ref: `src/persona/manager.rs`

### Persona File Caching
- Persona files (`solo.md`, `memory.md`, etc.) are cached in a global `HashMap<PathBuf, (String, SystemTime)>`.
- Cache key: file path. Cache value: (content, mtime).
- On read: check file mtime against cache. If unchanged, return cached content.
- Write-through: after reading from disk, update cache.
- Ref: `src/persona/mod.rs`

### Persona Extensibility
- `Persona::read_file(filename)` is the generic method for any file under the persona workspace.
- `read_solo()` and `read_memory()` are convenience wrappers.
- `PersonaHandler` iterates over `PERSONA_FILES` slice — adding new files just means appending to the list.
- Ref: `src/persona/mod.rs`, `src/persona/handler.rs`

### Reduce Module Coupling
- `session::db` is private (not `pub mod db`); types are re-exported from `session/mod.rs`.
- `persona::handler` imports from `crate::session` instead of `crate::session::db`.
- TODO: A shared types module (`crate::types`) may be needed long-term to fully decouple `persona` and `session`.

### Consolidate Time Dependencies
- Use only `chrono` — removed the `time` crate.
- Custom `ChronoLocalTimer` implements `tracing_subscriber::fmt::time::FormatTime`.
- No more redundant timestamp libraries.

### SQLx Migration Naming
- Files must follow `YYYYMMDDHHMMSS_description.sql` format.
- Fixed: `20260706_init_session_db.sql` → `20260706000000_init_session_db.sql`.

### English/Grammar Cleanup
- Log messages and user-facing strings should be idiomatic English.

## Provider System

### Built-in Providers (DeepSeek, OpenRouter, Custom)
- Provider metadata (URL, default model) lives in `crates/nota-infra/assets/providers.toml`,
  compiled in via `include_str!`. Used ONLY by the config wizard to pre-fill defaults.
- Saved `config.toml` is flat: `api_url`, `api_key`, `model` — no provider type distinction at runtime.
- The wizard (`config_wizard::run_wizard`) accepts an existing `Config` as defaults for editing.
  Final config is displayed as a summary before saving.

### `nota onboard` command
- Uses `clap` derive. Runs the wizard standalone (no server start).
- `nota` with no subcommand starts the server normally (auto-wizard if config missing).

## Tool System (nota-core)

### Domain types over generics
- `ToolParams` + `PropertyDef` structs model JSON Schema directly, NOT `serde_json::Value`
  or raw `String`. These are domain types with clear semantics, not serialization helpers.
- `ToolParams::object(properties, required)` is the canonical constructor.
- Serialization to actual JSON happens only in `nota-infra` (via `serde_json::to_value`).
- This was the result of multiple review rounds:
  1. First tried `serde_json::Value` (wrong — serialization lib in core)
  2. Then tried `String` (wrong — lost type safety, unreadable)
  3. Then tried custom `JsonValue` enum (wrong — still a generic container, not domain-specific)
  4. Finally: `ToolParams` + `PropertyDef` (correct — models the domain)

### AgentRunner
- Tool calling loop: max 16 iterations, LLM → tool_calls → execute → append results → repeat.
- `ToolDef` + `ToolCall` + `LlmResponse` types in `nota-core::llm`.
- Tool calls/results stored as messages (`role: "tool_call"` / `"tool_result"`).
- The runner returns all new messages; caller is responsible for persistence.

### Built-in tools (nota-infra)
- `file_read`, `file_write` — sandboxed to persona workspace (`canonicalize` path checks).
- `schedule` — stub implementation (scheduler not yet built).
- `get_version` — returns `env!("CARGO_PKG_VERSION")`.
- Registered via `register_builtin_tools(registry, personas_dir)`.

## Plugin System (nota-runtime)

### Architecture
- `deno_core 0.408` embeds V8. Each plugin gets its own `JsRuntime` (isolate).
- `PluginManager`: scans `~/.nota/plugins/` for `plugin.json`, loads/dispatches/reloads.
- **Embedded plugins**: compiled in via `include_str!` + `PluginInstance::load_from_memory()`
  — NO filesystem seeding. User plugins loaded from disk.
- Plugin lifecycle: `register → start → stop`. Hot reload: stop → new isolate → register → start.

### NotaContext
- JS plugins access `ctx.tool.register({name, description, parameters, run})` via deno_core ops.
- Tool metadata is stored; tool execution through JS (async op bridge) is TODO.
- `JsRuntime` is `Send` but NOT `Sync` — cannot be shared via `Arc<dyn Tool>`. Channel-based
  dispatch needed for cross-thread tool execution.

### deno_core patterns
- `#[op2(fast)]` for sync native functions: `state: &mut OpState`, `#[string]` for strings.
- OpDecl is obtained by calling the op function: `op_register_tool()` returns `OpDecl`.
- V8 function references can be passed through ops: `run_fn: v8::Local<v8::Function>`.
  Creating `v8::Global` requires `CallbackScope::new(&run_fn)`.

## LLM Client (nota-infra)

### OpenAiLlm
- OpenAI-compatible chat completions API.
- Request uses typed structs (`ChatMessage`, `ApiTool`), NOT raw `serde_json::Value` or `json!()`.
- Tool role translation: `tool_call` → assistant with `tool_calls`, `tool_result` → tool with `tool_call_id`.
- `ChatMessage` has optional `content`, `tool_calls`, `tool_call_id` fields.

## Hexagonal Refactor (workspace split)

The project was restructured into a Cargo workspace (`nota-core` / `nota-infra` /
`nota-cli`) using ports & adapters. Key decisions:

### Domain Purity (nota-core)
- Core entities (`Metadata`, `Message`, `Schedule`, `Persona`, `Session`) carry
  **no** `crudly::*` / `sqlx::FromRow` derives. Persistence row structs with
  those derives live only in `nota-infra/src/sqlite/row.rs`, bridged to core via
  `From` impls.
- `Session` no longer holds a `SqlitePool`; persistence is delegated to the
  `SessionRepository` port injected into `SessionManager`.
- `nota-core` `Cargo.toml` must NOT contain sqlx/crudly/axum/tracing/dialoguer/
  dirs/walkdir/tracing-subscriber.

### No Global State (DI)
- Removed `OnceLock<SessionManager>`, `OnceLock<PersonaManager>`, and
  `static BASE_DIR`. Managers take their ports (`Arc<dyn SessionRepository>`,
  `Arc<dyn PersonaStore>`, `Arc<dyn LlmClient>`) via constructors; `nota-cli`
  wires adapters in `main` and injects them.
- `PersonaManager` merged the old `PersonaHandler` and now `impl SessionHandler`
  directly; it is registered as the default handler via
  `SessionManager::register_handler_all`.
- `base_dir()` is resolved in `nota-cli` (`dirs::home_dir().join(".nota")`) and
  passed into adapters; the core never touches paths.

### Logging Boundary
- `nota-core`/`nota-infra` use the `log` facade only (`log::*`). `nota-cli` uses
  `tracing` + `tracing-log` (`LogTracer::init()`) to route `log` records into
  the tracing subscriber. Do not call `tracing::*` from core/infra.

### Deadlock Fix (DI side-effect)
- `set_archive_at` previously held `session_map.write()` then reentered the
  global `SessionManager::get().archive_expired_sessions()` — a reentrant
  deadlock. DI removed the global singleton, so the reentry is gone. The
  `// 这有bug` comment is retained with an explanatory addendum. Archive
  scheduling redesign is out of scope.