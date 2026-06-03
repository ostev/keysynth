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
    signal::{Reactive, SignalRef},
    size::Size,
};

use crate::gui::{display, editor::state::EditorState};

#[derive(Reactive)]
pub struct EditorView<'a> {
    state: SignalRef<'a, EditorState>,
}

impl<'a> EditorView<'a> {
    pub const fn new(state: SignalRef<'a, EditorState>) -> Self {
        Self { state }
    }
}

impl<'a> IntrinsicSize for EditorView<'a> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::zero()
    }
}

mod colors {
    use embedded_graphics::pixelcolor::{Rgb565, RgbColor};

    // Dracula!!
    pub const ERROR: Rgb565 = Rgb565::new(255, 85, 85);
    pub const TEXT: Rgb565 = Rgb565::new(248, 248, 242);
    pub const LITERAL: Rgb565 = Rgb565::new(255, 184, 108);
    pub const KEYWORD: Rgb565 = Rgb565::new(255, 121, 198);
}

const fn width(characters: u32) -> u32 {
    FONT.character_size.width * characters + FONT.character_spacing * (characters.saturating_sub(1))
}

const fn width_with_space_after(characters: u32) -> u32 {
    (FONT.character_size.width + FONT.character_spacing) * characters
}

const fn characters_in(width: u32) -> u32 {
    width / (FONT.character_size.width + FONT.character_spacing)
}

static FONT: MonoFont = FONT_10X20;
// const CHARACTER_UNIT_WIDTH: u32 = FONT.character_size.width + FONT.character_spacing;
const GUTTER_MAX_CHARACTERS: u32 = 3;
const GUTTER_WIDTH: u32 = width_with_space_after(GUTTER_MAX_CHARACTERS);
const MAX_LINE_LENGTH: u32 = characters_in(display::SIZE.width as u32) - GUTTER_MAX_CHARACTERS;

fn draw_cursor<T: DrawTarget<Color = Rgb565>>(
    position: embedded_graphics::geometry::Point,
    target: &mut T,
) -> Result<(), T::Error> {
    RoundedRectangle::new(
        Rectangle::new(
            position,
            embedded_graphics::geometry::Size::new(4, FONT.character_size.height),
        ),
        CornerRadii::new(embedded_graphics::geometry::Size::new(1, 1)),
    )
    .draw_styled(
        &PrimitiveStyleBuilder::new()
            .fill_color(colors::TEXT)
            .build(),
        target,
    )
}

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
                    embedded_graphics::geometry::Point::new(0, y_offset),
                );
            } else {
                let line_number = {
                    let line_number = index + 1;
                    let mut buffer = heapless::String::<3>::new();
                    write!(buffer, "{line_number}");

                    buffer
                };

                draw(
                    &line_number,
                    embedded_graphics::geometry::Point::new(0, y_offset),
                );

                draw(
                    unsafe { str::from_utf8_unchecked(&line[scroll_x_offset..]) },
                    embedded_graphics::geometry::Point::new(
                        4 * FONT_10X20.character_size.width as i32,
                        y_offset,
                    ),
                );

                if index == self.state.cursor.line {
                    draw_cursor(
                        embedded_graphics::geometry::Point::new(
                            (GUTTER_WIDTH + width(self.state.cursor.column as u32)) as i32,
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
