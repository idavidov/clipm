# CLI Clipboard Manager for macOS (`clipm`)

## Context
Create a new CLI clipboard manager in Rust from scratch in the empty `/Users/username/Code/claudecli` directory. The tool saves clipboard snapshots on demand (no background daemon), stores them in SQLite with full-text search, and lets users list/search/retrieve history.

## CLI Commands
| Command | Description |
|---------|-------------|
| `clipm store [-l label]` | Save current clipboard to history |
| `clipm get [id]` | Copy entry to clipboard (default: most recent) |
| `clipm list [-l limit] [-o offset]` | Show history as table |
| `clipm search <query> [-l limit]` | FTS5 full-text search |
| `clipm delete <id>` | Delete one entry |
| `clipm clear [-f]` | Clear all history (with confirmation) |

## Storage
- **Location**: `~/Library/Application Support/clipm/history.db`
- **Schema**: `clips` table (id, content, content_type, byte_size, created_at, label) + FTS5 virtual table with sync triggers
- **Duplicate detection**: Skip insert if content matches most recent entry
