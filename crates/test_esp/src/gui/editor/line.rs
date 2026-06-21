use core::convert::Infallible;

use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    primitives::Rectangle,
    text::renderer::TextRenderer,
};
use embedded_gui::{
    layout::IntrinsicSize,
    primitive::Primitive,
    signal::{Reactive, SignalRef},
    size::Size,
};
use keyboard_protocol::{Key, Modifier, StandardKey};

use crate::{
    gui::{
        background::draw_background,
        colors, display,
        editor::{cursor::draw_cursor, gap_buffer::GapBuffer},
    },
    input::event::{Event, KeyEvent},
    text::{self, ByteChar, FixedByteString},
};

const FONT: MonoFont = FONT_10X20;
const PADDING: i32 = 4;

pub struct State<const N: usize> {
    buffer: GapBuffer<N>,
}

impl<const N: usize> State<N> {
    pub const fn new() -> Self {
        Self {
            buffer: GapBuffer::new(),
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::MoveInsert(direction) => {
                let cursor = self.buffer.cursor();
                let _ = match direction {
                    MovementDirection::Left => self.buffer.set_cursor(cursor.saturating_sub(1)),
                    MovementDirection::Right => {
                        self.buffer.set_cursor((cursor + 1).min(self.buffer.len()))
                    }
                };
            }
            Msg::JumpInsert(direction) => {
                let _ = match direction {
                    MovementDirection::Left => self.buffer.set_cursor(0),
                    MovementDirection::Right => self.buffer.set_cursor(self.buffer.len()),
                };
            }

            Msg::Insert(character) => {
                let _ = self.buffer.insert(character);
            }
            Msg::Backspace => {
                self.buffer.backspace();
            }
            Msg::ForwardDelete => {
                // If this fails, then we've reached the end of the line.
                let _ = self.buffer.try_forward_delete();
            }

            Msg::NoOp => {}
        };
    }

    pub fn as_fixed_byte_string(&self) -> FixedByteString<N> {
        self.buffer.to_vec()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementDirection {
    Left,
    Right,
}

#[derive(Reactive)]
pub struct LineEditor<'a, const N: usize> {
    state: SignalRef<'a, State<N>>,
}

impl<'a, const N: usize> LineEditor<'a, N> {
    pub const fn new(state: SignalRef<'a, State<N>>) -> Self {
        Self { state }
    }
}

impl<'a, const N: usize> IntrinsicSize for LineEditor<'a, N> {
    fn intrinsic_size(&self) -> Size {
        Size::new(
            text::width(&FONT, N as u32) as u16 + 2 * PADDING as u16,
            FONT.character_size.height as u16 + 2 * PADDING as u16,
        )
    }
}

impl<'a, const N: usize> Primitive<display::Driver> for LineEditor<'a, N> {
    fn draw(
        &self,
        target: &mut embedded_gui::draw::LocalTarget<display::Driver>,
    ) -> Result<(), Infallible> {
        draw_background(
            Rectangle::new(
                embedded_graphics::geometry::Point::zero(),
                self.intrinsic_size().into(),
            ),
            target,
        );

        const STYLE: MonoTextStyle<'static, Rgb565> = MonoTextStyle::new(&FONT, colors::TEXT);

        STYLE.draw_string(
            &unsafe { self.state.buffer.to_string_unchecked() },
            embedded_graphics::geometry::Point::new(PADDING, PADDING),
            embedded_graphics::text::Baseline::Top,
            target,
        )?;

        draw_cursor(
            &FONT,
            embedded_graphics::geometry::Point::new(
                self.state.buffer.cursor() as i32 * FONT.character_size.width as i32,
                PADDING,
            ),
            target,
        );

        Ok(())
    }
}

pub enum Msg {
    // Insertion
    MoveInsert(MovementDirection),
    JumpInsert(MovementDirection),

    // Editing
    Insert(ByteChar),
    Backspace,
    ForwardDelete,

    NoOp,
}

impl Msg {
    pub fn from_event(event: Event) -> Msg {
        match event {
            Event::Key { event, keyboard } => {
                let is_shift = keyboard.modifier_bitfield.contains(Modifier::LeftShift)
                    | keyboard.modifier_bitfield.contains(Modifier::RightShift);
                let is_super = keyboard.modifier_bitfield.contains(Modifier::LeftSuper)
                    | keyboard.modifier_bitfield.contains(Modifier::RightSuper);

                match event {
                    KeyEvent::Pressed(key) => match key {
                        Key::Standard(standard) => {
                            match standard {
                                // Jump cursor
                                StandardKey::Left if is_super => {
                                    Msg::JumpInsert(MovementDirection::Left)
                                }
                                StandardKey::Right if is_super => {
                                    Msg::JumpInsert(MovementDirection::Right)
                                }

                                // Move cursor
                                StandardKey::Left => Msg::MoveInsert(MovementDirection::Left),
                                StandardKey::Right => Msg::MoveInsert(MovementDirection::Right),

                                // Deletion
                                StandardKey::Backspace => Msg::Backspace,
                                StandardKey::Delete => Msg::ForwardDelete,

                                _ => {
                                    if let Some(character) = standard.to_char(is_shift) {
                                        Msg::Insert(character)
                                    } else {
                                        Msg::NoOp
                                    }
                                }
                            }
                        }
                        Key::Modifier(_) => Msg::NoOp,
                        Key::Special(_) => Msg::NoOp,
                    },
                    KeyEvent::Released(_) => Msg::NoOp,
                }
            }
            _ => Msg::NoOp,
        }
    }
}
