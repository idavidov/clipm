# Release Engineer Memory

## Project: clipm
- CLI clipboard manager for macOS, Rust/SQLite/FTS5
- Repo: https://github.com/idavidov/clipm

## Branch Conventions
- Feature branches: `feature/<descriptive-name>` (e.g., `feature/days-filter`, `feature/list_label`)
- PRs merge into `main`

## Build Checks
- `cargo build --release`, `cargo test`, `cargo clippy -- -D warnings` -- all three must pass
- As of 2026-02-17: 45 tests, all passing, build is clean

## Code Review Patterns
- DB queries use dynamic query building with `Vec<Box<dyn rusqlite::types::ToSql>>` for parameterized queries
- `chrono` crate used for date/time calculations (Duration, Utc::now, to_rfc3339)
- `WHERE 1=1` pattern used for dynamic SQL filter composition in db.rs
- Test helper `sample_entry_at()` allows setting custom `created_at` timestamps

## DB Migration State
- Current schema version: `PRAGMA user_version = 2`
- v1: Initial schema with clips table, FTS5, basic triggers
- v2: Updated FTS triggers to exclude password content from index, added idx_clips_content_type index

## PR History
- PR #8: Add --days filter to list and search commands (feature/days-filter)
- PR #9: Add Password content type with --type flag (feature/password-content-type)
