use circular_buffer::CircularBuffer;
use esp_hal::time::Instant;

use crate::{
    gui::editor::position::Position,
    text::{ByteChar, ByteString},
};

const MAX_UNDO: usize = 10;
const MAX_REDO: usize = 5;

#[derive(Clone, Debug)]
pub enum Edit {
    Insert(ByteChar, Position),
    GroupInsert(ByteString, Position),
    Delete(ByteChar, Position),
    GroupDelete(ByteString, Position),
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
}

#[derive(Clone)]
pub struct History {
    history: CircularBuffer<MAX_UNDO, TimestampedEdit>,
    redo_history: CircularBuffer<MAX_REDO, TimestampedEdit>,
}

pub const BATCH_DURATION_MS: u32 = 2 * 1000;

impl History {
    pub fn new() -> History {
        History {
            history: CircularBuffer::new(),
            redo_history: CircularBuffer::new(),
        }
    }

    pub fn push(&mut self, edit: Edit) {
        let timestamp = Timestamp::now();

        self.history.push_back(TimestampedEdit { timestamp, edit });
        self.redo_history.clear();
    }

    pub fn undo(&mut self) -> Option<Edit> {
        let timestamped = self.history.pop_back()?;

        self.redo_history.push_back(timestamped.clone());

        let reverse = match timestamped.edit {
            Edit::Insert(character, position) => Edit::Delete(character, position),
            Edit::Delete(character, position) => Edit::Insert(character, position),
            Edit::GroupInsert(text, position) => Edit::GroupDelete(text, position),
            Edit::GroupDelete(text, position) => Edit::GroupInsert(text, position),
        };

        Some(reverse)
    }

    pub fn redo(&mut self) -> Option<Edit> {
        let timestamped = self.redo_history.pop_back()?;

        self.history.push_back(timestamped.clone());

        Some(timestamped.edit)
    }
}
