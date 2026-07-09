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

## Directory Layout

- Persona workspace: `base_dir().join("personas")` (plural)
- Session DBs: `base_dir().join("sessions")`, archived to `sessions/archive/`
- Rust modules use singular names (`persona`, `session`, `connect`, `config`)

## Tech Stack

- Rust (edition 2024)
- Axum 0.8.9
- SQLite via sqlx 0.9 (`runtime-tokio`, not `runtime-tokio-native-tls`)
- crudly 0.6 (sqlite feature, depends on sqlx 0.9)
- Tokio (rt-multi-thread), serde, TOML, tracing, dialoguer

## crudly Usage

| Struct | Table | ID | Required derives |
|--------|-------|----|-----------------|
| `Metadata` | `session_meta` | none (singleton, auto `NoId`) | `FromRow, Serialize, Clone, IntoRow, Schema, CrudlyDefault` |
| `Message` | `messages` | `#[crudly(id)]` (DB-assigned) | `FromRow, Clone, IntoRow, Schema, CrudlyDefault` |
| `Schedule` | `schedules` | `#[crudly(id)]` (DB-assigned) | `FromRow, Schema, CrudlyDefault` |

Singleton tables (no PK): omit `#[crudly(id)]` — crudly auto-derives `NoId`. Use `SelectAllNoId` and `InsertNoId`.

Auto-increment PK: use `#[crudly(id)]`. `InsertWithoutId` excludes the id from INSERT and returns `last_insert_rowid()`.

### Singleton read pattern

```rust
impl Metadata {
    pub async fn get(pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        use crudly::SelectAllNoId;
        let all: Vec<Self> = Self::select_all(pool).await?;
        if all.len() != 1 {
            return Err(sqlx::Error::Protocol(
                format!("session_meta must contain exactly 1 row, found {}", all.len()).into(),
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

- Each session gets its own `SqlitePool` with `max_connections(1)`.
- `sqlx::migrate!("./migrations")` runs on session creation.
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

1. **Never delete comments** — Chinese comments may be self-criticism or important context.
2. **crudly/sqlx version lock** — crudly 0.6 depends on sqlx 0.9. Don't upgrade sqlx without checking.
3. **`InsertNoId` / `InsertWithoutId` consume self** — clone before insert.
4. **`SelectAll` vs `SelectAllNoId`** — `NoId` types use `SelectAllNoId`; types with `#[crudly(id)]` use `SelectAll`.
5. **`Metadata` is a singleton** — `Metadata::get()` enforces exactly 1 row.
6. **`participant.rs` is a placeholder** — don't touch unless asked.