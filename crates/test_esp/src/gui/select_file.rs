use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii};
use embedded_gui::{
    app::Change,
    component::{Component, button::Button, group::Group},
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::text::Text,
    signal::{Reactive, Signal, SignalRef, Source},
    view::View,
};
use esp_println::println;
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
        event::{self},
        select_file::file_list::{FileList, ScrollDirection},
        text_bar::{OwnedTextBar, TextBar},
    },
    input::event::{Event, KeyEvent},
    storage::{self, Files, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name, fixed_str},
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

    NoOp,
}

#[derive(Reactive, embedded_gui::app::State)]
pub struct SelectFile {
    files: Source<Result<heapless::Vec<Name, { MAX_FILES }>, LoadError>>,
    scroll: Source<usize>,

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
                            .with_effect(Effect::Delete(name.clone()));
                    }
                }
            }

            Msg::FilesReceived(files) => {
                self.files.set(files.map(Files::sorted_by_most_recent));
            }

            Msg::NoOp => {}
        }

        Change::new()
    }

    fn scroll(&mut self, direction: ScrollDirection) -> Option<FocusKey> {
        let num_files = self.files.as_ref().map(|files| files.len()).unwrap_or(0);
        if *self.scroll == 0 && direction == ScrollDirection::Up {
            Some(FocusKey::NewFile)
        } else if num_files > 0 {
            self.scroll.set_with(|scroll| match direction {
                ScrollDirection::Down => (*scroll + 1).min(num_files.saturating_sub(1)).max(0),
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
        const BAR_HEIGHT: u16 = 40;

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
                            |_| {
                                v.component(
                                    Sizing::Constrained(BAR_HEIGHT),
                                    Group::zero(Signal::constant(Direction::Horizontal)),
                                    [
                                        v.primitive(
                                            Sizing::Intrinsic,
                                            Text {
                                                content: SignalRef::constant(&"New file: "),
                                                font_style: Signal::constant(
                                                    MonoTextStyleBuilder::new()
                                                        .font(&ascii::FONT_6X13_ITALIC)
                                                        .text_color(colors::TEXT)
                                                        .underline_with_color(colors::PURPLE)
                                                        .build(),
                                                ),
                                            },
                                        ),
                                        v.spacer(),
                                        v.primitive(
                                            Sizing::Intrinsic,
                                            LineEditor::new(self.line_editor.signal_ref()),
                                        ),
                                    ],
                                )
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
                        v.component(
                            Sizing::Constrained(BAR_HEIGHT),
                            TextBar {
                                text: SignalRef::constant(
                                    &"[enter] to select, [super + backspace] to delete",
                                ),
                            },
                            [],
                        ),
                    ],
                ),
            )],
        )
    }
}
