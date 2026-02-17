use arboard::Clipboard;
use crate::models::ClipmError;

pub fn read_text() -> Result<String, ClipmError> {
    let mut cb = Clipboard::new()?;
    let text = cb.get_text()?;
    if text.is_empty() {
        return Err(ClipmError::EmptyClipboard);
    }
    Ok(text)
}

pub fn write_text(text: &str) -> Result<(), ClipmError> {
    let mut cb = Clipboard::new()?;
    cb.set_text(text)?;
    Ok(())
}
