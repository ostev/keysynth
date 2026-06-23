use core::cmp;

use alloc::borrow::{Cow, ToOwned};
use embassy_time::Duration;
use embedded_gui::app::Change;
use keyboard_protocol::{Key, Modifier, StandardKey};

use crate::{
    gui::{
        self,
        editor::{
            clipboard::Clipboard,
            history::{self, History},
            position::{Direction, Position, Selection, SelectionRange},
            source::Source,
        },
        message::Message,
    },
    input::event::{Event, KeyEvent},
    text::{ByteChar, Name, fixed_str},
};

/// Determines how many lines of text are visible on the screen at once.
pub const LINES_VISIBLE: usize = 8;

/// Determines how many lines from either edge of the screen before the editor scrolls.
pub const SCROLL_GAP: usize = 2;

/// Contains all of the mutable state for the text editor, including the source code, editing state,
/// undo history and transient UI state.
#[derive(Clone)]
pub struct EditorState {
    pub(super) source: Source,
    pub(super) cursor: Position,
    pub(super) selection: Selection,
    pub(super) clipboard: Clipboard,
    pub(super) history: History,

    is_caps_lock: bool,

    pub(super) scroll_start: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(Source::new(fixed_str(&"unnamed")))
    }
}

impl EditorState {
    /// Creates an editor positioned at the start of the provided source.
    pub fn new(source: Source) -> Self {
        let cursor = Position::new(0, 0);

        Self {
            source,
            cursor,
            selection: Selection::new(None),
            clipboard: Clipboard::new(),
            history: History::new(cursor),

            is_caps_lock: false,

            scroll_start: 0,
        }
    }

    /// Returns a short preview of the document for use in menus.
    ///
    /// The preview is taken from the final line and truncated if necessary.
    pub fn get_preview<const PREVIEW_LENGTH: usize>(&self) -> heapless::String<PREVIEW_LENGTH> {
        self.source
            .lines
            .last()
            .map(|last_line| {
                heapless::String::<PREVIEW_LENGTH>::from_utf8(
                    heapless::Vec::from_slice(
                        &last_line[..(PREVIEW_LENGTH - 3).min(last_line.len())],
                    )
                    .unwrap(),
                )
                .unwrap_or(heapless::format!("<corrupted>").unwrap())
            })
            .unwrap_or(heapless::format!("<empty>").unwrap())
    }

    /// Serialise the source code into a buffer
    pub fn serialize<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], postcard::Error> {
        self.source.serialize(buffer)
    }

    /// Name of the currently open file
    pub fn name(&self) -> &Name {
        &self.source.name
    }

    /// Convert the editor source to a `String`. The source **must** be valid UTF-8.
    /// Since the user can only type ASCII, this will be fine unless the file has been
    /// modified on flash.
    pub unsafe fn to_string_unchecked(&self) -> alloc::string::String {
        unsafe { self.source.to_string_unchecked() }
    }

    /// Applies an editor [`Msg`] to the editor's state
    pub fn update(&mut self, msg: Msg) -> Change<gui::Msg, gui::FocusKey, gui::Effect> {
        match msg {
            // Movement
            Msg::MoveInsert(direction) => {
                self.move_cursor(direction, false);
            }
            Msg::JumpInsert(direction) => {
                self.jump_cursor(direction, false);
            }

            // Selection
            Msg::MoveSelection(direction) => {
                self.move_cursor(direction, true);
            }
            Msg::JumpSelection(direction) => {
                self.jump_cursor(direction, true);
            }
            Msg::ClearSelection => {
                self.selection.clear();
            }
            Msg::SelectAll => self.select_all(),
            Msg::SelectLine => self.select_line(),

            // Whitespace
            Msg::Tab => {
                if self.selection.is_active() {
                    self.delete(false, false);
                }

                // Insert two spaces
                let new_cursor =
                    (0..2).fold(self.cursor, |cursor, _| self.source.insert(b' ', cursor));

                self.history.push(
                    history::Edit::GroupInsert(alloc::vec![b' ', b' '], ()),
                    self.cursor,
                    new_cursor,
                );
                self.cursor = new_cursor;
            }

            // Insertion
            Msg::Insert(character) => {
                if self.selection.is_active() {
                    self.delete(false, false);
                }

                let character = if self.is_caps_lock {
                    character.to_ascii_uppercase()
                } else {
                    character
                };

                let new_cursor = self.source.insert(character, self.cursor);

                self.history.push(
                    history::Edit::Insert(character, self.cursor),
                    self.cursor,
                    new_cursor,
                );
                self.cursor = new_cursor;
            }

            // Deletion
            Msg::Backspace => self.delete(false, false),
            Msg::BackspaceLine => self.delete(false, true),
            Msg::ForwardDelete => self.delete(true, false),
            Msg::ForwardDeleteLine => self.delete(true, true),

            // Clipboard
            Msg::Copy => self.copy(),
            Msg::Paste => self.paste(),
            Msg::Cut => {
                self.copy();
                self.delete(false, false);
            }

            // History
            Msg::Undo => {
                if let Some((edit, cursor)) = self.history.undo() {
                    self.source.apply(edit);
                    self.cursor = cursor;

                    self.selection.clear();
                }
            }
            Msg::Redo => {
                if let Some((edit, cursor)) = self.history.redo() {
                    self.source.apply(edit);
                    self.cursor = cursor;

                    self.selection.clear();
                }
            }

            // Caps lock
            Msg::ToggleCapsLock => {
                self.is_caps_lock = !self.is_caps_lock;
            }
        };

        self.scroll();

        Change::new()
    }

    fn copy(&mut self) {
        if let Some(selection_range) = self.selection.range(self.cursor) {
            // If the copied selection is too large (which it probably won't be),
            // we just won't copy. This is fine, since it's pretty obvious.
            let _ = self.clipboard.set(self.source.get_range(selection_range));
        }
    }

    fn delete(&mut self, is_forward: bool, is_line: bool) {
        match self.selection.range(self.cursor) {
            Some(selection_range) => {
                // Delete the active selection
                let text = self.source.get_range(selection_range);

                let new_cursor = self.source.group_delete(selection_range);

                self.history.push(
                    history::Edit::GroupDelete(text, selection_range),
                    self.cursor,
                    new_cursor,
                );

                self.cursor = new_cursor;
                self.selection.clear();
            }

            None => {
                if is_line {
                    // Line deletion removes everything from the cursor to the beginning/end of the
                    // current line, depending on the direction.
                    let (edit, new_column) = if is_forward {
                        let deleted = self.source.forward_delete_line(self.cursor);
                        let new_column =
                            self.source.lines[self.cursor.line].len().saturating_sub(1);
                        (
                            history::Edit::GroupDelete(
                                deleted,
                                SelectionRange::new(
                                    self.cursor,
                                    Position::new(self.cursor.line, new_column),
                                ),
                            ),
                            new_column,
                        )
                    } else {
                        let deleted = self.source.backspace_line(self.cursor);
                        (
                            history::Edit::GroupDelete(
                                deleted,
                                SelectionRange::new(
                                    Position {
                                        line: self.cursor.line,
                                        column: 0,
                                    },
                                    self.cursor,
                                ),
                            ),
                            0,
                        )
                    };

                    let new_cursor = Position {
                        line: self.cursor.line,
                        column: new_column,
                    };

                    self.history.push(edit, self.cursor, new_cursor);
                    self.cursor = new_cursor;
                } else {
                    let line = &mut self.source.lines[self.cursor.line];

                    let deletion_cursor = if is_forward {
                        self.cursor
                    } else {
                        if self.cursor.column > 0 || self.cursor.line > 0 {
                            Position {
                                line: self.cursor.line,
                                column: self.cursor.column,
                            }
                        } else {
                            // We're at the start of the document, and we can't backspace here.
                            return;
                        }
                    };

                    // Character deletion falls back to deleting the newline when positioned at the
                    // end of a line, allowing adjacent lines to be merged.
                    let character = line
                        .get(deletion_cursor.column)
                        .map(|char| *char)
                        .unwrap_or(b'\n');

                    let new_cursor = self.source.delete(deletion_cursor);

                    self.history.push(
                        history::Edit::Delete(character, self.cursor),
                        self.cursor,
                        new_cursor,
                    );
                    self.cursor = new_cursor;
                }
            }
        }
    }

    // Replace the current selection before inserting the clipboard contents.
    fn paste(&mut self) {
        if self.selection.is_active() {
            // Delete the current selection if there is one
            self.delete(false, false);
        }

        let text = self.clipboard.get();

        let new_cursor = self.source.group_insert(text, self.cursor);

        self.history.push(
            history::Edit::GroupInsert(text.to_owned(), self.cursor),
            self.cursor,
            new_cursor,
        );

        self.cursor = new_cursor;
    }

    /// Moves the cursor by one step.
    fn move_cursor(&mut self, direction: Direction, is_selecting: bool) {
        let start = self.cursor;

        match direction {
            Direction::Left => {
                self.cursor.column = self.cursor.column.saturating_sub(1);
            }
            Direction::Right => {
                self.cursor.column = self.cursor.column.saturating_add(1);
                self.clamp_cursor_column();
            }
            Direction::Up => {
                self.move_cursor_up(1);
            }
            Direction::Down => {
                self.move_cursor_down(1);
            }
        }

        self.update_selection(start, is_selecting);
    }

    /// Updates the active selection if the user is selecting and clears it if not.
    fn update_selection(&mut self, start_cursor: Position, is_selecting: bool) {
        if is_selecting {
            self.selection = self.selection.with_cursor(start_cursor)
        } else {
            self.selection.clear();
        }
    }

    /// Moves the cursor up the file by a specified amount
    fn move_cursor_up(&mut self, amount: usize) {
        self.cursor.line = self.cursor.line.saturating_sub(amount);
        self.clamp_cursor_column();
    }

    /// Moves the cursor down the file by a specified amount
    fn move_cursor_down(&mut self, amount: usize) {
        self.cursor.line = self
            .cursor
            .line
            .saturating_add(amount)
            .min(self.source.lines.len().saturating_sub(1));
        self.clamp_cursor_column();
    }

    /// Scroll the editor up or down so that the cursor is kept out of the scroll margin.
    fn scroll(&mut self) {
        let offset_from_top = self.cursor.line.saturating_sub(self.scroll_start);
        let offset_from_bottom = LINES_VISIBLE.saturating_sub(offset_from_top);

        if offset_from_top < SCROLL_GAP {
            // Scroll up!
            self.scroll_start = self.cursor.line.saturating_sub(SCROLL_GAP);
        } else if offset_from_bottom < SCROLL_GAP {
            // Scroll down!
            // We don't need `saturating_add` since we won't reach `u32::MAX` lines.
            self.scroll_start += SCROLL_GAP;
        }
    }

    /// Ensures the cursor column remains valid after changing lines.
    fn clamp_cursor_column(&mut self) {
        self.cursor.column = cmp::min(
            self.cursor.column,
            self.source.lines[self.cursor.line].len(),
        );
    }

    /// Moves the cursor to the beginning/end of the current line or document.
    fn jump_cursor(&mut self, direction: Direction, is_selecting: bool) {
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

        self.update_selection(start, is_selecting);
    }

    /// Selects everything and moves the cursor to the end
    fn select_all(&mut self) {
        let last_line = self.source.lines.len().saturating_sub(1);
        let end_col = self.source.lines[last_line].len();

        self.cursor = Position {
            line: last_line,
            column: end_col,
        };
        self.selection = Selection::new(Some(Position::zero()));
    }

    /// Select all of the current line
    fn select_line(&mut self) {
        let line_len = self.source.lines[self.cursor.line].len();

        self.cursor.column = line_len;
        self.selection = Selection::new(Some(Position::new(self.cursor.line, 0)));
    }
}

pub enum Msg {
    // Insertion
    MoveInsert(Direction),
    JumpInsert(Direction),

    // Editing
    Insert(ByteChar),
    Backspace,
    BackspaceLine,
    ForwardDelete,
    ForwardDeleteLine,

    // Whitespace
    Tab,

    // Selection
    MoveSelection(Direction),
    JumpSelection(Direction),
    SelectAll,
    SelectLine,
    ClearSelection,

    // Clipboard
    Copy,
    Cut,
    Paste,

    // History
    Undo,
    Redo,

    // Caps lock
    ToggleCapsLock,
}

impl Msg {
    /// Converts keyboard input into editor [`Msg`]s.
    pub fn from_event(event: Event) -> Option<Msg> {
        match event {
            Event::Key { event, keyboard } => {
                let is_shift = keyboard.is_shift();
                let is_super = keyboard.is_super();

                let msg = match event {
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

                                StandardKey::Esc => Msg::ClearSelection,

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

                                // Deletion
                                StandardKey::Backspace => {
                                    if is_super {
                                        Msg::BackspaceLine
                                    } else {
                                        Msg::Backspace
                                    }
                                }
                                StandardKey::Delete => {
                                    if is_super {
                                        Msg::ForwardDeleteLine
                                    } else {
                                        Msg::ForwardDelete
                                    }
                                }

                                // Caps lock
                                StandardKey::CapsLock => Msg::ToggleCapsLock,

                                // Whitespace
                                StandardKey::Tab => Msg::Tab,

                                // Insertion
                                _ => {
                                    if is_super {
                                        return None;
                                    } else {
                                        let to_char = if is_shift {
                                            StandardKey::to_char_upper
                                        } else {
                                            StandardKey::to_char_lower
                                        };

                                        if let Some(character) = to_char(standard) {
                                            Msg::Insert(character)
                                        } else {
                                            return None;
                                        }
                                    }
                                }
                            }
                        }
                        Key::Modifier(_) => return None,
                        Key::Special(_) => return None,
                    },
                    KeyEvent::Released(_) => return None,
                };

                Some(msg)
            }

            _ => None,
        }
    }
}
