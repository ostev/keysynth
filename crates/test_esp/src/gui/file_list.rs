use embedded_graphics::mono_font::{MonoTextStyleBuilder, ascii::FONT_6X12};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::owned_text::OwnedText,
    signal::{Reactive, Signal, SignalRef},
    size::Size,
    view::Children,
};

use keyboard_protocol::{Key, StandardKey};

use crate::{
    gui::{
        self, colors, display,
        event::{self, Event, KeyEvent::Pressed},
        select_file,
    },
    storage::MAX_FILES,
    text::{Name, name_to_string},
};

pub const VISIBLE_FILES: usize = 10;

#[derive(Reactive)]
pub struct FileList<'a> {
    pub files: SignalRef<'a, heapless::Vec<Name, { MAX_FILES }>>,
    pub scroll: Signal<usize>,
}

impl<'a> IntrinsicSize for FileList<'a> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        unimplemented!()
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
        let Ok(list_items) = self
            .files
            .iter()
            .skip(*self.scroll)
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
            .collect::<heapless::Vec<_, { VISIBLE_FILES }>>()
            .into_array::<VISIBLE_FILES>()
        else {
            panic!("More files selected than can be displayed!")
        };

        v.view(
            Direction::Vertical,
            // [v.interactive(
            //     gui::FocusKey::FileList,
            //     |event| match event {
            //         Event::Key { event, .. } => match event {
            //             Pressed(Key::Standard(key)) => match key {
            //                 StandardKey::Down => gui::Msg::ScrollFileList(ScrollDirection::Down),
            //                 StandardKey::Up => gui::Msg::ScrollFileList(ScrollDirection::Up),
            //                 _ => gui::Msg::NoOp,
            //             },
            //             _ => gui::Msg::NoOp,
            //         },
            //     },
            //     |_| v.group(Direction::Vertical, []),
            // )],
            list_items,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Reactive)]
pub struct FileEntry {
    pub name: Signal<Name>,
    pub index: Signal<usize>,
}

impl IntrinsicSize for FileEntry {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::new(200, 40)
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
            Direction::Horizontal,
            [v.interactive(
                select_file::FocusKey::FileList(*self.index),
                move |event| {
                    event::handler(
                        event,
                        select_file::Msg::ScrollFileList(ScrollDirection::Down),
                        select_file::Msg::ScrollFileList(ScrollDirection::Up),
                        select_file::Msg::SelectFile(index),
                        select_file::Msg::NoOp,
                    )
                },
                move |focus_state| {
                    let font_style = {
                        let builder = MonoTextStyleBuilder::new()
                            .font(&FONT_6X12)
                            .text_color(colors::TEXT);

                        let with_underline = if let Some(_) = focus_state {
                            builder.underline()
                        } else {
                            builder
                        };

                        with_underline.build()
                    };

                    v.group(
                        Direction::Horizontal,
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
            )],
        )
        .with_background(colors::PURPLE)
    }
}
