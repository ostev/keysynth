use embedded_graphics::mono_font::{MonoTextStyleBuilder, ascii};
use embedded_gui::{
    app::Change,
    interactive::FocusState,
    layout::{Direction, Sizing},
    primitive::{spacer::Spacer, text::Text},
    signal::{Reactive, Signal, SignalRef, Source},
    size::Size,
    view::View,
};
use keyboard_protocol::{Key, StandardKey};

pub mod file_list;

use crate::{
    gui::{
        self, colors, display,
        editor::{
            self,
            line::{self, LineEditor},
        },
        effect::Effect,
        select_file::file_list::{FileList, ScrollDirection},
    },
    input::event::{Event, KeyEvent},
    storage::{self, Files, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FocusKey {
    #[default]
    NewFile,

    FileList(usize),
}

pub enum Msg {
    LineEditor(line::Msg),

    ScrollFileList(ScrollDirection),
    SelectFile(usize),
    DeleteFile(usize),

    FilesReceived(Result<storage::Files, LoadError>),
}

/// State for the file selection page.
///
/// This page allows the user to create a new file, browse existing files,
/// load a selected file, or delete a file.
#[derive(Reactive, embedded_gui::app::State)]
pub struct SelectFile {
    /// The list of available files.
    files: Source<Result<heapless::Vec<Name, { MAX_FILES }>, LoadError>>,

    /// The scroll offset of the list.
    scroll: Source<usize>,

    /// State of the filename editor used when creating a new file.
    line_editor: Source<line::State<{ NAME_SIZE }>>,
}

impl Default for SelectFile {
    fn default() -> Self {
        Self {
            files: Source::new(Ok(heapless::Vec::new())),
            scroll: Source::new(0),
            line_editor: Source::new(line::State::new()),
        }
    }
}

impl SelectFile {
    pub fn update(&mut self, msg: Msg) -> Change<gui::Msg, gui::FocusKey, Effect> {
        match msg {
            Msg::LineEditor(line_msg) => {
                self.line_editor.update(|line| line.update(line_msg));
            }

            Msg::ScrollFileList(direction) => {
                return Change::new().with_focus_key_if_present(self.scroll(direction));
            }

            Msg::SelectFile(index) => {
                if let Ok(files) = &*self.files {
                    if let Some(name) = files.get(index) {
                        return Change::new().with_effect(Effect::Load(name.clone()));
                    }
                }
            }
            Msg::DeleteFile(index) => {
                if let Ok(files) = &*self.files {
                    if let Some(name) = files.get(index).cloned() {
                        let focus_key = if index == files.len() - 1 {
                            self.scroll(ScrollDirection::Up)
                        } else {
                            None
                        };

                        return Change::new()
                            .with_focus_key_if_present(focus_key)
                            .with_effect(Effect::Delete(name));
                    }
                }
            }

            Msg::FilesReceived(files) => {
                self.files.set(files.map(Files::sorted_by_most_recent));
            }
        }

        Change::new()
    }

    /// Scroll the file list in the specified direction.
    fn scroll(&mut self, direction: ScrollDirection) -> Option<FocusKey> {
        let num_files = self.files.as_ref().map(|files| files.len()).unwrap_or(0);
        if *self.scroll == 0 && direction == ScrollDirection::Up {
            Some(FocusKey::NewFile)
        } else if num_files > 0 {
            self.scroll.set_with(|scroll| match direction {
                ScrollDirection::Down => (*scroll + 1).min(num_files.saturating_sub(1)),
                ScrollDirection::Up => scroll.saturating_sub(1),
            });
            Some(FocusKey::FileList(*self.scroll))
        } else {
            None
        }
    }
}

impl SelectFile {
    pub fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
    ) -> View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        let file_name = self.line_editor.as_fixed_byte_string();
        let num_files = self.files.as_ref().map(|files| files.len()).unwrap_or(0);

        v.view(
            Direction::Vertical,
            [v.centered(
                Direction::Horizontal,
                v.group(
                    Direction::Vertical,
                    Sizing::Constrained(200),
                    [
                        v.interactive(
                            FocusKey::NewFile,
                            move |event| match event {
                                Event::Key {
                                    event: key_event, ..
                                } => match key_event {
                                    KeyEvent::Pressed(key) => match key {
                                        Key::Standard(StandardKey::Down) => {
                                            if num_files > 0 {
                                                Some(gui::Msg::ChangeFocus(
                                                    gui::FocusKey::SelectFile(FocusKey::FileList(
                                                        0,
                                                    )),
                                                ))
                                            } else {
                                                None
                                            }
                                        }
                                        Key::Standard(StandardKey::Enter) => {
                                            Some(gui::Msg::LoadCompleted(Ok(
                                                editor::source::Source::new(file_name.clone()),
                                            )))
                                        }
                                        _ => line::Msg::from_event(event)
                                            .map(|msg| Msg::LineEditor(msg).into()),
                                    },

                                    _ => line::Msg::from_event(event)
                                        .map(|msg| Msg::LineEditor(msg).into()),
                                },
                                _ => line::Msg::from_event(event)
                                    .map(|msg| Msg::LineEditor(msg).into()),
                            },
                            |focus_state| {
                                v.group(
                                    Direction::Vertical,
                                    Sizing::Constrained(40),
                                    [
                                        v.primitive(
                                            Sizing::Intrinsic,
                                            Text {
                                                content: SignalRef::constant(&"New file: "),
                                                font_style: focus_state.map(|focus_state| {
                                                    let builder = MonoTextStyleBuilder::new()
                                                        .font(&ascii::FONT_10X20)
                                                        .text_color(colors::TEXT);

                                                    match focus_state {
                                                        FocusState::Focused => builder
                                                            .underline_with_color(colors::TEXT)
                                                            .build(),
                                                        FocusState::Unfocused => builder.build(),
                                                    }
                                                }),
                                            },
                                        ),
                                        v.primitive(
                                            Sizing::Fill,
                                            LineEditor::new(self.line_editor.signal_ref()),
                                        ),
                                    ],
                                )
                            },
                        ),
                        v.primitive(
                            Sizing::Intrinsic,
                            Spacer {
                                size: Signal::constant(Size::new(0, 20)),
                            },
                        ),
                        v.component(
                            Sizing::Fill,
                            FileList {
                                files: self.files.signal_ref(),
                                scroll: self.scroll.signal(),
                            },
                            [],
                        ),
                    ],
                ),
            )],
        )
    }
}
