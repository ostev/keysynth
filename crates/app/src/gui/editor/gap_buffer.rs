use core::str::Utf8Error;

use crate::text::{ByteChar, ByteStr};

/// Cursor column position within the buffer
type Cursor = usize;

/// Fixed-capacity gap buffer used for efficient editing.
///
/// Text before the cursor occupies the beginning of `buffer`, while text after
/// the cursor is stored at the end. The unused space between them forms the
/// gap.
#[derive(Clone, Debug)]
pub struct GapBuffer<const N: usize> {
    buffer: [ByteChar; N],
    /// Cursor position, equal to the start of the gap.
    cursor: Cursor,
    length: usize,
}

impl<const N: usize> GapBuffer<N> {
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            cursor: 0,
            length: 0,
        }
    }

    pub const fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    const fn gap_size(&self) -> usize {
        N - self.length
    }

    const fn check_cursor(&self, cursor: Cursor) -> Result<(), Error> {
        if cursor <= self.length {
            Ok(())
        } else {
            Err(Error::OutOfBoundsCursor)
        }
    }

    const fn increase_length(&mut self, amount: usize) -> Result<(), Error> {
        let new_length = self.length + amount;

        if new_length <= N {
            self.length = new_length;
            Ok(())
        } else {
            Err(Error::CannotExpandBeyondCapacity)
        }
    }

    /// Updates the cursor position and correspondingly moves the gap.
    pub fn set_cursor(&mut self, cursor: Cursor) -> Result<(), Error> {
        self.check_cursor(cursor)?;

        if cursor > self.cursor {
            let distance = cursor - self.cursor;

            let after_gap_start = self.cursor + self.gap_size();
            let after_gap_end = after_gap_start + distance;
            self.buffer
                .copy_within(after_gap_start..after_gap_end, self.cursor);

            self.cursor = cursor;
        } else if cursor < self.cursor {
            let distance = self.cursor - cursor;

            let before_gap_start = cursor;
            let before_gap_end = self.cursor;

            let after_gap_start = self.cursor + self.gap_size() - distance;
            self.buffer
                .copy_within(before_gap_start..before_gap_end, after_gap_start);

            self.cursor = cursor;
        }

        Ok(())
    }

    /// Inserts a character at the cursor.
    pub fn insert(&mut self, character: ByteChar) -> Result<(), Error> {
        self.increase_length(1)?;

        let new_cursor = self.cursor + 1;

        self.buffer[self.cursor] = character;

        self.cursor = new_cursor;

        Ok(())
    }

    pub const fn backspace(&mut self) {
        if self.cursor > 0 {
            // Expand the gap backwards to swallow the letter before it---
            // we don't need to set that letter to 0 as it'll be overwritten
            // later.
            self.buffer[self.cursor - 1] = 0;
            self.cursor -= 1;
            self.length -= 1;
        }
    }

    /// Deletes the character immediately after the cursor.
    pub fn try_forward_delete(&mut self) -> Result<(), Error> {
        if self.cursor < self.length {
            self.length -= 1;

            Ok(())
        } else {
            Err(Error::OutOfBoundsCursor)
        }
    }

    /// Inserts a slice of bytes at the cursor position.
    pub fn try_insert_many(&mut self, text: &ByteStr) -> Result<(), Error> {
        self.increase_length(text.len())?;

        let end_cursor = self.cursor + text.len();

        self.buffer[self.cursor..end_cursor].copy_from_slice(text);

        self.cursor = end_cursor;

        Ok(())
    }

    /// Iterates over the logical contents of the buffer.
    pub fn iter(&self) -> Iter<'_> {
        let gap_size = self.gap_size();
        Iter {
            left: self.buffer[..self.cursor].iter(),
            // Skip the gap
            right: self.buffer[self.cursor + gap_size..].iter(),
        }
    }

    pub fn to_vec(&self) -> heapless::Vec<ByteChar, N> {
        self.iter().copied().collect()
    }

    pub fn to_string(&self) -> Result<heapless::String<N>, Utf8Error> {
        heapless::String::from_utf8(self.to_vec())
    }

    /// # Safety
    ///
    /// The contents of the gap buffer must **always** contain ASCII.
    pub unsafe fn to_string_unchecked(&self) -> heapless::String<N> {
        unsafe { heapless::String::from_utf8_unchecked(self.to_vec()) }
    }
}

pub enum Error {
    OutOfBoundsCursor,
    CannotExpandBeyondCapacity,
}

/// Iterator over the logical contents of a gap buffer.
pub struct Iter<'a> {
    left: core::slice::Iter<'a, ByteChar>,
    right: core::slice::Iter<'a, ByteChar>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a ByteChar;

    fn next(&mut self) -> Option<Self::Item> {
        self.left.next().or_else(|| self.right.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.left.len() + self.right.len();
        (len, Some(len))
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // Start from the right and work back to the front
        self.right.next_back().or_else(|| self.left.next_back())
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a, const N: usize> IntoIterator for &'a GapBuffer<N> {
    type Item = &'a ByteChar;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
