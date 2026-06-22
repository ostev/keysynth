use core::{ops::Range, str::Utf8Error};

use crate::text::{ByteChar, ByteStr};

type Cursor = usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SelectionRange {
    start: Cursor,
    end: Cursor,
}

impl SelectionRange {
    pub const fn new(range: Range<Cursor>) -> SelectionRange {
        assert!(
            range.start <= range.end,
            "The range start must be less than its end!",
        );

        SelectionRange {
            start: range.start,
            end: range.end,
        }
    }

    pub fn new_unchecked(range: Range<Cursor>) -> SelectionRange {
        SelectionRange {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GapBuffer<const N: usize> {
    buffer: [ByteChar; N],
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

    pub fn try_forward_delete(&mut self) -> Result<(), Error> {
        if self.cursor < self.length {
            self.length -= 1;

            Ok(())
        } else {
            Err(Error::OutOfBoundsCursor)
        }
    }

    pub fn try_delete(&mut self, range: SelectionRange) -> Result<(), Error> {
        self.check_cursor(range.end)?;
        self.set_cursor(range.start)?;

        let count = range.end - range.start;

        // Expand the gap to swallow the text---no need to set them to 0
        // as they'll be overwritten later.
        self.length -= count;

        Ok(())
    }

    pub fn try_insert_many(&mut self, text: &ByteStr) -> Result<(), Error> {
        self.increase_length(text.len())?;

        let end_cursor = self.cursor + text.len();

        self.buffer[self.cursor..end_cursor].copy_from_slice(text);

        self.cursor = end_cursor;

        Ok(())
    }

    pub fn iter(&self) -> Iter<'_> {
        let gap_size = self.gap_size();
        Iter {
            left: self.buffer[..self.cursor].iter(),
            right: self.buffer[self.cursor + gap_size..].iter(),
        }
    }

    // /// Converts the buffer into a null-terminated array of `ByteChar`s
    // pub fn to_vec(&self) -> heapless::Vec<N> {
    //     let mut out = hea;
    //     let gap_size = self.gap_size();

    //     // Copy the text before the cursor
    //     out[..self.cursor].copy_from_slice(&self.buffer[..self.cursor]);

    //     // Copy the text after the cursor
    //     out[self.cursor..self.length].copy_from_slice(&self.buffer[self.cursor + gap_size..]);

    //     out
    // }

    pub fn to_vec(&self) -> heapless::Vec<ByteChar, N> {
        self.iter().copied().collect()
    }

    pub fn to_string(&self) -> Result<heapless::String<N>, Utf8Error> {
        heapless::String::from_utf8(self.to_vec())
    }

    pub unsafe fn to_string_unchecked(&self) -> heapless::String<N> {
        unsafe { heapless::String::from_utf8_unchecked(self.to_vec()) }
    }
}

pub enum Error {
    OutOfBoundsCursor,
    CannotExpandBeyondCapacity,
}

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
