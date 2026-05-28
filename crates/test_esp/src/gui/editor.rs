use alloc::{boxed::Box, string::String, vec::Vec};
use embedded_gui::{Reactive, signal::SignalRef};
use enumflags2::BitFlags;
use keyboard_protocol::{Key, KeyboardStatus, Modifier, StandardKey};

use crate::gui::event::{Event, KeyEvent};

#[derive(Reactive)]

pub struct Editor<'model> {
    pub source: SignalRef<'model, Source>,
    pub cursor: SignalRef<'model, Position>,
    pub selection: SignalRef<'model, Option<SelectionRange>>,
    pub clipboard: SignalRef<'model, Clipboard>,
}

pub struct Position {
    pub line: usize,
    pub column: usize,
}

pub struct Clipboard {
    registers: [String; 10],
}

pub struct Source {
    lines: Vec<String>,
    name: String,
}

pub struct SelectionRange {
    pub start: usize,
    pub end: usize,
}

pub enum Edit {
    Insert(char),
    Delete,
}

pub enum Msg {
    // Insertion
    MoveInsert(Direction),
    JumpInsert(Direction),

    // Editing
    PerformEdit(Edit),

    // Selection
    MoveSelection(Direction),
    JumpSelection(Direction),
    SelectAll,
    SelectLine,

    // Clipboard
    Copy,
    Cut,
    Paste,

    // History
    Undo,
    Redo,

    // Saving
    Save,

    NoOp,
}

impl Msg {
    fn from_event(event: Event, keyboard: KeyboardStatus) -> Msg {
        let is_shift = keyboard.modifier_bitfield.contains(Modifier::LeftShift)
            | keyboard.modifier_bitfield.contains(Modifier::RightShift);
        let is_super = keyboard.modifier_bitfield.contains(Modifier::LeftSuper)
            | keyboard.modifier_bitfield.contains(Modifier::RightSuper);

        match event {
            Event::Key(key_event) => match key_event {
                KeyEvent::Pressed(key) => match key {
                    Key::Standard(standard) => {
                        if is_super {
                            if is_shift {
                                match standard {
                                    // Selection
                                    StandardKey::Left => Msg::JumpSelection(Direction::Left),
                                    StandardKey::Right => Msg::JumpSelection(Direction::Right),
                                    StandardKey::Up => Msg::JumpSelection(Direction::Up),
                                    StandardKey::Down => Msg::JumpSelection(Direction::Down),

                                    // History
                                    StandardKey::Z => Msg::Redo,

                                    _ => Msg::NoOp,
                                }
                            } else {
                                match standard {
                                    // Clipboard
                                    StandardKey::C => Msg::Copy,
                                    StandardKey::X => Msg::Cut,
                                    StandardKey::V => Msg::Paste,

                                    // Selection
                                    StandardKey::A => Msg::SelectAll,

                                    // Cursor
                                    StandardKey::Left => Msg::JumpInsert(Direction::Left),
                                    StandardKey::Right => Msg::JumpInsert(Direction::Right),
                                    StandardKey::Down => Msg::JumpInsert(Direction::Down),
                                    StandardKey::Up => Msg::JumpInsert(Direction::Up),

                                    // History
                                    StandardKey::Z => Msg::Undo,

                                    // History
                                    _ => Msg::NoOp,
                                }
                            }
                        } else {
                            if is_shift {
                                match standard {
                                    // Selection
                                    StandardKey::Left => Msg::MoveSelection(Direction::Left),
                                    StandardKey::Right => Msg::MoveSelection(Direction::Right),
                                    StandardKey::Down => Msg::MoveSelection(Direction::Down),
                                    StandardKey::Up => Msg::MoveSelection(Direction::Up),

                                    // Insertion
                                    _ => {
                                        if let Some(character) = standard.to_char(true) {
                                            Msg::PerformEdit(Edit::Insert(character))
                                        } else {
                                            Msg::NoOp
                                        }
                                    }
                                }
                            } else {
                                match standard {
                                    // Selection
                                    StandardKey::Left => Msg::MoveInsert(Direction::Left),
                                    StandardKey::Right => Msg::MoveInsert(Direction::Right),
                                    StandardKey::Down => Msg::MoveInsert(Direction::Down),
                                    StandardKey::Up => Msg::MoveInsert(Direction::Up),

                                    // Insertion
                                    _ => {
                                        if let Some(character) = standard.to_char(false) {
                                            Msg::PerformEdit(Edit::Insert(character))
                                        } else {
                                            Msg::NoOp
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Key::Modifier(_) => Msg::NoOp,
                    Key::Special(_) => Msg::NoOp,
                },
                KeyEvent::Released(_) => Msg::NoOp,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
