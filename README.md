# Nota.Core

A framework for AI agent sessions with SQLite-backed message storage.

## Build

```sh
cargo build
```

## Run

```sh
cargo run
```

## Tech Stack

Rust 2024 · Axum · SQLite (sqlx 0.9) · Tokio

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
~/.nota/
├── personas/          # persona workspaces
├── sessions/          # active session DBs
│   └── archive/       # archived sessions
├── .logs/             # rotating logs (30-day)
└── config.toml
```

## Documentation

See [`.agent/guide.md`](.agent/guide.md) for development conventions and crudly usage patterns.