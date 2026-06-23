use crate::text::{ByteStr, ByteString};

/// Clipboard used by the text editor.
#[derive(Clone)]
pub struct Clipboard {
    text: ByteString,
}

/// Maximum amount of text that can be stored in the clipboard.
pub const MAX_SIZE: usize = 1024;

impl Clipboard {
    pub fn new() -> Self {
        Self {
            text: ByteString::new(),
        }
    }

    /// Replaces the current clipboard contents.
    pub fn set(&mut self, text: ByteString) -> Result<(), ClipboardError> {
        if text.len() > MAX_SIZE {
            Err(ClipboardError::TextTooLarge)
        } else {
            self.text = text;

            Ok(())
        }
    }

    /// Returns the current clipboard contents.
    pub fn get(&self) -> &ByteStr {
        &self.text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    TextTooLarge,
}

impl ClipboardError {
    /// Returns a user-facing description of the error.
    pub fn as_str(self) -> &'static str {
        match self {
            ClipboardError::TextTooLarge => "You can't copy that much text!",
        }
    }
}
