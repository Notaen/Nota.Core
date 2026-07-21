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
| `nota-core` | Domain types + port traits (`SessionRepository`, `PersonaStore`, `LlmClient`, `SessionHandler`). DI, no global state. |
| `nota-infra` | Adapters: SQLite via crudly+sqlx, axum HTTP, filesystem persona store, stub LLM, TOML config. |
| `nota-cli` | Binary (`nota`). Wires adapters into core, starts axum on `127.0.0.1:2349`. |

## Critical rules

- **Do not delete or modify comments** without understanding them. Chinese comments are authoritative.
- **Keep core pure**: never add sqlx, crudly, axum, tracing, dialoguer, dirs, or walkdir to `nota-core`.
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
├── .logs/             # rotating logs (30-day)
└── config.toml
```
