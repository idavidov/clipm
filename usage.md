# clipm Usage

A CLI clipboard manager for macOS. Saves clipboard snapshots on demand, stores them in SQLite with full-text search, and lets you list, search, and retrieve history.

## Installation

```bash
cargo build --release
cp target/release/clipm /usr/local/bin/
```

## Commands

### Store clipboard

Save the current clipboard contents to history.

```bash
clipm store
clipm store -l "meeting notes"
```

Duplicate detection: if the clipboard content matches the most recent entry, the store is skipped.

### Get an entry

Copy an entry back to the clipboard. Defaults to the most recent entry.

```bash
clipm get        # most recent entry
clipm get 5      # entry with ID 5
```

### List history

Show clipboard history as a table.

```bash
clipm list
clipm list -l 10          # show 10 entries
clipm list -l 10 -o 20    # show 10 entries, skip first 20
```

### Search

Full-text search across content and labels using SQLite FTS5.

```bash
clipm search "meeting"
clipm search "TODO" -l 5
```

### Label an entry

Add, update, or remove a label on an existing entry.

```bash
clipm label 3 "important"    # set label
clipm label 3                # remove label
```

### Delete an entry

```bash
clipm delete 3
```

### Clear all history

```bash
clipm clear       # prompts for confirmation
clipm clear -f    # skip confirmation
```

## Storage

History is stored at `~/Library/Application Support/clipm/history.db`.
