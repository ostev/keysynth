use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub const fn advance_newline(self) -> Position {
        Self {
            line: self.line + 1,
            column: 0,
        }
    }
    pub const fn retreat_newline(self) -> Position {
        Self {
            line: self.line.saturating_sub(1),
            column: 0,
        }
    }

    pub const fn advance(self) -> Position {
        Self {
            line: self.line,
            column: self.column + 1,
        }
    }
    pub const fn retreat(self) -> Position {
        Self {
            line: self.line,
            column: self.column.saturating_sub(1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: Position,
    pub end: Position,
}

impl SelectionRange {
    pub const fn new(start: Position, end: Position) -> SelectionRange {
        SelectionRange { start, end }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
