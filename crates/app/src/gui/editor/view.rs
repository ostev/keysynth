use core::fmt::Write;

use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, ascii::FONT_10X20},
    text::renderer::TextRenderer,
};
use embedded_gui::{
    draw::LocalTarget,
    layout::IntrinsicSize,
    primitive::Primitive,
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};

use crate::{
    gui::{
        colors, display,
        editor::{
            cursor::draw_cursor,
            state::{EditorState, LINES_VISIBLE},
        },
    },
    text,
};

/// Renders the visible portion of the source editor, including line numbers,
/// selections and the cursor.
#[derive(Reactive)]
pub struct EditorView<'a> {
    pub state: SignalRef<'a, EditorState>,
    pub is_active: Signal<bool>,
}

impl<'a> IntrinsicSize for EditorView<'a> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::zero()
    }
}

const FONT: MonoFont = FONT_10X20;
/// Maximum number of digits reserved for line numbers.
const GUTTER_MAX_CHARACTERS: u32 = 3;
/// Width of the line number gutter, including trailing spacing.
const GUTTER_WIDTH: u32 = text::width_with_space_after(&FONT, GUTTER_MAX_CHARACTERS);
/// Maximum number of source characters that can be displayed on a single line
/// once the gutter and padding have been accounted for.
const MAX_LINE_LENGTH: u32 =
    text::characters_in(&FONT, display::WIDTH as u32) - GUTTER_MAX_CHARACTERS - 4;

impl<'a> Primitive<display::Driver> for EditorView<'a> {
    fn draw(
        &self,
        target: &mut LocalTarget<display::Driver>,
    ) -> Result<(), <display::Driver as embedded_graphics::prelude::DrawTarget>::Error> {
        let lines = self
            .state
            .source
            .lines
            .iter()
            .enumerate()
            .skip(self.state.scroll_start)
            .take(LINES_VISIBLE);

        let mut y_offset = 0;

        let font_style = MonoTextStyle::new(&FONT, colors::TEXT);

        // Begin horizontally scrolling once the cursor moves beyond the visible width.
        let scroll_x_offset = self
            .state
            .cursor
            .column
            .saturating_sub(MAX_LINE_LENGTH as usize);
        let selection_range = self.state.selection.range(self.state.cursor);

        for (index, line) in lines {
            // Text starts after the gutter
            let line_start = embedded_graphics::geometry::Point::new(
                text::width(&FONT_10X20, GUTTER_MAX_CHARACTERS + 1) as i32,
                y_offset,
            );

            // Selection is drawn before text as it needs to appear behind it
            if let Some(selection) = selection_range {
                let Ok(_) = selection.draw(
                    &FONT,
                    line_start,
                    index,
                    scroll_x_offset,
                    MAX_LINE_LENGTH as usize,
                    target,
                );
            }

            let mut draw = |text: &str, position: embedded_graphics::geometry::Point| {
                let Ok(next) = font_style.draw_string(
                    text,
                    position,
                    embedded_graphics::text::Baseline::Top,
                    target,
                );

                next
            };

            if index >= 999 {
                // We can't display more than 3 digits in the gutter
                draw(
                    "This file is too long and I can't display all of it!",
                    line_start,
                );
            } else {
                let line_number = {
                    let line_number = index + 1;
                    let mut buffer = heapless::String::<3>::new();
                    write!(buffer, "{line_number}").unwrap();

                    buffer
                };

                draw(
                    &line_number,
                    embedded_graphics::geometry::Point::new(0, y_offset),
                );

                if line.len() > scroll_x_offset {
                    draw(
                        // SAFETY: The code is always in ASCII. This means that `scroll_x_offset`
                        // always refers to a character boundary.
                        unsafe { str::from_utf8_unchecked(&line[scroll_x_offset..]) },
                        line_start,
                    );
                }

                if index == self.state.cursor.line {
                    // Draw the cursor only on the active line
                    draw_cursor(
                        &FONT,
                        embedded_graphics::geometry::Point::new(
                            (GUTTER_WIDTH
                                + text::width(
                                    &FONT,
                                    (self.state.cursor.column - scroll_x_offset + 1) as u32,
                                )) as i32,
                            y_offset,
                        ),
                        target,
                    );
                }
            }

            y_offset += FONT_10X20.character_size.height as i32;
        }

        Ok(())
    }
}
