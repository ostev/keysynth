use crate::note::Note;

/// Represents a key's index. Indices start at the top-left of the physical QWERTY keyboard
/// and increase from left-to-right and top-to-bottom. For example, in a QWERTY 84-key keyboard, escape is
/// (0, 0), F1 is (0, 1), F2 is (0, 2), etc. Then, in the next row, ~ is (1, 0), 1 is (1, 1), 2 is (1, 2), etc.
pub struct Key {
    pub row: u32,
    pub column: u32,
}

impl Key {
    pub const fn new(row: u32, column: u32) -> Key {
        Key { row, column }
    }
}

/// Calculates the semitone offset represented by a given key.
pub fn semitone_offset_of(key: Key) -> Option<u32> {
    const COLUMN_INTERVAL: u32 = 3;
    const ROW_INTERVAL: u32 = 1;

    Some(key.row * ROW_INTERVAL + key.column * COLUMN_INTERVAL)
}

/// Calculates the note represented by a given key offset from a reference
/// frequency.
pub fn note_of(key: Key, reference: Note) -> Option<Note> {
    semitone_offset_of(key).map(|semitones| Note::from_semitones(semitones as f32, reference))
}
