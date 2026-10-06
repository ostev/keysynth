use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoFont,
    pixelcolor::Rgb565,
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
};
use serde::{Deserialize, Serialize};

use crate::{
    gui::colors,
    text::{self, ByteChar},
};

/// A cursor position in a source code file, represented as its line
/// and column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const fn new(line: usize, column: usize) -> Position {
        Position { line, column }
    }

    pub const fn zero() -> Position {
        Position { line: 0, column: 0 }
    }

    /// Advances the position by a newline down.
    pub const fn advance_newline(self) -> Position {
        Self {
            line: self.line + 1,
            column: 0,
        }
    }

    /// Advances the position as if the provided character had been inserted.
    pub const fn advance_char(self, character: ByteChar) -> Position {
        match character {
            b'\n' => self.advance_newline(),
            _ => self.advance(),
        }
    }

    /// Advances the position by one character.
    pub const fn advance(self) -> Position {
        Self {
            line: self.line,
            column: self.column + 1,
        }
    }
}

/// Tracks the anchor point of an active selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    anchor: Option<Position>,
}

impl Selection {
    pub const fn new(anchor: Option<Position>) -> Selection {
        Selection { anchor }
    }

    /// Updates the selection for the current cursor position.
    pub fn with_cursor(self, cursor: Position) -> Selection {
        match self.anchor {
            Some(anchor) => {
                // If the cursor is at the same position as the anchor,
                // the selection is cleared.
                if anchor == cursor {
                    Selection { anchor: None }
                } else {
                    self
                }
            }
            None => Selection {
                anchor: Some(cursor),
            },
        }
    }

    pub fn is_active(self) -> bool {
        self.anchor.is_some()
    }

    pub fn clear(&mut self) {
        self.anchor = None;
    }

    pub fn range(self, cursor: Position) -> Option<SelectionRange> {
        self.anchor
            .map(|anchor| SelectionRange::new(anchor, cursor))
    }
}

/// A normalized selection range.
///
/// `start` is always less than or equal to `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    start: Position,
    end: Position,
}

impl SelectionRange {
    pub fn new(start: Position, end: Position) -> SelectionRange {
        // Normalises the range so that start <= end
        if start <= end {
            SelectionRange { start, end }
        } else {
            SelectionRange {
                start: end,
                end: start,
            }
        }
    }

    #[inline]
    pub const fn start(self) -> Position {
        self.start
    }

    #[inline]
    pub const fn end(self) -> Position {
        self.end
    }

    /// Create a selection range from a start position and the provided text.
    // pub fn from_start_and_text(start: Position, text: &ByteStr) -> SelectionRange {
    //     let end = text.iter().fold(start, |position, &character| {
    //         position.advance_char(character)
    //     });

    //     SelectionRange::new(start, end)
    // }

    /// Draws the portion of the selection visible on a single line.
    pub fn draw<T: DrawTarget<Color = Rgb565>>(
        &self,
        font: &MonoFont,
        line_start: Point,
        current_line: usize,
        scroll_x: usize,
        visible_line_length: usize,
        target: &mut T,
    ) -> Result<(), T::Error> {
        const CORNER_RADIUS: u32 = 2;

        let (start_column, width) =
            if current_line > self.start.line && current_line < self.end.line {
                // In between, all the line is selected
                (0, visible_line_length - scroll_x)
            } else {
                match (
                    current_line == self.start.line,
                    current_line == self.end.line,
                ) {
                    (true, true) => {
                        // The selection is just a single line
                        let width = self.end.column - self.start.column - scroll_x;
                        (self.start.column, width.min(visible_line_length))
                    }
                    (true, false) => {
                        // This is the start of the selection
                        (
                            self.start.column,
                            visible_line_length - self.start.column - scroll_x,
                        )
                    }
                    (false, true) => {
                        // This is the end of the selection
                        (0, self.end.column - scroll_x)
                    }
                    (false, false) => {
                        // This line is not selected
                        return Ok(());
                    }
                }
            };

        let rounded = RoundedRectangle::new(
            Rectangle::new(
                line_start + Point::new(text::width(font, start_column as u32) as i32, 0),
                embedded_graphics::geometry::Size::new(
                    text::width(font, width as u32),
                    font.character_size.height,
                ),
            ),
            CornerRadii::new(embedded_graphics::geometry::Size::new(
                CORNER_RADIUS,
                CORNER_RADIUS,
            )),
        );
        rounded.draw_styled(
            &PrimitiveStyleBuilder::new()
                .fill_color(colors::SELECTION)
                .build(),
            target,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
