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
| `nota-core` | Domain entities, aggregates, **port traits** (`SessionRepository`, `PersonaStore`, `LlmClient`, `SessionHandler`). No global state (DI). Logging via `log` facade only. | `log`, `serde`, `async-trait`, `chrono`, `anyhow`, `tokio` (sync) |
| `nota-infra` | Adapters implementing the ports: SQLite (`sqlx`+`crudly`), axum HTTP, filesystem persona store, stub LLM, TOML config. | `nota-core`, `sqlx`, `crudly`, `axum`, `tokio`, … |
| `nota-cli` | Binary: tracing init + `tracing-log` bridge, config wizard, adapter wiring (DI), HTTP start, graceful shutdown. | `nota-core`, `nota-infra`, `tracing`, `dialoguer`, … |

### Directory Layout (source)

```
crates/nota-core/src/
├── session/{mod,entity,repository}.rs   # Session/Message/Metadata/Schedule + ports + SessionManager (DI)
└── persona/mod.rs                        # Persona + PersonaStore/LlmClient ports + PersonaManager (impl SessionHandler)

crates/nota-infra/src/
├── sqlite/{mod,row}.rs                   # SqliteSessionRepository (crudly/sqlx rows live here only)
├── persona_store/mod.rs                 # FilePersonaStore (mtime cache)
├── llm/mod.rs                            # StubLlm
├── config/mod.rs                        # Config + ConfigStore
└── http/{mod,session,admin}.rs           # axum router (state = Arc<SessionManager>)
```

### Runtime Layout

- Persona workspace: `base_dir().join("personas")` (plural)
- Session DBs: `base_dir().join("sessions")`, archived to `sessions/archive/`
- `base_dir()` is resolved in `nota-cli` (`dirs::home_dir().join(".nota")`) and injected into adapters; the core never touches paths.

## Tech Stack

- Rust (edition 2024)
- Axum 0.8.9
- SQLite via sqlx 0.9 (`runtime-tokio`, not `runtime-tokio-native-tls`)
- crudly 0.6 (sqlite feature, depends on sqlx 0.9)
- Tokio (rt-multi-thread), serde, TOML, log (core) / tracing (cli), dialoguer (cli)

## crudly Usage (nota-infra only)

Row structs live in `crates/nota-infra/src/sqlite/row.rs`; they bridge to core
entities via `From` impls. Core types carry **no** crudly/sqlx derives.

| Struct | Table | ID | Required derives |
|--------|-------|----|-----------------|
| `MetadataRow` | `session_meta` | none (singleton, auto `NoId`) | `FromRow, Serialize, Clone, IntoRow, Schema, CrudlyDefault` |
| `MessageRow` | `messages` | `#[crudly(id)]` (DB-assigned) | `FromRow, Clone, IntoRow, Schema, CrudlyDefault` |
| `ScheduleRow` | `schedules` | `#[crudly(id)]` (DB-assigned) | `FromRow, Schema, CrudlyDefault` |

Singleton tables (no PK): omit `#[crudly(id)]` — crudly auto-derives `NoId`. Use `SelectAllNoId` and `InsertNoId`.

Auto-increment PK: use `#[crudly(id)]`. `InsertWithoutId` excludes the id from INSERT and returns `last_insert_rowid()`.

### Singleton read pattern

```rust
impl MetadataRow {
    pub async fn read_from(pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        use crudly::SelectAllNoId;
        let all: Vec<Self> = Self::select_all(pool).await?;
        if all.len() != 1 {
            return Err(sqlx::Error::Protocol(
                format!("session_meta must contain exactly 1 row, found {}", all.len()),
            ));
        }
        Ok(all.into_iter().next().unwrap())
    }
}
```

### Insert takes self by value

`insert()` consumes self. Clone first if you need the struct afterward:

```rust
let id = msg.clone().insert(&pool).await?;
self.messages.push(Message { id, ..msg });
```

### DateTime: use i64, not DateTime<Utc>

crudly's `IntoRow` binds `DateTime<Utc>` as ISO 8601 `TEXT`, which mismatches `INTEGER` columns. Store as `i64` (Unix seconds):

```rust
pub created_at: i64,
pub archive_at: Option<i64>,
```

## SQLx / SQLite

- Migrations live in `crates/nota-infra/assets/migrations/` and are run via `sqlx::migrate!("./assets/migrations")` from the `nota-infra` crate.
- Each session gets its own `SqlitePool` with `max_connections(1)`, owned by `SqliteSessionRepository`.
- New session DBs: `SqliteConnectOptions::from_str(...)?.create_if_missing(true)`.

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/session` | List session metadata |
| POST | `/session` | Create session (`{"creator": "..."}`) |
| GET | `/session/{sid}/archive_at` | Get archive timestamp |
| POST | `/session/{sid}/archive_at` | Set archive timestamp |
| POST | `/admin/stop` | Graceful shutdown |

## Pitfalls

1. **Chinese comments are authoritative** — they may be self-criticism, TODOs, or rules. If a comment describes a concrete fix, implement it and record the decision in `.agent/notes.md`. Never delete a comment without understanding why it was there.
2. **crudly/sqlx version lock** — crudly 0.6 depends on sqlx 0.9. Don't upgrade sqlx without checking.
3. **`InsertNoId` / `InsertWithoutId` consume self** — clone before insert.
4. **`SelectAll` vs `SelectAllNoId`** — `NoId` types use `SelectAllNoId`; types with `#[crudly(id)]` use `SelectAll`.
5. **`MetadataRow` is a singleton** — `MetadataRow::read_from()` enforces exactly 1 row.
6. **Keep core pure** — never add sqlx/crudly/axum/tracing to `nota-core`. Adapters belong in `nota-infra`; wiring in `nota-cli`.
7. **Logging** — `nota-core`/`nota-infra` use the `log` facade; `nota-cli` bridges it into `tracing` via `tracing_log::LogTracer`. Don't call `tracing::*` from core/infra.
