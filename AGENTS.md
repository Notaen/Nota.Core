# AGENTS.md

## Required reading

Before making changes, read:
- `.agent/guide.md` — architecture, crudly patterns, commit conventions, pitfalls
- `.agent/notes.md` — design decisions, refactor history, naming rules

## Commands

```sh
cargo build                          # build (default: nota-cli)
cargo run -p nota-cli                # run
cargo check                          # type-check
cargo check -p nota-core             # type-check single crate
cargo clippy --all-targets           # lint
```

No tests, no CI exists in this repo.

## Architecture

```
nota-cli → nota-infra → nota-core   (one-way; core never sees sqlx/crudly/axum)
```

| Crate | What it does |
|-------|--------------|
| `nota-core` | Domain types + port traits (`SessionRepository`, `PersonaStore`, `LlmClient`, `SessionHandler`, `Tool`, `ToolRegistry`, `AgentRunner`). DI, no global state. |
| `nota-infra` | Adapters: SQLite via crudly+sqlx, axum HTTP, filesystem persona store, `OpenAiLlm`, TOML config, `ToolRegistryImpl`, built-in tools. |
| `nota-cli` | Binary (`nota`). Wires adapters into core, starts axum on `127.0.0.1:2349`. |
| `nota-runtime` | Plugin system: deno_core V8 embedding, `PluginManager` (scan/load/reload from disk + embedded `include_str!`). |

## Critical rules

- **Do not delete or modify comments** without understanding them. Chinese comments are authoritative.
- **Keep core pure**: never add sqlx, crudly, axum, tracing, dialoguer, dirs, walkdir, serde_json, or reqwest to `nota-core`.
- **Domain types over generic wrappers**: `nota-core` defines its own types for domain concepts (e.g. `ToolParams`, `PropertyDef` for JSON Schema). Do NOT use `serde_json::Value` or raw `String` as parameter types — model the domain directly. Serialization to/from JSON happens at the infra boundary.
- **Logging boundary**: core/infra use `log::*` facade; only `nota-cli` uses `tracing`. `tracing-log::LogTracer` bridges them.
- **DI only**: no `OnceLock<T>` or `RwLock<Option<T>>` for manager singletons. `nota-cli` creates adapters and injects them via `Arc`.
- **Edition 2024**: requires nightly Rust.

## crudly gotchas

- `DateTime<Utc>` must be stored as `i64` (Unix seconds) — crudly binds DateTime as ISO TEXT, which mismatches INTEGER columns.
- `insert()` / `InsertWithoutId` consume `self` — clone first if you need the struct afterward.
- Singleton tables (no PK): use `SelectAllNoId` / `InsertNoId`. PK tables: use `SelectAll` / `InsertWithoutId`.
- Row structs live in `nota-infra/src/sqlite/row.rs`; bridged to core entities via `From` impls.

## Migrations

Single migration file: `crates/nota-infra/assets/migrations/`. Naming: `YYYYMMDDHHMMSS_description.sql`. Each session gets its own SQLite DB with `max_connections(1)`.

## Runtime data

```
~/.nota/
├── personas/          # persona workspaces (plural, not "persona")
├── sessions/          # per-session SQLite DBs
│   └── archive/       # archived sessions
├── plugins/           # user plugins (scanned from disk, hot-reloadable)
├── .logs/             # rotating logs (30-day)
└── config.toml
```

## Plugin system (nota-runtime)

- **deno_core 0.408**: embeds V8; `#[op2(fast)]` for native function registration.
- `op2` requires `state: &mut OpState` (not full path), `#[string]` for strings, `#[serde]` for complex types.
- **JsRuntime is NOT Send + Sync** — cannot be shared across threads directly. Plugin tool execution through JS requires channel-based dispatch (not yet implemented).
- **Embedded plugins**: loaded via `include_str!` + `PluginInstance::load_from_memory()` — no filesystem seeding. User plugins are scanned from `~/.nota/plugins/` via `WalkDir`.
- Plugins register tools via `ctx.tool.register({name, description, parameters, run})` in JS. The `run` function reference is stored but execution bridge is a TODO.
- Hot reload: `PluginManager::reload(name)` → stop old isolate → unregister old tools → create new isolate → re-execute entry.

## deno_core gotchas

- `#[op2]` return type for sync ops: `()` or `Result<(), AnyError>`. Use `use deno_core::error::AnyError`.
- `OpDecl` is generated as `op_function_name()` — call the function to get the OpDecl, store in a variable for lifetime.
- `execute_script` accepts `String` directly (not `.into()`).
- V8 function passing through ops: `v8::Local<v8::Function>` works as op2 arg, but `v8::Global`requires `CallbackScope` to create.
