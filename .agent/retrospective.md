# Retrospective

Past session lessons. Read if you want to avoid repeating mistakes.

## Mistakes to Avoid
- Don't replace existing types when user says "add". Keep both.
- Don't auto-add defaults (persona, user, etc.) unless user explicitly requests.
- Don't over-engineer minimal asks (e.g. just creating a file ≠ building a CRUD manager).
- Don't hardcode names that should come from filesystem config.

## Technical Tips
- `crudly` binds `DateTime<Utc>` as ISO 8601 TEXT. Use `i64` for Unix seconds in INTEGER columns.
- `crudly::SelectAll` works on `&SqlitePool` directly — no need to acquire a connection.
- `Participant.handler: Option<Box<dyn SessionHandler>>` — store all in DB, only attach handler at runtime for types that need it.