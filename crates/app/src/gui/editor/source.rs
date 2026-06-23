use core::ops::Index;

use alloc::vec;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::{
    gui::editor::{
        history::Edit,
        position::{Position, SelectionRange},
    },
    text::{ByteChar, ByteStr, ByteString, Name},
};

/// Each program can be a maximum of 9KB
pub const MAX_SIZE: usize = 3 * 1024;

/// Editable text stored as a collection of lines.
///
/// Newline characters are represented implicitly by the separation between
/// entries in `lines`.
#[derive(Clone)]
pub struct Source {
    pub name: Name,

    /// Individual lines of the document, excluding trailing newline characters.
    /// The document always contains at least one line.
    ///
    // TODO: replace with a gap buffer for better insert performance in future.
    pub lines: Vec<ByteString>,
}

impl Source {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            lines: vec![ByteString::new()],
        }
    }

    /// Inserts a character and returns the updated cursor position.
    pub fn insert(&mut self, character: ByteChar, position: Position) -> Position {
        match character {
            b'\n' => {
                // Split the current line at the cursor and move the trailing text onto a new line.
                let tail = self.lines[position.line].split_off(position.column);
                self.lines.insert(position.line + 1, tail);

                position.advance_newline()
            }
            _ => {
                let line = &mut self.lines[position.line];

                line.insert(position.column, character);
                position.advance()
            }
        }
    }

    /// Backspaces at the specified character at the provided cursor position.
    /// If the cursor is at the start of the line, that line gets merged with
    /// the previous.
    pub fn delete(&mut self, position: Position) -> Position {
        if position.column == 0 {
            if position.line > 0 {
                let removed_line = self.lines.remove(position.line);

                let new_line_index = position.line.saturating_sub(1);
                let new_line = &mut self.lines[new_line_index];
                // Move the cursor to the point before the merged text
                let column = new_line.len();
                // Merge the removed text
                new_line.extend_from_slice(&removed_line);

                Position {
                    line: new_line_index,
                    column,
                }
            } else {
                position
            }
        } else {
            let line = &mut self.lines[position.line];
            let new_column = position.column - 1;

            line.remove(new_column);

            Position {
                line: position.line,
                column: new_column,
            }
        }
    }

    /// Removes all text before the cursor on the current line.
    pub fn backspace_line(&mut self, position: Position) -> ByteString {
        let line = &mut self.lines[position.line];
        let mut removed = line.split_off(position.column);
        core::mem::swap(line, &mut removed);

        removed
    }

    /// Removes all text after the cursor on the current line.
    pub fn forward_delete_line(&mut self, position: Position) -> ByteString {
        self.lines[position.line].split_off(position.column)
    }

    /// Inserts a sequence of characters as a single editing operation.
    pub fn group_insert(&mut self, text: &ByteStr, start: Position) -> Position {
        text.iter().fold(start, |position, character| {
            self.insert(*character, position)
        })
    }
    /// Deletes a range of characters as a single editing operation.
    pub fn group_delete(&mut self, range: SelectionRange) -> Position {
        let start = range.start();
        let mut end = range.end();

        while end != start {
            if end.column > 0 {
                end.column -= 1;
            } else {
                end.line -= 1;
                end.column = self.lines[end.line].len().saturating_sub(1);
            }

            self.delete(end);
        }

        start
    }

    /// Applies an edit to the file and returns the new cursor position.
    pub fn apply(&mut self, edit: Edit) -> Position {
        match edit {
            Edit::Insert(character, position) => self.insert(character, position),
            Edit::Delete(_, position) => self.delete(position),

            Edit::GroupInsert(text, start) => self.group_insert(&text, start),
            Edit::GroupDelete(_, range) => self.group_delete(range),
        }
    }

    /// Serializes the file into the provided buffer.
    pub fn serialize<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], postcard::Error> {
        postcard::to_slice(&SerializedSource::from(self), buffer).map(|bytes| &*bytes)
    }

    /// Deserializes a previously serialized file.
    pub fn deserialize(name: Name, serialized: &[u8]) -> Result<Source, postcard::Error> {
        let source: SerializedSource = postcard::from_bytes(serialized)?;

        Ok(Source::from_serialized(name, source))
    }

    fn from_serialized(name: Name, serialized: SerializedSource) -> Source {
        Source {
            name,
            // Split the string buffer on newlines
            lines: serialized
                .text
                .split(|&character| character == b'\n')
                .map(ByteString::from)
                .collect(),
        }
    }

    /// Returns the text contained within the specified selection range.
    pub fn get_range(&self, range: SelectionRange) -> ByteString {
        let start = range.start();
        let end = range.end();

        if start.line == end.line {
            self.lines[start.line][start.column..end.column].into()
        } else {
            let mut text = ByteString::new();
            text.extend_from_slice(&self.lines[start.line][start.column..]);
            // Preserve line breaks between copied lines:
            text.push(b'\n');

            for line in self.lines[start.line + 1..end.line].iter() {
                text.extend_from_slice(line);
                text.push(b'\n');
            }

            text.extend_from_slice(&self.lines[end.line][..end.column]);

            text
        }
    }

    pub fn to_byte_string(&self) -> ByteString {
        SerializedSource::from(self).text
    }

    /// # Safety
    ///
    /// Every line in the source must be valid UTF-8 bytes. It should be ASCII, so this is fine
    /// in theory.
    pub unsafe fn to_string_unchecked(&self) -> alloc::string::String {
        unsafe { alloc::string::String::from_utf8_unchecked(self.to_byte_string()) }
    }
}

impl Index<Position> for Source {
    type Output = ByteChar;

    fn index(&self, position: Position) -> &Self::Output {
        &self.lines[position.line][position.column]
    }
}

#[derive(Deserialize, Serialize)]
struct SerializedSource {
    text: ByteString,
}

impl From<&Source> for SerializedSource {
    fn from(source: &Source) -> Self {
        let text = source
            .lines
            .iter()
            .fold(ByteString::new(), |mut text: ByteString, line| {
                text.extend_from_slice(line);
                text.push(b'\n');
                text
            });

        Self { text }
    }
}
