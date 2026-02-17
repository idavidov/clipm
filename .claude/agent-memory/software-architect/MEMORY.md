# Architect Memory - clipm

## Project Structure
- Rust CLI clipboard manager for macOS, SQLite-backed with FTS5
- 5 modules: cli.rs (clap), clipboard.rs (arboard), commands.rs (logic), db.rs (rusqlite), models.rs (types)
- DB path: ~/Library/Application Support/clipm/history.db

## Key Architecture Facts
- `ContentType` enum currently only has `Text`; `FromStr` accepts ANY string and returns `Text` (line 19 of models.rs)
- FTS5 indexes both `content` and `label` columns via triggers (clips_ai, clips_ad, clips_au)
- DB migration versioned via `PRAGMA user_version`, currently at version 1
- `row_to_entry` in db.rs uses `.unwrap()` on ContentType parse (line 80) - safe only because FromStr is Infallible
- Dynamic SQL building in `db::list()` and `db::search()` uses `Vec<Box<dyn ToSql>>` pattern
- `entry_to_row` in commands.rs builds ClipRow for tabled display (id, preview, label, created_at)
- `ClipRow` does NOT include content_type column in display

## Schema (user_version=1)
- clips: id, content, content_type, byte_size, created_at, label
- clips_fts: content, label (FTS5 virtual table synced via triggers)
- idx_clips_label index on label column

## Testing Patterns
- DB tests use `Connection::open_in_memory()` with `migrate()`
- commands.rs tests cover only pure utility functions (truncate, format_size, format_timestamp)
- Test helpers: `sample_entry()`, `sample_entry_at()` in db.rs tests
