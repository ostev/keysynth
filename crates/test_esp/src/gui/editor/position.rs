use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoFont,
    pixelcolor::Rgb565,
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
};
use serde::{Deserialize, Serialize};

use crate::{gui::colors, text};

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

    pub fn draw<T: DrawTarget<Color = Rgb565>>(
        &self,
        font: &MonoFont,
        line_start: Point,
        target: &mut T,
    ) -> Result<(), T::Error> {
        let rounded = RoundedRectangle::new(
            Rectangle::new(
                line_start + Point::new(text::width(font, self.start.column as u32) as i32, 0),
                embedded_graphics::geometry::Size::new(
                    text::width(font, self.end.column as u32),
                    font.character_size.height,
                ),
            ),
            CornerRadii::new(embedded_graphics::geometry::Size::new(2, 2)),
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
