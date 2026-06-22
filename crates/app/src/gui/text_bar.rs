use alloc::boxed::Box;
use bumpalo::Bump;
use embedded_graphics::mono_font::{MonoFont, MonoTextStyleBuilder, ascii::FONT_10X20};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::owned_text::OwnedText,
    signal::{Reactive, Signal},
    size::Size,
};

use crate::{
    gui::{self, colors, display},
    input::event::Event,
};

const FONT: MonoFont = FONT_10X20;

/// A full-width colored bar that displays centered text.
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
        )
        .with_background(colors::PURPLE)
    }
}
