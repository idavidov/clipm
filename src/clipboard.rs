use arboard::Clipboard;
use crate::models::ClipmError;

pub fn read_text() -> Result<String, ClipmError> {
    let mut cb = Clipboard::new()
        .map_err(|e| ClipmError::Clipboard(e.to_string()))?;
    let text = cb.get_text()
        .map_err(|e| ClipmError::Clipboard(e.to_string()))?;
    if text.is_empty() {
        return Err(ClipmError::EmptyClipboard);
    }
    Ok(text)
}

pub fn write_text(text: &str) -> Result<(), ClipmError> {
    let mut cb = Clipboard::new()
        .map_err(|e| ClipmError::Clipboard(e.to_string()))?;
    cb.set_text(text)
        .map_err(|e| ClipmError::Clipboard(e.to_string()))?;
    Ok(())
}
