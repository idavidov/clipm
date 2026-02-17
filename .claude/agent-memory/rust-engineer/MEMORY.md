# clipm Project Memory

## Schema Migrations

- DB migrations are multi-version via `PRAGMA user_version`
- Migration v1→v2 added password support:
  - Dropped and recreated FTS triggers with `CASE WHEN content_type = 'password' THEN '' ELSE content END` to exclude password content from FTS
  - Used `INSERT INTO clips_fts(clips_fts) VALUES('rebuild')` to rebuild FTS index
  - Added index on content_type column
  - Updated test_migration_idempotent to expect version 2

## FTS5 Trigger Pattern

When schema changes require updating FTS triggers:
1. DROP old triggers first
2. CREATE new triggers with updated logic
3. Rebuild FTS index with `INSERT INTO clips_fts(clips_fts) VALUES('rebuild')`
4. Set new PRAGMA user_version in same transaction

## ContentType Enum

- Adding new variants requires:
  - Add variant to enum with `PartialEq` derive
  - Update `Display` impl match arm
  - Update `FromStr` impl (changed from `Infallible` to `String` error type in v2)
  - Add tests for Display, FromStr, and PartialEq

## Function Signatures with Optional Filters

Pattern for list/search functions with multiple optional filters:
- Build SQL string dynamically with `WHERE 1=1` base
- Use `Vec<Box<dyn rusqlite::types::ToSql>>` for params
- Push conditions with `sql.push_str(" AND field = ?")`
- Convert to refs with `params.iter().map(|p| p.as_ref()).collect()`

## Test Updates for Signature Changes

When adding a parameter to public API functions:
- Use `replace_all: true` in Edit tool to update all call sites in tests at once
- Check both db.rs and commands.rs test modules
- Pass `None` for new optional parameters in existing tests

## Password Content Type Behavior

- Passwords skip duplicate detection (is_duplicate not called)
- Auto-labeled as "password" if no label provided
- Masked as `"********"` in list/search table output (entry_to_row)
- Content excluded from FTS index but labels are indexed
- Checked via `content_type == ContentType::Password` (requires PartialEq derive)
