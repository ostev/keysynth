use crate::note::Note;

/// Represents a key's index. Indices start at the top-left of the physical QWERTY keyboard
/// and increase from left-to-right and top-to-bottom. For example, in a QWERTY 84-key keyboard, escape is
/// (0, 0), F1 is (0, 1), F2 is (0, 2), etc. Then, in the next row, ~ is (1, 0), 1 is (1, 1), 2 is (1, 2), etc.
pub struct Key {
    row: u32,
    column: u32,
}

impl Key {
    pub const fn new(row: u32, column: u32) -> Key {
        Key { row, column }
    }
}

pub struct Keyboard {
    offset: (u32, u32),
    size: (u32, u32),
}

impl Keyboard {
    pub fn new(offset: (u32, u32), size: (u32, u32)) -> Option<Keyboard> {
        if size.0 == 0 || size.1 == 0 {
            None
        } else {
            Some(Keyboard { offset, size })
        }
    }

    pub fn semitone_offset_of(&self, key: Key) -> Option<u32> {
        let (width, height) = self.size;
        let (row_offset, column_offset) = self.offset;

        let translated_key = {
            let row = key.row.checked_sub(row_offset)?;
            let column = key.column.checked_sub(column_offset)?;

            if row >= height || column >= width {
                None
            } else {
                Some(Key::new(row, column))
            }
        }?;

        Some(translated_key.row + translated_key.column * (height - 1))
    }

    pub fn note_of(&self, key: Key, reference: Note) -> Option<Note> {
        self.semitone_offset_of(key)
            .map(|semitones| Note::from_semitones(semitones as f32, reference))
    }
}
