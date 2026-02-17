use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Text,
    Password,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentType::Text => write!(f, "text"),
            ContentType::Password => write!(f, "password"),
        }
    }
}

impl std::str::FromStr for ContentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(ContentType::Text),
            "password" => Ok(ContentType::Password),
            _ => Err(format!("Invalid content type: {s}. Must be 'text' or 'password'.")),
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
    Io(String),
    NotFound(String),
    InvalidInput(String),
    EmptyClipboard,
}

impl fmt::Display for ClipmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipmError::Clipboard(msg) => write!(f, "Clipboard error: {msg}"),
            ClipmError::Database(msg) => write!(f, "Database error: {msg}"),
            ClipmError::Io(msg) => write!(f, "I/O error: {msg}"),
            ClipmError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ClipmError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
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

impl From<arboard::Error> for ClipmError {
    fn from(e: arboard::Error) -> Self {
        ClipmError::Clipboard(e.to_string())
    }
}

impl From<std::io::Error> for ClipmError {
    fn from(e: std::io::Error) -> Self {
        ClipmError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_display() {
        assert_eq!(ContentType::Text.to_string(), "text");
        assert_eq!(ContentType::Password.to_string(), "password");
    }

    #[test]
    fn test_content_type_from_str() {
        assert!(matches!("text".parse::<ContentType>(), Ok(ContentType::Text)));
        assert!(matches!("password".parse::<ContentType>(), Ok(ContentType::Password)));
        assert!("unknown".parse::<ContentType>().is_err());
    }

    #[test]
    fn test_content_type_partial_eq() {
        assert_eq!(ContentType::Text, ContentType::Text);
        assert_eq!(ContentType::Password, ContentType::Password);
        assert_ne!(ContentType::Text, ContentType::Password);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(ClipmError::EmptyClipboard.to_string(), "Clipboard is empty");
        assert_eq!(
            ClipmError::NotFound("no entry".into()).to_string(),
            "Not found: no entry"
        );
        assert_eq!(
            ClipmError::InvalidInput("bad".into()).to_string(),
            "Invalid input: bad"
        );
    }
}
