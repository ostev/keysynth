use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii};
use embedded_gui::{
    app::Change,
    component::{Component, button::Button, group::Group},
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::text::Text,
    signal::{Reactive, Signal, SignalRef, Source},
    view::{View, Widget},
};

use crate::{
    gui::{
        self, colors, display,
        editor::{
            self,
            line::{self, LineEditor},
        },
        effect::Effect,
        event::{self, Event},
        file_list::{FileList, ScrollDirection},
        text_bar::TextBar,
    },
    storage::{self, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name, fixed_str},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]

pub enum FocusKey {
    NewFile,
    FileList(usize),
}

pub enum Msg {
    LineEditor(line::Msg),

    ScrollFileList(ScrollDirection),
    SelectFile(usize),

    FilesReceived(Result<storage::Files, LoadError>),

    NoOp,
}

#[derive(Reactive, embedded_gui::app::State)]
pub struct SelectFile {
    files: Source<heapless::Vec<Name, { MAX_FILES }>>,
    scroll: Source<usize>,

    line_editor: Source<line::State<{ NAME_SIZE }>>,
}

impl Default for SelectFile {
    fn default() -> Self {
        Self {
            files: Source::new(heapless::Vec::new()),
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

            Msg::ScrollFileList(scroll_direction) => {
                if *self.scroll == 0 {
                    return Change::none().with_focus_key(FocusKey::NewFile);
                } else {
                    self.scroll.update(|scroll| match scroll_direction {
                        ScrollDirection::Down => (*scroll + 1).min(MAX_FILES),
                        ScrollDirection::Up => scroll.saturating_sub(1),
                    });
                }
            }

            Msg::SelectFile(index) => {
                if let Some(name) = self.files.get(index) {
                    return Change::none().with_effect(Effect::Load(name.clone()));
                }
            }

            Msg::FilesReceived(files) => {
                self.files.update(|_| {
                    files.ok().map(|unsorted_files| {
                        let mut reversed_files: heapless::Vec<Name, MAX_FILES> =
                            unsorted_files.files.into_iter().cloned().collect();

                        reversed_files.reverse();

                        reversed_files
                    })
                });
            }

            Msg::NoOp => {}
        }

        Change::none()
    }
}

impl IntrinsicSize for SelectFile {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        unimplemented!()
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

        v.view(
            Direction::Vertical,
            [
                v.interactive(
                    FocusKey::NewFile,
                    move |event| {
                        event::handler(
                            event,
                            {
                                gui::Msg::ChangeFocus(gui::FocusKey::SelectFile(
                                    FocusKey::FileList(0),
                                ))
                            },
                            Msg::NoOp.into(),
                            gui::Msg::LoadCompleted(Ok(editor::source::Source::new(
                                file_name.clone(),
                            ))),
                            Msg::NoOp.into(),
                        )
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
                        text: Signal::constant(fixed_str(&"[enter] to select")),
                    },
                    [],
                ),
            ],
        )
    }
}
