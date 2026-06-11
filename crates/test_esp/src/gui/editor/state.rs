use core::cmp;

use alloc::{
    borrow::{Cow, ToOwned},
    format,
};
use embassy_time::{Duration, Instant};
use embedded_gui::{app::Change, size::Size};
use embedded_storage_async::nor_flash::NorFlash;
use esp_println::println;
use keyboard_protocol::{Key, Modifier, StandardKey};

use crate::{
    gui::{
        self,
        editor::{
            clipboard::Clipboard,
            history::{self, History},
            position::{Direction, Position, SelectionRange},
            source::Source,
        },
        effect::Effect,
        event::{Event, KeyEvent},
        message::Message,
    },
    text::{ByteChar, ByteString},
};

pub struct EditorState {
    pub(super) source: Source,
    pub(super) cursor: Position,
    pub(super) selection: Option<SelectionRange>,
    pub(super) clipboard: Clipboard,
    pub(super) history: History,
    pub(super) message: Option<Message>,

    pub(super) scroll_start: usize,
    pub(super) lines_visible: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(Source::default())
    }
}

impl EditorState {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            cursor: Position::new(0, 0),
            selection: None,
            clipboard: Clipboard::new(),
            history: History::new(),
            message: None,

            scroll_start: 0,
            lines_visible: 10,
        }
    }

    pub unsafe fn to_string_unchecked(&self) -> alloc::string::String {
        unsafe { self.source.to_string_unchecked() }
    }

    // fn intrinsic_size(&self) -> Size {
    //     let font = embedded_graphics::mono_font::ascii::FONT_8X13;

    //     let char_width = font.character_size.width as u16;
    //     let line_height = font.character_size.height as u16;

    //     let max_chars = self
    //         .source
    //         .lines
    //         .iter()
    //         // Since we only support ASCII, `len` works fine here as one character
    //         // is always one byte.
    //         .map(|line| line.len() as u16)
    //         .max()
    //         .unwrap_or(0);

    //     Size::new(
    //         max_chars * char_width,
    //         self.source.lines.len() as u16 * line_height,
    //     )
    // }

    pub fn update(&mut self, msg: Msg) -> Change<gui::Msg, gui::FocusKey, gui::Effect> {
        self.clear_message_if_old();

        match msg {
            Msg::MoveInsert(direction) => {
                self.selection = None;
                self.move_cursor(direction, false);
            }
            Msg::JumpInsert(direction) => {
                self.selection = None;
                self.jump_cursor(direction, false);
            }
            Msg::MoveSelection(direction) => {
                self.move_cursor(direction, true);
            }
            Msg::JumpSelection(direction) => {
                self.jump_cursor(direction, true);
            }
            Msg::SelectAll => self.select_all(),
            Msg::SelectLine => self.select_line(),
            Msg::Insert(character) => {
                self.history
                    .push(history::Edit::Insert(character, self.cursor));

                self.cursor = self.source.insert(character, self.cursor);
            }
            Msg::Delete => self.delete(),
            Msg::Copy => self.copy(),
            Msg::Paste => self.paste(),
            Msg::Cut => {
                self.copy();
                self.delete();
            }

            Msg::Undo => {
                if let Some(edit) = self.history.undo() {
                    self.source.apply(edit);
                }
            }
            Msg::Redo => {
                if let Some(edit) = self.history.redo() {
                    self.source.apply(edit);
                }
            }
            Msg::Save => match self.source.name {
                Some(name) => match self.source.serialize() {
                    Ok(serialized) => {
                        return Change::none().with_effect(Effect::Save(name, serialized));
                    }
                    Err(_) => {
                        let text = Cow::Borrowed(
                            "A serialization error occurred while saving! Please try again.",
                        );

                        self.message = Some(Message::now(text))
                    }
                },
                None => todo!(),
            },

            Msg::NoOp => {}
        };

        Change::none()
    }

    fn clear_message_if_old(&mut self) {
        const MESSAGE_DISPLAY_DURATION: Duration = Duration::from_secs(2);

        if let Some(message) = &self.message {
            if message.timestamp.elapsed() > MESSAGE_DISPLAY_DURATION {
                self.message = None;
            }
        }
    }

    fn copy(&mut self) {
        if let Some(selection) = self.selection {
            if let Err(error) = self.clipboard.set(self.source.get_range(selection)) {
                self.message = Some(Message::now(Cow::Borrowed(error.as_str())))
            }
        }
    }

    fn delete(&mut self) {
        println!("Delete!");
        match self.selection {
            Some(range) => {
                let text = self.source.get_range(range);

                let new_cursor = self.source.group_delete(&text, self.cursor);
                self.history
                    .push(history::Edit::GroupDelete(text, self.cursor));

                self.cursor = new_cursor;
            }

            None => {
                let character = self.source.lines[self.cursor.line][self.cursor.column];

                self.history
                    .push(history::Edit::Delete(character, self.cursor));

                self.cursor = self.source.delete(character, self.cursor);
            }
        }
    }

    fn paste(&mut self) {
        let text = self.clipboard.get();

        self.history
            .push(history::Edit::GroupInsert(text.to_owned(), self.cursor));

        self.cursor = self.source.group_insert(text, self.cursor);
    }

    fn move_cursor(&mut self, direction: Direction, selecting: bool) {
        let start = self.cursor;

        match direction {
            Direction::Left => {
                self.cursor.column = self.cursor.column.saturating_sub(1);
            }
            Direction::Right => {
                self.cursor.column = self.cursor.column.saturating_add(1);
            }
            Direction::Up => {
                self.cursor.line = self.cursor.line.saturating_sub(1);
                self.clamp_cursor_column();
            }
            Direction::Down => {
                self.cursor.line = self.cursor.line.saturating_add(1);
                self.clamp_cursor_column();
            }
        }

        self.update_selection(start, selecting);
    }

    fn clamp_cursor_column(&mut self) {
        self.cursor.column = cmp::min(
            self.cursor.column,
            self.source.lines[self.cursor.line].len().saturating_sub(1),
        );
    }

    fn jump_cursor(&mut self, direction: Direction, selecting: bool) {
        let start = self.cursor;

        match direction {
            Direction::Left => self.cursor.column = 0,
            Direction::Right => self.cursor.column = self.source.lines[self.cursor.line].len(),
            Direction::Up => {
                self.cursor.line = 0;
                self.clamp_cursor_column();
            }
            Direction::Down => {
                self.cursor.line = self.source.lines.len().saturating_sub(1);
                self.clamp_cursor_column();
            }
        }

        self.update_selection(start, selecting);
    }

    fn update_selection(&mut self, start: Position, is_selecting: bool) {
        if is_selecting {
            let selection_start = self
                .selection
                .map(|selection| selection.start)
                .unwrap_or(start);

            self.selection = Some(SelectionRange {
                start: selection_start,
                end: self.cursor,
            });
        } else {
            self.selection = None;
        }
    }

    fn select_all(&mut self) {
        let last_line = self.source.lines.len().saturating_sub(1);
        let end_col = self.source.lines[last_line].len();

        self.selection = Some(SelectionRange {
            start: Position::new(0, 0),
            end: Position::new(last_line, end_col),
        });
    }

    fn select_line(&mut self) {
        let line_len = self.source.lines[self.cursor.line].len();

        self.selection = Some(SelectionRange {
            start: Position::new(self.cursor.line, 0),
            end: Position::new(self.cursor.line, line_len),
        });
    }
}

pub enum Msg {
    // Insertion
    MoveInsert(Direction),
    JumpInsert(Direction),

    // Editing
    Insert(ByteChar),
    Delete,

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
    pub fn from_event(event: Event) -> Msg {
        let Event::Key { event, keyboard } = event;
        let is_shift = keyboard.modifier_bitfield.contains(Modifier::LeftShift)
            | keyboard.modifier_bitfield.contains(Modifier::RightShift);
        let is_super = keyboard.modifier_bitfield.contains(Modifier::LeftSuper)
            | keyboard.modifier_bitfield.contains(Modifier::RightSuper);

        match event {
            KeyEvent::Pressed(key) => match key {
                Key::Standard(standard) => {
                    match standard {
                        // Selection
                        StandardKey::Left if is_shift => {
                            if is_super {
                                Msg::JumpSelection(Direction::Left)
                            } else {
                                Msg::MoveSelection(Direction::Left)
                            }
                        }
                        StandardKey::Right if is_shift => {
                            if is_super {
                                Msg::JumpSelection(Direction::Right)
                            } else {
                                Msg::MoveSelection(Direction::Right)
                            }
                        }
                        StandardKey::Up if is_shift => {
                            if is_super {
                                Msg::JumpSelection(Direction::Up)
                            } else {
                                Msg::MoveSelection(Direction::Up)
                            }
                        }
                        StandardKey::Down if is_shift => {
                            if is_super {
                                Msg::JumpSelection(Direction::Down)
                            } else {
                                Msg::MoveSelection(Direction::Down)
                            }
                        }

                        StandardKey::A if is_super => {
                            if is_shift {
                                Msg::SelectLine
                            } else {
                                Msg::SelectAll
                            }
                        }

                        // Jump cursor
                        StandardKey::Left if is_super => Msg::JumpInsert(Direction::Left),
                        StandardKey::Right if is_super => Msg::JumpInsert(Direction::Right),
                        StandardKey::Down if is_super => Msg::JumpInsert(Direction::Down),
                        StandardKey::Up if is_super => Msg::JumpInsert(Direction::Up),

                        // Move cursor
                        StandardKey::Left => Msg::MoveInsert(Direction::Left),
                        StandardKey::Right => Msg::MoveInsert(Direction::Right),
                        StandardKey::Down => Msg::MoveInsert(Direction::Down),
                        StandardKey::Up => Msg::MoveInsert(Direction::Up),

                        // Clipboard
                        StandardKey::C if is_super => Msg::Copy,
                        StandardKey::X if is_super => Msg::Cut,
                        StandardKey::V if is_super => Msg::Paste,

                        // History
                        StandardKey::Z if is_super => {
                            if is_shift {
                                Msg::Redo
                            } else {
                                Msg::Undo
                            }
                        }

                        _ => {
                            let to_char = if is_shift {
                                StandardKey::to_char_upper
                            } else {
                                StandardKey::to_char_lower
                            };

                            if let Some(character) = to_char(standard) {
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
}
