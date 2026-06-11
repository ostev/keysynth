use core::fmt::Write;

use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoFont, MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
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
        editor::{cursor::draw_cursor, state::EditorState},
    },
    text,
};

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
const GUTTER_MAX_CHARACTERS: u32 = 3;
const GUTTER_WIDTH: u32 = text::width_with_space_after(&FONT, GUTTER_MAX_CHARACTERS);
const MAX_LINE_LENGTH: u32 =
    text::characters_in(&FONT, display::SIZE.width as u32) - GUTTER_MAX_CHARACTERS - 4;

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
            .take(self.state.lines_visible);

        let mut y_offset = 0;

        let font_style = MonoTextStyle::new(&FONT, colors::TEXT);

        let scroll_x_offset = self
            .state
            .cursor
            .column
            .saturating_sub(MAX_LINE_LENGTH as usize);

        for (index, line) in lines {
            let line_start = embedded_graphics::geometry::Point::new(
                4 * FONT_10X20.character_size.width as i32,
                y_offset,
            );

            if let Some(selection) = self.state.selection {
                let Ok(_) = selection.draw(&FONT, line_start, target);
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

                draw(
                    unsafe { str::from_utf8_unchecked(&line[scroll_x_offset..]) },
                    line_start,
                );

                if index == self.state.cursor.line {
                    draw_cursor(
                        &FONT,
                        embedded_graphics::geometry::Point::new(
                            (GUTTER_WIDTH + text::width(&FONT, self.state.cursor.column as u32 + 1))
                                as i32,
                            y_offset,
                        ),
                        target,
                    );
                }
            }

            y_offset += FONT_10X20.character_size.height as i32;
            // let scanner =
            //     calc::parser::scanner::Scanner::new(unsafe { str::from_utf8_unchecked(line) });

            // let mut x_offset = 0;

            // for token in scanner {
            //     let (text, color) = match token {
            //         Ok((start, token, end)) => (
            //             0,
            //             match token {
            //                 Token::Newline => colors::TEXT,
            //                 Token::Semicolon => colors::TEXT,
            //                 Token::Identifier(_) => colors::TEXT,
            //                 Token::Number(_) => colors::LITERAL,
            //                 Token::True => colors::KEYWORD,
            //                 Token::False => colors::KEYWORD,
            //                 Token::Fn => colors::KEYWORD,
            //                 Token::LeftParen => colors::TEXT,
            //                 Token::RightParen => colors::TEXT,
            //                 Token::Comma => colors::TEXT,
            //                 Token::Arrow => colors::TEXT,
            //                 Token::Minus => colors::TEXT,
            //                 Token::Plus => colors::TEXT,
            //                 Token::Slash => colors::TEXT,
            //                 Token::Star => colors::TEXT,
            //                 Token::Caret => colors::TEXT,
            //                 Token::Equal => colors::TEXT,
            //                 Token::EqualEqual => colors::TEXT,
            //                 Token::BangEqual => colors::TEXT,
            //                 Token::Greater => colors::TEXT,
            //                 Token::GreaterEqual => colors::TEXT,
            //                 Token::Less => colors::TEXT,
            //                 Token::LessEqual => colors::TEXT,
            //                 Token::If => colors::KEYWORD,
            //                 Token::Then => colors::KEYWORD,
            //                 Token::Else => colors::KEYWORD,
            //                 Token::Let => colors::KEYWORD,
            //                 Token::In => colors::KEYWORD,
            //             },
            //         ),
            //         Err((start, _, end)) => colors::ERROR,
            //     };

            //     MonoTextStyle::new(&FONT_10X20, color);
            // }
        }

        Ok(())
    }
}
