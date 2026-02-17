# clipm

CLI clipboard manager for macOS. Stores clipboard snapshots in SQLite with FTS5 full-text search.

## Commands

```bash
cargo build --release          # build
cargo test                     # run all tests (32 unit tests)
cargo test -- test_name        # run a single test
cargo clippy                   # lint
```

## Architecture

```
src/
  main.rs      — CLI entry point, dispatches subcommands
  cli.rs       — clap argument definitions (Cli, Command enum)
  clipboard.rs — read/write system clipboard via arboard
  commands.rs  — business logic for each subcommand
  db.rs        — SQLite database (rusqlite), migrations, CRUD, FTS5 search
  models.rs    — ClipEntry, ContentType, ClipmError types
```

## Key Patterns

- **Error handling**: All errors flow through `ClipmError` enum with `From` impls for rusqlite, arboard, and std::io errors. Use `?` operator throughout.
- **DB migrations**: Versioned via `PRAGMA user_version`. Check version before running schema. Bump version in the same `execute_batch`.
- **FTS5 sync**: Triggers (`clips_ai`, `clips_ad`, `clips_au`) keep `clips_fts` virtual table in sync with `clips`. Any schema change to `clips` must update these.
- **Duplicate detection**: `is_duplicate` checks only the most recent entry, not all history.
- **Search escaping**: FTS5 queries are double-quote escaped to handle special characters.

## Testing

- DB tests use `Connection::open_in_memory()` — no filesystem needed.
- `commands.rs` tests cover utility functions only (truncate, format_size, format_timestamp) since command functions require a real clipboard.
- `models.rs` tests cover Display/FromStr/error formatting.

## Gotchas

- `truncate()` in commands.rs counts **chars** not bytes (unicode-safe). Don't use byte slicing.
- `byte_size` on `ClipEntry` is `usize` in Rust but stored as `INTEGER` (i64) in SQLite — cast when reading/writing.
- DB path is `~/Library/Application Support/clipm/history.db` (macOS-specific via `dirs::data_dir()`).
- `ContentType` currently only has `Text`; `FromStr` returns `Text` for any input. Add new variants to both the enum and the `fmt::Display` match.
