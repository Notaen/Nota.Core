# Project Guide

## Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description`

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code change that neither fixes nor adds
- `docs:` — documentation only
- `chore:` — tooling, deps, CI

## Code Modification Rules

- Never delete or modify existing comments without explicit approval. Chinese comments are authoritative.
- Read `.agent/notes.md` before making changes.

## Architecture (Hexagonal / Ports & Adapters)

The project is a Cargo workspace; dependency flow is one-way
`nota-cli → nota-infra → nota-core`.

| Crate | Role | Notable deps |
|-------|------|--------------|
| `nota-core` | Domain entities + **port traits** (`PersonaStore`, `LlmClient`, `Tool`, `ToolRegistry`, `AgentRunner`), `EventBus`, `PermissionRegistry`. No global state (DI). Logging via `log` facade only. | `log`, `serde`, `async-trait`, `chrono`, `anyhow`, `tokio` (sync) |
| `nota-infra` | Adapters implementing the ports: `axum` HTTP (REST + WebSocket), filesystem persona store, `OpenAiLlm`, TOML config, built-in tools. | `nota-core`, `axum`, `tokio`, `reqwest`, `serde_json`, `tower-http` |
| `nota-cli` | Binary: tracing init + `tracing-log` bridge, config wizard, adapter wiring (DI), HTTP start, graceful shutdown. | `nota-core`, `nota-infra`, `tracing`, `dialoguer` |

### Directory Layout (source)

```
crates/nota-core/src/
├── bus.rs                  # EventBus (mpsc broadcast to all subscribers) + BusEvent + EventKind
├── permissions.rs          # PermissionRegistry (pending permission oneshots keyed by id)
├── llm.rs                  # LlmClient trait + ToolDef/ToolCall/LlmResponse/ChatMessage
├── tool.rs                 # Tool / ToolRegistry traits + ToolContext (bus + permissions + request_id)
├── agent/mod.rs            # AgentRunner: LLM ↔ tool loop, returns ChatMessage list
└── persona/mod.rs          # Persona + PersonaStore trait + PersonaRuntime (event loop)

crates/nota-infra/src/
├── persona_store/mod.rs    # FilePersonaStore (chatlog.json + solo.md + memory.md)
├── llm/mod.rs              # OpenAiLlm (OpenAI-compatible chat completions)
├── config/mod.rs           # Config + ConfigStore
├── tool/{mod,builtin}.rs   # ToolRegistryImpl + file_read/file_write/schedule/get_version
└── http/{mod,ws,api,admin}.rs  # axum router: REST /api/*, WS /ws/chat, /admin/stop
```

### Runtime Layout

```
~/.nota/
├── personas/          # persona workspaces (plural, not "persona"); each has solo.md, memory.md, chatlog.json
├── plugins/           # user plugins (deno_core scanned from disk, hot-reloadable)
├── .logs/             # rotating logs (30-day)
└── config.toml        # api_url, api_key, model
```

- `base_dir()` is resolved in `nota-cli` (`dirs::home_dir().join(".nota")`) and injected into adapters; the core never touches paths.

## Tech Stack

- Rust (edition 2024)
- Axum 0.8.9 (with `ws` feature for WebSocket support)
- Tokio (rt-multi-thread, sync, fs)
- reqwest 0.13 (OpenAI-compatible HTTP)
- serde, serde_json, TOML
- log (core/infra) / tracing (cli only, via `tracing-log::LogTracer`)
- dialoguer (cli onboarding wizard)
- Frontend: Bun + React 19 + Vite 6 + Tailwind 3 (in separate `Nota.Webui` repo, see WebUI section)

## Event Bus

`nota-core::bus::EventBus` is a multi-producer / multi-consumer FIFO. Every
subscriber (`bus.subscribe()`) gets its own unbounded mpsc receiver; `bus.send(event)`
clones the event to all of them.

`BusEvent`:
```rust
pub struct BusEvent {
    pub kind: EventKind,                  // Message | PermissionRequest
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub context: String,
    pub request_id: Option<String>,
    pub parent_request_id: Option<String>,
    pub target: Option<String>,           // if Some, only the persona with this name processes it
}
```

Persona loop filters by `target`:
```rust
if event.sender == name { continue; }
if let Some(ref t) = event.target { if t != &name { continue; } }
```

The HTTP layer subscribes once, tracks `active_request_ids` per WS connection,
and only forwards events whose `request_id` (or `parent_request_id` for permission
requests) is in that set — so multiple WS clients coexist without leaking each
other's messages.

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/api/personas` | List personas |
| POST | `/api/personas` | Create persona (`{"name": "..."}`) |
| GET | `/api/personas/:name` | Persona info |
| DELETE | `/api/personas/:name` | Delete persona |
| GET | `/api/personas/:name/files/:filename` | Read persona file |
| PUT | `/api/personas/:name/files/:filename` | Write persona file (`{"content": "..."}`) |
| GET | `/api/personas/:name/chatlog` | Read chatlog |
| GET | `/api/settings` | Get config (api_url, api_key, model) |
| PUT | `/api/settings` | Update config |
| GET | `/ws/chat` | WebSocket: chat channel |
| POST | `/admin/stop` | Graceful shutdown |

### WebSocket protocol (`/ws/chat`)

Client → Server:
```json
{ "type": "send", "persona": "alice", "content": "hello", "request_id": "<uuid>" }
{ "type": "permission", "permission_id": "<uuid>", "approved": true }
```

Server → Client:
```json
{ "type": "message", "content": "hi", "request_id": "<uuid>" }
{ "type": "permission_needed", "permission_id": "<uuid>", "prompt": "Allow file_read on /etc/passwd?", "request_id": "<uuid>" }
{ "type": "error", "content": "..." }
```

## Web UI (submodule)

`webui/` is a **git submodule** tracking `https://github.com/Notaen/Nota.Webui.git`.
It is a separate TypeScript + React + Vite project. See `AGENTS.md` "Web UI
submodule" for clone/build instructions.

`nota webui` (a subcommand of the CLI) serves `webui/dist/` on port 5173
using `tower-http::services::ServeDir` with SPA fallback to `index.html`. The
web UI connects to the running `nota` server (default port 2349) for WS + REST.

Override the webui directory via the `NOTA_WEBUI_DIR` env var.

## Pitfalls

1. **Chinese comments are authoritative** — they may be self-criticism, TODOs, or rules. If a comment describes a concrete fix, implement it and record the decision in `.agent/notes.md`. Never delete a comment without understanding why it was there.
2. **Keep core pure** — never add `axum`/`reqwest`/`serde_json`/`tracing`/`dialoguer`/`dirs`/`walkdir`/`tokio-tungstenite` to `nota-core`. Adapters belong in `nota-infra`; wiring in `nota-cli`. `tokio` is allowed but only for sync primitives (`tokio::sync::RwLock`, etc.) — no runtime.
3. **No global state** — `OnceLock<T>` / `RwLock<Option<T>>` are forbidden for manager singletons. `nota-cli` creates adapters and injects them via `Arc`.
4. **Domain types over generics** — model the domain directly (`ToolParams`/`PropertyDef` for JSON Schema, `ChatMessage` for LLM turns). Serialization happens at the infra boundary, not in core.
5. **Logging** — `nota-core`/`nota-infra` use the `log` facade; `nota-cli` bridges it into `tracing` via `tracing_log::LogTracer`. Don't call `tracing::*` from core/infra.
6. **Async blocking** — never call `blocking_*` on a tokio RwLock from inside an async context. Use `.write().await`. `blocking_*` panics the runtime.
7. **WebSocket ↔ bus routing** — the WS handler filters events by `request_id` in its `active_request_ids` set. Events with mismatched `request_id` are silently dropped (don't echo other clients' messages).
8. **Default persona is gone** — `nota` no longer auto-creates any persona. Use `nota onboard` or manually create files under `~/.nota/personas/<name>/`.
