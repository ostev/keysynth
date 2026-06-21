use circular_buffer::CircularBuffer;
use esp_hal::time::Instant;
use esp_println::println;

use crate::{
    gui::editor::position::{Position, SelectionRange},
    text::{ByteChar, ByteString},
};

const MAX_UNDO: usize = 10;
const MAX_REDO: usize = 5;

#[derive(Clone, Debug)]
pub enum Edit {
    Insert(ByteChar, Position),
    GroupInsert(ByteString, Position),
    Delete(ByteChar, Position),
    GroupDelete(ByteString, SelectionRange),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    ms: u32,
}

impl Timestamp {
    pub fn now() -> Timestamp {
        Timestamp {
            ms: Instant::now().duration_since_epoch().as_millis() as u32,
        }
    }
}

#[derive(Clone, Debug)]
struct TimestampedEdit {
    timestamp: Timestamp,
    edit: Edit,
    cursor_before: Position,
    cursor_after: Position,
}

#[derive(Clone)]
pub struct History {
    history: CircularBuffer<MAX_UNDO, TimestampedEdit>,
    redo_history: CircularBuffer<MAX_REDO, TimestampedEdit>,
    // /// The fallback cursor for if the user performs an undo operation and there
    // /// was no edit beforehand.
    // starting_cursor: Position,
}

pub const BATCH_DURATION_MS: u32 = 2 * 1000;

impl History {
    pub fn new(starting_cursor: Position) -> History {
        History {
            history: CircularBuffer::new(),
            redo_history: CircularBuffer::new(),
            // starting_cursor,
        }
    }

    pub fn push(&mut self, edit: Edit, cursor_before: Position, cursor_after: Position) {
        let timestamp = Timestamp::now();

        self.history.push_back(TimestampedEdit {
            timestamp,
            edit,
            cursor_before,
            cursor_after,
        });
        self.redo_history.clear();
    }

    pub fn undo(&mut self) -> Option<(Edit, Position)> {
        let timestamped = self.history.pop_back()?;

        self.redo_history.push_back(timestamped.clone());

        let reverse = match timestamped.edit {
            Edit::Insert(character, position) => Edit::Delete(
                character,
                Position {
                    line: position.line,
                    column: position.column,
                },
            ),
            Edit::Delete(character, position) => Edit::Insert(
                character,
                Position {
                    line: position.line,
                    column: position.column.saturating_sub(1),
                },
            ),
            Edit::GroupInsert(text, position) => {
                let range = SelectionRange::from_start_and_text(position, &text);
                Edit::GroupDelete(text, range)
            }
            Edit::GroupDelete(text, range) => Edit::GroupInsert(text, range.start()),
        };

        Some((reverse, timestamped.cursor_before))
    }

    pub fn redo(&mut self) -> Option<(Edit, Position)> {
        let timestamped = self.redo_history.pop_back()?;

        self.history.push_back(timestamped.clone());

        Some((timestamped.edit, timestamped.cursor_after))
    }
}
