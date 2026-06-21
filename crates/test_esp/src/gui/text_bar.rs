use alloc::boxed::Box;
use bumpalo::Bump;
use embedded_graphics::{
    mono_font::{
        MonoFont, MonoTextStyleBuilder,
        ascii::{FONT_6X12, FONT_10X20},
    },
    pixelcolor::{Rgb565, RgbColor},
};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::{owned_text::OwnedText, text::Text},
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};
use heapless::format;

use crate::{
    gui::{self, AnyPrimitive, colors, display},
    input::event::Event,
    text::FixedByteString,
};

const FONT: MonoFont = FONT_10X20;

#[derive(Reactive)]
pub struct OwnedTextBar<const N: usize> {
    pub text: Signal<heapless::String<N>>,
}

impl<const N: usize> IntrinsicSize for OwnedTextBar<N> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::new(display::WIDTH as u16, 20)
    }
}

impl<'a, const N: usize>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for OwnedTextBar<N>
where
    Box<OwnedText<display::Color, N>, &'a Bump>: Into<gui::AnyPrimitive<'a>>,
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
        v.view(
            Direction::Horizontal,
            [v.centered(
                Direction::Horizontal,
                v.primitive(
                    Sizing::Constrained(N as u16 * FONT.character_size.width as u16),
                    OwnedText {
                        content: self.text.clone(),
                        font_style: Signal::constant(
                            MonoTextStyleBuilder::new()
                                .font(&FONT)
                                .underline()
                                .text_color(colors::BACKGROUND_DARK)
                                .build(),
                        ),
                    },
                ),
            )],
            // [v.spacer()],
        )
        .with_background(colors::PURPLE)
    }
}

#[derive(Reactive)]
pub struct TextBar<'a, S: AsRef<str>> {
    pub text: SignalRef<'a, S>,
}

impl<'a, S: AsRef<str>> IntrinsicSize for TextBar<'a, S> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::new(display::WIDTH as u16, FONT.character_size.height as u16)
    }
}

impl<'a, S: AsRef<str>>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for TextBar<'a, S>
where
    Box<Text<'a, display::Color, S>, &'a Bump>: Into<gui::AnyPrimitive<'a>>,
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
        v.view(
            Direction::Horizontal,
            [v.centered(
                Direction::Horizontal,
                v.primitive(
                    Sizing::Intrinsic,
                    Text {
                        content: self.text.clone(),
                        font_style: Signal::constant(
                            MonoTextStyleBuilder::new()
                                .font(&FONT)
                                .underline()
                                .text_color(colors::BACKGROUND_DARK)
                                .build(),
                        ),
                    },
                ),
            )],
        )
        .with_background(colors::PURPLE)
    }
}
