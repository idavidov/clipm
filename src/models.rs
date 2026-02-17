use std::fmt;

#[derive(Debug, Clone)]
pub enum ContentType {
    Text,
    Html,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentType::Text => write!(f, "text"),
            ContentType::Html => write!(f, "html"),
        }
    }
}

impl ContentType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "html" => ContentType::Html,
            _ => ContentType::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipEntry {
    pub id: i64,
    pub content: String,
    pub content_type: ContentType,
    pub byte_size: usize,
    pub created_at: String,
    pub label: Option<String>,
}

#[derive(Debug)]
pub enum ClipmError {
    Clipboard(String),
    Database(String),
    NotFound(String),
    EmptyClipboard,
}

impl fmt::Display for ClipmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipmError::Clipboard(msg) => write!(f, "Clipboard error: {msg}"),
            ClipmError::Database(msg) => write!(f, "Database error: {msg}"),
            ClipmError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ClipmError::EmptyClipboard => write!(f, "Clipboard is empty"),
        }
    }
}

impl std::error::Error for ClipmError {}

impl From<rusqlite::Error> for ClipmError {
    fn from(e: rusqlite::Error) -> Self {
        ClipmError::Database(e.to_string())
    }
}
