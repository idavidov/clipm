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
    #[tabled(rename = "Type")]
    content_type: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Label")]
    label: String,
    #[tabled(rename = "Created")]
    created_at: String,
}

fn truncate(s: &str, max_len: usize) -> String {
    let single_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if single_line.len() <= max_len {
        single_line
    } else {
        format!("{}…", &single_line[..max_len - 1])
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

fn entry_to_row(e: &ClipEntry) -> ClipRow {
    ClipRow {
        id: e.id,
        preview: truncate(&e.content, 60),
        content_type: e.content_type.to_string(),
        size: format_size(e.byte_size),
        label: e.label.clone().unwrap_or_default(),
        created_at: e.created_at.clone(),
    }
}

pub fn store(label: Option<String>) -> Result<(), ClipmError> {
    let content = clipboard::read_text()?;
    let conn = db::open()?;

    if db::is_duplicate(&conn, &content)? {
        println!("Skipped: content matches most recent entry.");
        return Ok(());
    }

    let entry = ClipEntry {
        id: 0,
        byte_size: content.len(),
        content_type: ContentType::Text,
        created_at: chrono::Utc::now().to_rfc3339(),
        label,
        content,
    };
    let id = db::insert(&conn, &entry)?;
    println!("Stored as entry #{id} ({}).", format_size(entry.byte_size));
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

pub fn list(limit: usize, offset: usize) -> Result<(), ClipmError> {
    let conn = db::open()?;
    let entries = db::list(&conn, limit, offset)?;
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

pub fn search(query: &str, limit: usize) -> Result<(), ClipmError> {
    let conn = db::open()?;
    let entries = db::search(&conn, query, limit)?;
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
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
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
