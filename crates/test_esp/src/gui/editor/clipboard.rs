use core::fmt::Display;

use crate::text::{ByteStr, ByteString};

pub struct Clipboard {
    text: ByteString,
}

pub const MAX_SIZE: usize = 1024;

impl Clipboard {
    pub fn new() -> Self {
        Self {
            text: ByteString::new(),
        }
    }

    pub fn set(&mut self, text: ByteString) -> Result<(), ClipboardError> {
        if text.len() > MAX_SIZE {
            Err(ClipboardError::TextTooLarge)
        } else {
            self.text = text;

            Ok(())
        }
    }

    pub fn get(&self) -> &ByteStr {
        &self.text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    TextTooLarge,
}

impl ClipboardError {
    pub fn as_str(self) -> &'static str {
        match self {
            ClipboardError::TextTooLarge => "You can't copy that much text!",
        }
    }
}
