# QA Guardian Memory - clipm

## Test Coverage Summary
- Total tests: 45 (all passing as of 2026-02-17)
- DB tests use `Connection::open_in_memory()` - no filesystem
- Commands tests cover utility functions only (truncate, format_size, format_timestamp, entry_to_row)
- Models tests cover Display/FromStr/error formatting

## Key Testing Patterns
- `test_conn()` helper creates in-memory SQLite DB with migrations
- `sample_entry()` creates test ClipEntry with ContentType::Text default
- `sample_entry_at()` for testing time-based filters
- All DB operations test both success and error paths

## Known Test Gaps
- Command functions (store, get, list, search, delete, clear) not tested - require real clipboard
- Migration from v0→v1→v2 progression not tested (only idempotency at v2)
- FTS rebuild with existing password entries not explicitly tested
- ContentType filter validation (invalid type strings) tested at model level, not integration level
- Auto-label "password" behavior only tested via manual inspection, no automated test

## Critical Behaviors to Validate
- Password content NEVER appears in FTS index (empty string in triggers)
- Password preview ALWAYS masked as "********" in list/search (entry_to_row)
- `get` returns unmasked password content (no masking in db::get_by_id)
- Duplicate check skipped for passwords (commands.rs line 69)
- Auto-label "password" when no label given for password type (commands.rs line 75-78)
- ContentType::FromStr rejects unknown types with descriptive error
- DB migration v1→v2: drops old triggers, recreates with CASE WHEN masking, rebuilds FTS, adds content_type index

## Regression-Prone Areas
- FTS5 trigger sync - any schema change to `clips` must update all 3 triggers (ai, ad, au)
- Migration version bumps - must increment PRAGMA user_version in same transaction
- `byte_size` type casting - usize in Rust, i64 in SQLite
- Unicode handling in truncate() - uses char count, not byte count

## Recent Changes (2026-02-17)
- Added Password content type feature
- DB schema v1→v2 migration for password exclusion from FTS
- All 45 tests passing, 0 Clippy warnings
- Added 13 new tests for password feature
