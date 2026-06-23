use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_10X20},
    text::renderer::TextRenderer,
};
use embedded_gui::{
    component::Component,
    interactive::FocusState,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::owned_text::OwnedText,
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};

use itertools::Itertools;
use keyboard_protocol::{Key, StandardKey};

use crate::{
    gui::{self, colors, display, event, select_file},
    input::event::Event,
    storage::{LoadError, MAX_FILES},
    text::{Name, fixed_str, name_to_string},
};

pub const VISIBLE_FILES: usize = 8;

/// Represents a list of files that the user can scroll through
#[derive(Reactive)]
pub struct FileList<'a> {
    pub files: SignalRef<'a, Result<heapless::Vec<Name, { MAX_FILES }>, LoadError>>,
    pub scroll: Signal<usize>,
}

impl<'a> IntrinsicSize for FileList<'a> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        unimplemented!("File list only supports fill sizing")
    }
}

impl<'a>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for FileList<'a>
{
    fn view(
        &self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
        _: embedded_gui::view::Children<
            'a,
            display::Driver,
            Event,
            gui::Msg,
            gui::FocusKey,
            gui::AnyComponent<'a>,
            gui::AnyPrimitive<'a>,
        >,
    ) -> embedded_gui::view::View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        // Convert the provided list of files into a list of file entry views.
        let Ok(list_items) = self
            .files
            .as_ref()
            .unwrap_or(&heapless::Vec::from_array([fixed_str(&"Error loading")]))
            .iter()
            // Skip scrolled files
            .skip(self.scroll.saturating_sub(VISIBLE_FILES))
            // Only take the number of files that we can show on screen at once
            .take(VISIBLE_FILES)
            .enumerate()
            .map(|(index, file_name)| {
                v.component(
                    Sizing::Fill,
                    FileEntry {
                        name: self.files.map(|_| file_name.clone()),
                        index: self.scroll.map(|_| index),
                    },
                    [],
                )
            })
            // To ensure that we always provide an array of 10 elements, we pad it with spacer rows
            .pad_using(VISIBLE_FILES, |_| v.spacer())
            .collect::<heapless::Vec<_, { VISIBLE_FILES }>>()
            .into_array::<VISIBLE_FILES>()
        else {
            panic!("More files selected than can be displayed!")
        };

        v.view(Direction::Vertical, list_items)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

/// A row in a file list representing an individual file
#[derive(Reactive)]
pub struct FileEntry {
    pub name: Signal<Name>,
    pub index: Signal<usize>,
}

impl IntrinsicSize for FileEntry {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        unimplemented!("File entry only supports fill sizing")
    }
}

impl<'a>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for FileEntry
{
    fn view(
        &self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
        _: embedded_gui::view::Children<
            'a,
            display::Driver,
            Event,
            gui::Msg,
            gui::FocusKey,
            gui::AnyComponent<'a>,
            gui::AnyPrimitive<'a>,
        >,
    ) -> embedded_gui::view::View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        let text = self.name.map(|name| name_to_string(name.clone()));
        let index = *self.index;

        v.view(
            Direction::Vertical,
            [
                v.spacer(),
                v.interactive(
                    select_file::FocusKey::FileList(*self.index),
                    event::on_keydown(
                        move |key| match key {
                            // Scroll to the file above us
                            Key::Standard(StandardKey::Down) => {
                                Some(select_file::Msg::ScrollFileList(ScrollDirection::Down))
                            }
                            // Scroll to the file below us
                            Key::Standard(StandardKey::Up) => {
                                Some(select_file::Msg::ScrollFileList(ScrollDirection::Up))
                            }
                            // They've selected us!
                            Key::Standard(StandardKey::Enter) => {
                                Some(select_file::Msg::SelectFile(index))
                            }
                            _ => None,
                        },
                        move |key| match key {
                            // super + backspace -> delete :(
                            Key::Standard(StandardKey::Backspace) => {
                                Some(select_file::Msg::DeleteFile(index))
                            }
                            _ => None,
                        },
                    ),
                    move |focus_state| {
                        let font_style = {
                            let builder = MonoTextStyleBuilder::new()
                                .font(&FONT_10X20)
                                .text_color(colors::TEXT);

                            let with_underline = match *focus_state {
                                FocusState::Focused => builder.underline(),
                                FocusState::Unfocused => builder,
                            };

                            with_underline.build()
                        };

                        v.group(
                            Direction::Horizontal,
                            Sizing::Constrained(font_style.line_height() as u16),
                            [
                                v.primitive(
                                    Sizing::Intrinsic,
                                    OwnedText {
                                        content: text,
                                        font_style: Signal::constant(font_style),
                                    },
                                ),
                                v.spacer(),
                            ],
                        )
                    },
                ),
                v.spacer(),
            ],
        )
        .with_background(colors::PURPLE)
    }
}
