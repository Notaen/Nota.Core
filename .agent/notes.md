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