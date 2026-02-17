use rusqlite::{Connection, params};
use std::path::PathBuf;

use crate::models::{ClipEntry, ClipmError, ContentType};

fn db_path() -> Result<PathBuf, ClipmError> {
    let dir = dirs::data_dir()
        .ok_or_else(|| ClipmError::Database("Cannot determine data directory".into()))?
        .join("clipm");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ClipmError::Database(format!("Cannot create data directory: {e}")))?;
    Ok(dir.join("history.db"))
}

pub fn open() -> Result<Connection, ClipmError> {
    let path = db_path()?;
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), ClipmError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version >= 1 {
        return Ok(());
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clips (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            content     TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'text',
            byte_size   INTEGER NOT NULL,
            created_at  TEXT NOT NULL,
            label       TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_clips_label ON clips(label);

        CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
            content,
            label,
            content='clips',
            content_rowid='id'
        );

        -- Sync triggers
        CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
            INSERT INTO clips_fts(rowid, content, label)
            VALUES (new.id, new.content, new.label);
        END;

        CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
            INSERT INTO clips_fts(clips_fts, rowid, content, label)
            VALUES ('delete', old.id, old.content, old.label);
        END;

        CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
            INSERT INTO clips_fts(clips_fts, rowid, content, label)
            VALUES ('delete', old.id, old.content, old.label);
            INSERT INTO clips_fts(rowid, content, label)
            VALUES (new.id, new.content, new.label);
        END;

        PRAGMA user_version = 1;"
    )?;
    Ok(())
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<ClipEntry> {
    Ok(ClipEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        content_type: row.get::<_, String>(2)?.parse::<ContentType>().unwrap(),
        byte_size: row.get::<_, i64>(3)? as usize,
        created_at: row.get(4)?,
        label: row.get(5)?,
    })
}

pub fn is_duplicate(conn: &Connection, content: &str) -> Result<bool, ClipmError> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM clips WHERE id = (SELECT MAX(id) FROM clips) AND content = ?1"
    )?;
    Ok(stmt.exists(params![content])?)
}

pub fn insert(conn: &Connection, entry: &ClipEntry) -> Result<i64, ClipmError> {
    conn.execute(
        "INSERT INTO clips (content, content_type, byte_size, created_at, label)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            entry.content,
            entry.content_type.to_string(),
            entry.byte_size as i64,
            entry.created_at,
            entry.label,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<ClipEntry, ClipmError> {
    conn.query_row(
        "SELECT id, content, content_type, byte_size, created_at, label FROM clips WHERE id = ?1",
        params![id],
        row_to_entry,
    ).map_err(|_| ClipmError::NotFound(format!("No entry with id {id}")))
}

pub fn update_label(conn: &Connection, id: i64, label: Option<&str>) -> Result<(), ClipmError> {
    let changed = conn.execute(
        "UPDATE clips SET label = ?1 WHERE id = ?2",
        params![label, id],
    )?;
    if changed == 0 {
        return Err(ClipmError::NotFound(format!("No entry with id {id}")));
    }
    Ok(())
}

pub fn get_most_recent(conn: &Connection) -> Result<ClipEntry, ClipmError> {
    conn.query_row(
        "SELECT id, content, content_type, byte_size, created_at, label FROM clips ORDER BY id DESC LIMIT 1",
        [],
        row_to_entry,
    ).map_err(|_| ClipmError::NotFound("No entries in history".into()))
}

pub fn list(conn: &Connection, limit: usize, offset: usize, label: Option<&str>) -> Result<Vec<ClipEntry>, ClipmError> {
    let entries = match label {
        Some(l) => {
            let mut stmt = conn.prepare(
                "SELECT id, content, content_type, byte_size, created_at, label
                 FROM clips WHERE label = ?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(params![l, limit as i64, offset as i64], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, content, content_type, byte_size, created_at, label
                 FROM clips ORDER BY id DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };
    Ok(entries)
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<ClipEntry>, ClipmError> {
    let escaped = format!("\"{}\"", query.replace('"', "\"\""));
    let mut stmt = conn.prepare(
        "SELECT c.id, c.content, c.content_type, c.byte_size, c.created_at, c.label
         FROM clips_fts f
         JOIN clips c ON c.id = f.rowid
         WHERE clips_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    )?;
    let entries = stmt.query_map(params![escaped, limit as i64], row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), ClipmError> {
    let changed = conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(ClipmError::NotFound(format!("No entry with id {id}")));
    }
    Ok(())
}

pub fn clear(conn: &Connection) -> Result<usize, ClipmError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips", [], |r| r.get(0))?;
    conn.execute_batch("DELETE FROM clips;")?;
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample_entry(content: &str) -> ClipEntry {
        ClipEntry {
            id: 0,
            content: content.to_string(),
            content_type: ContentType::Text,
            byte_size: content.len(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            label: None,
        }
    }

    #[test]
    fn test_insert_and_get_by_id() {
        let conn = test_conn();
        let entry = sample_entry("hello world");
        let id = insert(&conn, &entry).unwrap();
        let fetched = get_by_id(&conn, id).unwrap();
        assert_eq!(fetched.content, "hello world");
        assert_eq!(fetched.id, id);
    }

    #[test]
    fn test_get_most_recent() {
        let conn = test_conn();
        insert(&conn, &sample_entry("first")).unwrap();
        insert(&conn, &sample_entry("second")).unwrap();
        let recent = get_most_recent(&conn).unwrap();
        assert_eq!(recent.content, "second");
    }

    #[test]
    fn test_get_most_recent_empty() {
        let conn = test_conn();
        assert!(get_most_recent(&conn).is_err());
    }

    #[test]
    fn test_is_duplicate() {
        let conn = test_conn();
        assert!(!is_duplicate(&conn, "anything").unwrap());
        insert(&conn, &sample_entry("hello")).unwrap();
        assert!(is_duplicate(&conn, "hello").unwrap());
        assert!(!is_duplicate(&conn, "world").unwrap());
    }

    #[test]
    fn test_is_duplicate_checks_most_recent() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello")).unwrap();
        insert(&conn, &sample_entry("world")).unwrap();
        assert!(!is_duplicate(&conn, "hello").unwrap());
        assert!(is_duplicate(&conn, "world").unwrap());
    }

    #[test]
    fn test_list() {
        let conn = test_conn();
        for i in 0..5 {
            insert(&conn, &sample_entry(&format!("entry {i}"))).unwrap();
        }
        let entries = list(&conn, 3, 0, None).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].content, "entry 4");
    }

    #[test]
    fn test_list_with_offset() {
        let conn = test_conn();
        for i in 0..5 {
            insert(&conn, &sample_entry(&format!("entry {i}"))).unwrap();
        }
        let entries = list(&conn, 2, 2, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "entry 2");
    }

    #[test]
    fn test_list_with_label_filter() {
        let conn = test_conn();
        let mut labeled = sample_entry("labeled");
        labeled.label = Some("important".to_string());
        insert(&conn, &labeled).unwrap();
        insert(&conn, &sample_entry("unlabeled")).unwrap();

        let entries = list(&conn, 10, 0, Some("important")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "labeled");
    }

    #[test]
    fn test_update_label() {
        let conn = test_conn();
        let id = insert(&conn, &sample_entry("hello")).unwrap();
        update_label(&conn, id, Some("tagged")).unwrap();
        let entry = get_by_id(&conn, id).unwrap();
        assert_eq!(entry.label.as_deref(), Some("tagged"));

        update_label(&conn, id, None).unwrap();
        let entry = get_by_id(&conn, id).unwrap();
        assert_eq!(entry.label, None);
    }

    #[test]
    fn test_update_label_nonexistent() {
        let conn = test_conn();
        assert!(update_label(&conn, 999, Some("tag")).is_err());
    }

    #[test]
    fn test_delete() {
        let conn = test_conn();
        let id = insert(&conn, &sample_entry("hello")).unwrap();
        delete(&conn, id).unwrap();
        assert!(get_by_id(&conn, id).is_err());
    }

    #[test]
    fn test_delete_nonexistent() {
        let conn = test_conn();
        assert!(delete(&conn, 999).is_err());
    }

    #[test]
    fn test_clear() {
        let conn = test_conn();
        insert(&conn, &sample_entry("one")).unwrap();
        insert(&conn, &sample_entry("two")).unwrap();
        let count = clear(&conn).unwrap();
        assert_eq!(count, 2);
        let entries = list(&conn, 10, 0, None).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_clear_empty() {
        let conn = test_conn();
        let count = clear(&conn).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_search() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello world")).unwrap();
        insert(&conn, &sample_entry("goodbye world")).unwrap();
        let results = search(&conn, "hello", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn test_search_no_results() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello world")).unwrap();
        let results = search(&conn, "nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_special_chars() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello \"world\"")).unwrap();
        let results = search(&conn, "hello", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 1);
    }
}
