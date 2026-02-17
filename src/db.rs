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
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;"
    )?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), ClipmError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < 1 {
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
    }

    if version < 2 {
        conn.execute_batch(
            "-- Drop old triggers
            DROP TRIGGER IF EXISTS clips_ai;
            DROP TRIGGER IF EXISTS clips_ad;
            DROP TRIGGER IF EXISTS clips_au;

            -- Recreate triggers with password masking
            CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
                INSERT INTO clips_fts(rowid, content, label)
                VALUES (
                    new.id,
                    CASE WHEN new.content_type = 'password' THEN '' ELSE new.content END,
                    new.label
                );
            END;

            CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, content, label)
                VALUES (
                    'delete',
                    old.id,
                    CASE WHEN old.content_type = 'password' THEN '' ELSE old.content END,
                    old.label
                );
            END;

            CREATE TRIGGER clips_au AFTER UPDATE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, content, label)
                VALUES (
                    'delete',
                    old.id,
                    CASE WHEN old.content_type = 'password' THEN '' ELSE old.content END,
                    old.label
                );
                INSERT INTO clips_fts(rowid, content, label)
                VALUES (
                    new.id,
                    CASE WHEN new.content_type = 'password' THEN '' ELSE new.content END,
                    new.label
                );
            END;

            -- Rebuild FTS index
            INSERT INTO clips_fts(clips_fts) VALUES('rebuild');

            -- Add index on content_type
            CREATE INDEX IF NOT EXISTS idx_clips_content_type ON clips(content_type);

            PRAGMA user_version = 2;"
        )?;
    }

    Ok(())
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<ClipEntry> {
    let content_type_str: String = row.get(2)?;
    let content_type = content_type_str.parse::<ContentType>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    Ok(ClipEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        content_type,
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
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => ClipmError::NotFound(format!("No entry with id {id}")),
        other => ClipmError::Database(other.to_string()),
    })
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
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => ClipmError::NotFound("No entries in history".into()),
        other => ClipmError::Database(other.to_string()),
    })
}

pub fn list(conn: &Connection, limit: usize, offset: usize, label: Option<&str>, days: Option<u32>, content_type: Option<&str>) -> Result<Vec<ClipEntry>, ClipmError> {
    let mut sql = "SELECT id, content, content_type, byte_size, created_at, label FROM clips WHERE 1=1".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(l) = label {
        sql.push_str(" AND label = ?");
        params.push(Box::new(l.to_string()));
    }

    if let Some(d) = days {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(d as i64);
        let cutoff_str = cutoff.to_rfc3339();
        sql.push_str(" AND created_at >= ?");
        params.push(Box::new(cutoff_str));
    }

    if let Some(ct) = content_type {
        sql.push_str(" AND content_type = ?");
        params.push(Box::new(ct.to_string()));
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit as i64));
    params.push(Box::new(offset as i64));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let entries = stmt.query_map(param_refs.as_slice(), row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn search(conn: &Connection, query: &str, limit: usize, days: Option<u32>, content_type: Option<&str>) -> Result<Vec<ClipEntry>, ClipmError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(ClipmError::InvalidInput("Empty search query".into()));
    }
    let escaped = trimmed.replace('"', "\"\"");

    let mut sql = "SELECT c.id, c.content, c.content_type, c.byte_size, c.created_at, c.label
         FROM clips_fts f
         JOIN clips c ON c.id = f.rowid
         WHERE clips_fts MATCH ?1".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(escaped)];

    if let Some(d) = days {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(d as i64);
        let cutoff_str = cutoff.to_rfc3339();
        sql.push_str(" AND c.created_at >= ?");
        params.push(Box::new(cutoff_str));
    }

    if let Some(ct) = content_type {
        sql.push_str(" AND c.content_type = ?");
        params.push(Box::new(ct.to_string()));
    }

    sql.push_str(" ORDER BY bm25(clips_fts) LIMIT ?");
    params.push(Box::new(limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let entries = stmt.query_map(param_refs.as_slice(), row_to_entry)?
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

    fn sample_entry_at(content: &str, created_at: &str) -> ClipEntry {
        ClipEntry {
            id: 0,
            content: content.to_string(),
            content_type: ContentType::Text,
            byte_size: content.len(),
            created_at: created_at.to_string(),
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
    fn test_get_by_id_db_error_not_masked() {
        let conn = test_conn();
        conn.execute_batch("DROP TABLE clips;").unwrap();
        let err = get_by_id(&conn, 1).unwrap_err();
        assert!(matches!(err, ClipmError::Database(_)));
    }

    #[test]
    fn test_get_most_recent_db_error_not_masked() {
        let conn = test_conn();
        conn.execute_batch("DROP TABLE clips;").unwrap();
        let err = get_most_recent(&conn).unwrap_err();
        assert!(matches!(err, ClipmError::Database(_)));
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
        let entries = list(&conn, 3, 0, None, None, None).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].content, "entry 4");
    }

    #[test]
    fn test_list_with_offset() {
        let conn = test_conn();
        for i in 0..5 {
            insert(&conn, &sample_entry(&format!("entry {i}"))).unwrap();
        }
        let entries = list(&conn, 2, 2, None, None, None).unwrap();
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

        let entries = list(&conn, 10, 0, Some("important"), None, None).unwrap();
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
        let entries = list(&conn, 10, 0, None, None, None).unwrap();
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
        let results = search(&conn, "hello", 10, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn test_search_no_results() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello world")).unwrap();
        let results = search(&conn, "nonexistent", 10, None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_special_chars() {
        let conn = test_conn();
        insert(&conn, &sample_entry("hello \"world\"")).unwrap();
        let results = search(&conn, "hello", 10, None, None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty_query() {
        let conn = test_conn();
        let err = search(&conn, "   ", 10, None, None).unwrap_err();
        assert!(matches!(err, ClipmError::InvalidInput(_)));
    }

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn test_list_with_days_filter() {
        let conn = test_conn();
        let now = chrono::Utc::now();
        let three_days_ago = now - chrono::Duration::days(3);
        let thirty_days_ago = now - chrono::Duration::days(30);

        insert(&conn, &sample_entry_at("today", &now.to_rfc3339())).unwrap();
        insert(&conn, &sample_entry_at("three days ago", &three_days_ago.to_rfc3339())).unwrap();
        insert(&conn, &sample_entry_at("thirty days ago", &thirty_days_ago.to_rfc3339())).unwrap();

        let entries = list(&conn, 10, 0, None, Some(7), None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "three days ago");
        assert_eq!(entries[1].content, "today");
    }

    #[test]
    fn test_list_with_days_and_label() {
        let conn = test_conn();
        let now = chrono::Utc::now();
        let ten_days_ago = now - chrono::Duration::days(10);

        let mut recent_labeled = sample_entry_at("recent labeled", &now.to_rfc3339());
        recent_labeled.label = Some("important".to_string());
        insert(&conn, &recent_labeled).unwrap();

        let mut old_labeled = sample_entry_at("old labeled", &ten_days_ago.to_rfc3339());
        old_labeled.label = Some("important".to_string());
        insert(&conn, &old_labeled).unwrap();

        let mut recent_unlabeled = sample_entry_at("recent unlabeled", &now.to_rfc3339());
        recent_unlabeled.label = None;
        insert(&conn, &recent_unlabeled).unwrap();

        let entries = list(&conn, 10, 0, Some("important"), Some(7), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "recent labeled");
    }

    #[test]
    fn test_search_with_days_filter() {
        let conn = test_conn();
        let now = chrono::Utc::now();
        let five_days_ago = now - chrono::Duration::days(5);
        let twenty_days_ago = now - chrono::Duration::days(20);

        insert(&conn, &sample_entry_at("hello recent", &now.to_rfc3339())).unwrap();
        insert(&conn, &sample_entry_at("hello five", &five_days_ago.to_rfc3339())).unwrap();
        insert(&conn, &sample_entry_at("hello old", &twenty_days_ago.to_rfc3339())).unwrap();

        let results = search(&conn, "hello", 10, Some(10), None).unwrap();
        assert_eq!(results.len(), 2);
        let contents: Vec<String> = results.iter().map(|e| e.content.clone()).collect();
        assert!(contents.contains(&"hello recent".to_string()));
        assert!(contents.contains(&"hello five".to_string()));
        assert!(!contents.contains(&"hello old".to_string()));
    }

    #[test]
    fn test_insert_password_entry() {
        let conn = test_conn();
        let mut entry = sample_entry("my-secret-password");
        entry.content_type = ContentType::Password;
        let id = insert(&conn, &entry).unwrap();
        let fetched = get_by_id(&conn, id).unwrap();
        assert_eq!(fetched.content, "my-secret-password");
        assert_eq!(fetched.content_type, ContentType::Password);
    }

    #[test]
    fn test_password_not_in_fts() {
        let conn = test_conn();
        let mut entry = sample_entry("my-secret-password");
        entry.content_type = ContentType::Password;
        insert(&conn, &entry).unwrap();
        let results = search(&conn, "secret", 10, None, None).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_password_label_in_fts() {
        let conn = test_conn();
        let mut entry = sample_entry("my-secret-password");
        entry.content_type = ContentType::Password;
        entry.label = Some("github-token".to_string());
        insert(&conn, &entry).unwrap();
        let results = search(&conn, "github", 10, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "my-secret-password");
    }

    #[test]
    fn test_list_filter_by_content_type() {
        let conn = test_conn();
        insert(&conn, &sample_entry("text content")).unwrap();
        let mut pass_entry = sample_entry("password123");
        pass_entry.content_type = ContentType::Password;
        insert(&conn, &pass_entry).unwrap();

        let text_entries = list(&conn, 10, 0, None, None, Some("text")).unwrap();
        assert_eq!(text_entries.len(), 1);
        assert_eq!(text_entries[0].content, "text content");

        let pass_entries = list(&conn, 10, 0, None, None, Some("password")).unwrap();
        assert_eq!(pass_entries.len(), 1);
        assert_eq!(pass_entries[0].content, "password123");
    }

    #[test]
    fn test_search_filter_by_content_type() {
        let conn = test_conn();
        let mut text_entry = sample_entry("hello world");
        text_entry.label = Some("greeting".to_string());
        insert(&conn, &text_entry).unwrap();

        let mut pass_entry = sample_entry("secret123");
        pass_entry.content_type = ContentType::Password;
        pass_entry.label = Some("greeting".to_string());
        insert(&conn, &pass_entry).unwrap();

        let text_results = search(&conn, "greeting", 10, None, Some("text")).unwrap();
        assert_eq!(text_results.len(), 1);
        assert_eq!(text_results[0].content, "hello world");

        let pass_results = search(&conn, "greeting", 10, None, Some("password")).unwrap();
        assert_eq!(pass_results.len(), 1);
        assert_eq!(pass_results[0].content, "secret123");
    }
}
