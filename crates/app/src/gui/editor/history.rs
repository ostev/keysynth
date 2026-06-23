use circular_buffer::CircularBuffer;
use esp_hal::time::Instant;

use crate::{
    gui::editor::position::{Position, SelectionRange},
    text::{ByteChar, ByteString},
};

/// Maximum number of undo operations retained.
const MAX_UNDO: usize = 15;
/// Maximum number of redo operations retained.
const MAX_REDO: usize = 15;

/// A reversible editing operation stored in the undo history.
#[derive(Clone, Debug)]
pub enum Edit {
    Insert(ByteChar, Position),
    GroupInsert(ByteString, Position),
    Backspace(ByteChar, Position),
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

/// An edit together with the cursor positions needed to restore editor state
/// during undo and redo.
#[derive(Clone, Debug)]
struct TimestampedEdit {
    /// Timestamp is stored for when I later implement edit batching.
    timestamp: Timestamp,
    edit: Edit,
    cursor_before: Position,
    cursor_after: Position,
}

/// Fixed-capacity undo/redo history.
///
/// Recording a new edit clears the redo history.
#[derive(Clone)]
pub struct History {
    history: CircularBuffer<MAX_UNDO, TimestampedEdit>,
    redo_history: CircularBuffer<MAX_REDO, TimestampedEdit>,
}

impl History {
    pub const fn new() -> History {
        History {
            history: CircularBuffer::new(),
            redo_history: CircularBuffer::new(),
        }
    }

    /// Records a new edit and invalidates any redo history.
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

    /// Returns the operation to undo the most recent edit, as well as the cursor
    /// position from before it was applied.
    pub fn undo(&mut self) -> Option<(Edit, Position)> {
        let timestamped = self.history.pop_back()?;

        self.redo_history.push_back(timestamped.clone());

        let reverse = match timestamped.edit {
            Edit::Insert(character, position) => Edit::Backspace(
                character,
                Position {
                    line: position.line,
                    column: position.column,
                },
            ),
            Edit::Backspace(character, position) => Edit::Insert(
                character,
                Position {
                    line: position.line,
                    column: position.column.saturating_sub(1),
                },
            ),
            Edit::GroupInsert(text, position) => {
                let range = SelectionRange::new(
                    // Position {
                    //     line: position.line,
                    //     column: position.column,
                    // },
                    position,
                    timestamped.cursor_after,
                );
                Edit::GroupDelete(text, range)
            }
            Edit::GroupDelete(text, range) => Edit::GroupInsert(text, range.start()),
        };

        Some((reverse, timestamped.cursor_before))
    }

    /// Reapplies the most recently undone edit, returning the cursor position
    /// from after the original edit.
    pub fn redo(&mut self) -> Option<(Edit, Position)> {
        let timestamped = self.redo_history.pop_back()?;

        self.history.push_back(timestamped.clone());

        Some((timestamped.edit, timestamped.cursor_after))
    }
}
