# Nota

A framework for AI agent sessions with SQLite-backed message storage, built
with a hexagonal (ports & adapters) architecture.

## Build

```sh
cargo build
```

## Run

```sh
cargo run -p nota-cli
```

## Architecture

The project is a Cargo workspace of three crates:

| Crate | Role |
|-------|------|
| `nota-core` | Domain entities, aggregates, and **port traits**. No persistence/HTTP/IO dependencies — only `log` (facade), `serde`, `async-trait`, `chrono`, `anyhow`, `tokio` (sync). |
| `nota-infra` | **Adapters**: SQLite repository (sqlx + crudly), axum HTTP, filesystem persona store, stub LLM, TOML config. Implements the `nota-core` ports. |
| `nota-cli` | Binary entry point. Initializes logging (`tracing` + `tracing-log` bridge), runs the config wizard, wires adapters into core, and starts the HTTP server. |

Dependency flow is strictly one-way: `nota-cli → nota-infra → nota-core`. The
core never references sqlx, axum, crudly, or `tracing`.

## Tech Stack

Rust 2024 · Axum · SQLite (sqlx 0.9) · crudly 0.6 · Tokio

## API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/session` | List sessions |
| POST | `/session` | Create session |
| GET | `/session/{sid}/archive_at` | Get archive time |
| POST | `/session/{sid}/archive_at` | Set archive time |
| POST | `/admin/stop` | Graceful shutdown |

## Layout

```
crates/
├── nota-core/    # domain + ports
├── nota-infra/   # adapters (sqlite / http / persona_store / llm / config)
└── nota-cli/     # binary: wiring + logging
```

Runtime data lives under the user's home directory:

```
~/.nota/
├── personas/          # persona workspaces
├── sessions/          # active session DBs
│   └── archive/       # archived sessions
├── .logs/             # rotating logs (30-day)
└── config.toml
```

## Documentation

See [`.agent/guide.md`](.agent/guide.md) for development conventions and crudly usage patterns.
