use embedded_graphics::mono_font::{MonoTextStyleBuilder, ascii::FONT_6X12};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::owned_text::OwnedText,
    signal::{Reactive, Signal},
    size::Size,
};
use heapless::format;

use crate::{
    gui::{self, colors, display, event::Event},
    text::FixedByteString,
};

#[derive(Reactive)]
pub struct TextBar {
    pub text: Signal<FixedByteString<14>>,
}

impl IntrinsicSize for TextBar {
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
    > for TextBar
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
        let text = self.text.map(|text| {
            heapless::String::from_utf8(text.clone()).unwrap_or(format!("<corrupted>").unwrap())
        });

        v.view(
            Direction::Horizontal,
            [v.centered(
                Direction::Horizontal,
                v.primitive(
                    Sizing::Intrinsic,
                    OwnedText {
                        content: text,
                        font_style: Signal::constant(
                            MonoTextStyleBuilder::new()
                                .font(&FONT_6X12)
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
