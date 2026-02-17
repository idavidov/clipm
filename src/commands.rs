use std::io::{self, Write};
use tabled::{Table, Tabled};

use crate::clipboard;
use crate::db;
use crate::models::{ClipEntry, ClipmError, ContentType};

#[derive(Tabled)]
struct ClipRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Preview")]
    preview: String,
    #[tabled(rename = "Label")]
    label: String,
    #[tabled(rename = "Created")]
    created_at: String,
}

fn truncate(s: &str, max_chars: usize) -> String {
    let single_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    let char_count = single_line.chars().count();
    if char_count <= max_chars {
        single_line
    } else {
        let truncated: String = single_line.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    }
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn format_timestamp(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

fn entry_to_row(e: &ClipEntry) -> ClipRow {
    let preview = if e.content_type == ContentType::Password {
        "********".to_string()
    } else {
        truncate(&e.content, 60)
    };
    ClipRow {
        id: e.id,
        preview,
        label: e.label.clone().unwrap_or_default(),
        created_at: format_timestamp(&e.created_at),
    }
}

pub fn store(label: Option<String>, content_type_str: &str) -> Result<(), ClipmError> {
    let content = clipboard::read_text()?;
    let conn = db::open()?;

    let content_type = content_type_str.parse::<ContentType>()
        .map_err(ClipmError::InvalidInput)?;

    // Skip duplicate check for passwords
    if content_type != ContentType::Password && db::is_duplicate(&conn, &content)? {
        println!("Skipped: content matches most recent entry.");
        return Ok(());
    }

    // Auto-label as "password" if no label given for password type
    let label = match (label, &content_type) {
        (None, ContentType::Password) => Some("password".to_string()),
        (l, _) => l,
    };

    let entry = ClipEntry {
        id: 0,
        byte_size: content.len(),
        content_type,
        created_at: chrono::Utc::now().to_rfc3339(),
        label,
        content,
    };
    let id = db::insert(&conn, &entry)?;
    match &entry.label {
        Some(l) => println!("Stored as entry #{id} ({}, label: \"{l}\").", format_size(entry.byte_size)),
        None => println!("Stored as entry #{id} ({}).", format_size(entry.byte_size)),
    }
    Ok(())
}

pub fn get(id: Option<i64>) -> Result<(), ClipmError> {
    let conn = db::open()?;
    let entry = match id {
        Some(id) => db::get_by_id(&conn, id)?,
        None => db::get_most_recent(&conn)?,
    };
    clipboard::write_text(&entry.content)?;
    println!(
        "Copied entry #{} to clipboard ({}).",
        entry.id,
        format_size(entry.byte_size)
    );
    Ok(())
}

pub fn list(limit: usize, offset: usize, label: Option<&str>, days: Option<u32>, content_type: Option<&str>) -> Result<(), ClipmError> {
    let conn = db::open()?;
    let entries = db::list(&conn, limit, offset, label, days, content_type)?;
    if entries.is_empty() {
        println!("No entries in clipboard history.");
        return Ok(());
    }
    let rows: Vec<ClipRow> = entries.iter().map(entry_to_row).collect();
    println!("{}", Table::new(rows));
    Ok(())
}

pub fn label(id: i64, label: Option<String>) -> Result<(), ClipmError> {
    let conn = db::open()?;
    db::update_label(&conn, id, label.as_deref())?;
    match &label {
        Some(l) => println!("Entry #{id} labeled \"{l}\"."),
        None => println!("Label removed from entry #{id}."),
    }
    Ok(())
}

pub fn search(query: &str, limit: usize, days: Option<u32>, content_type: Option<&str>) -> Result<(), ClipmError> {
    let conn = db::open()?;
    let entries = db::search(&conn, query, limit, days, content_type)?;
    if entries.is_empty() {
        println!("No results for \"{query}\".");
        return Ok(());
    }
    let rows: Vec<ClipRow> = entries.iter().map(entry_to_row).collect();
    println!("{}", Table::new(rows));
    Ok(())
}

pub fn delete(id: i64) -> Result<(), ClipmError> {
    let conn = db::open()?;
    db::delete(&conn, id)?;
    println!("Deleted entry #{id}.");
    Ok(())
}

pub fn clear(force: bool) -> Result<(), ClipmError> {
    if !force {
        print!("Delete all clipboard history? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    let conn = db::open()?;
    let count = db::clear(&conn)?;
    println!("Cleared {count} entries.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn test_truncate_newlines_replaced() {
        assert_eq!(truncate("hello\nworld", 20), "hello world");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("日本語テスト", 4), "日本語…");
    }

    #[test]
    fn test_truncate_emoji() {
        assert_eq!(truncate("😀😁😂🤣😃", 3), "😀😁…");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn test_format_timestamp_valid() {
        let ts = "2026-02-17T10:30:00+00:00";
        let result = format_timestamp(ts);
        assert!(result.contains("2026"));
        assert!(result.contains("02"));
        assert!(!result.contains("+00:00"));
    }

    #[test]
    fn test_format_timestamp_invalid_falls_back() {
        let ts = "not-a-timestamp";
        assert_eq!(format_timestamp(ts), "not-a-timestamp");
    }

    #[test]
    fn test_entry_to_row_masks_password() {
        let text_entry = ClipEntry {
            id: 1,
            content: "hello world".to_string(),
            content_type: ContentType::Text,
            byte_size: 11,
            created_at: "2026-02-17T10:00:00Z".to_string(),
            label: None,
        };
        let row = entry_to_row(&text_entry);
        assert_eq!(row.preview, "hello world");

        let password_entry = ClipEntry {
            id: 2,
            content: "my-secret-password".to_string(),
            content_type: ContentType::Password,
            byte_size: 18,
            created_at: "2026-02-17T10:00:00Z".to_string(),
            label: None,
        };
        let row = entry_to_row(&password_entry);
        assert_eq!(row.preview, "********");
    }
}
