use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoFont,
    pixelcolor::Rgb565,
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
};
use esp_println::println;
use serde::{Deserialize, Serialize};

use crate::{gui::colors, text};

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
pub struct Selection {
    anchor: Option<Position>,
}

impl Selection {
    pub fn new(anchor: Option<Position>) -> Selection {
        Selection { anchor }
    }

    pub fn with_cursor(self, cursor: Position) -> Selection {
        match self.anchor {
            Some(anchor) => {
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

    pub fn draw<T: DrawTarget<Color = Rgb565>>(
        &self,
        font: &MonoFont,
        line_start: Point,
        target: &mut T,
    ) -> Result<(), T::Error> {
        const CORNER_RADIUS: u32 = 2;

        let width = self.end.column - self.start.column;

        let rounded = RoundedRectangle::new(
            Rectangle::new(
                line_start + Point::new(text::width(font, self.start.column as u32) as i32, 0),
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
