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
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clips (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            content     TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'text',
            byte_size   INTEGER NOT NULL,
            created_at  TEXT NOT NULL,
            label       TEXT
        );

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
        END;"
    )?;
    Ok(())
}

pub fn is_duplicate(conn: &Connection, content: &str) -> Result<bool, ClipmError> {
    let mut stmt = conn.prepare(
        "SELECT content FROM clips ORDER BY id DESC LIMIT 1"
    )?;
    let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
    Ok(result.as_deref() == Some(content))
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
        |row| {
            Ok(ClipEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                content_type: ContentType::from_str(&row.get::<_, String>(2)?),
                byte_size: row.get::<_, i64>(3)? as usize,
                created_at: row.get(4)?,
                label: row.get(5)?,
            })
        },
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
        |row| {
            Ok(ClipEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                content_type: ContentType::from_str(&row.get::<_, String>(2)?),
                byte_size: row.get::<_, i64>(3)? as usize,
                created_at: row.get(4)?,
                label: row.get(5)?,
            })
        },
    ).map_err(|_| ClipmError::NotFound("No entries in history".into()))
}

pub fn list(conn: &Connection, limit: usize, offset: usize) -> Result<Vec<ClipEntry>, ClipmError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, content_type, byte_size, created_at, label
         FROM clips ORDER BY id DESC LIMIT ?1 OFFSET ?2"
    )?;
    let entries = stmt.query_map(params![limit as i64, offset as i64], |row| {
        Ok(ClipEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            content_type: ContentType::from_str(&row.get::<_, String>(2)?),
            byte_size: row.get::<_, i64>(3)? as usize,
            created_at: row.get(4)?,
            label: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<ClipEntry>, ClipmError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.content, c.content_type, c.byte_size, c.created_at, c.label
         FROM clips_fts f
         JOIN clips c ON c.id = f.rowid
         WHERE clips_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    )?;
    let entries = stmt.query_map(params![query, limit as i64], |row| {
        Ok(ClipEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            content_type: ContentType::from_str(&row.get::<_, String>(2)?),
            byte_size: row.get::<_, i64>(3)? as usize,
            created_at: row.get(4)?,
            label: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
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
