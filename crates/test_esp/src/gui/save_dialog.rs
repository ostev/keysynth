use embedded_graphics::mono_font::{
    MonoTextStyleBuilder,
    ascii::{self, FONT_10X20},
};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::{spacer::Spacer, text::Text},
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};
use keyboard_protocol::{Key, StandardKey};

use crate::{
    gui::{
        self, colors, display,
        editor::line::{self, LineEditor},
        event::Event,
        home,
    },
    storage::NAME_LENGTH,
};

#[derive(Reactive)]
pub struct SaveDialog<'a> {
    pub state: SignalRef<'a, line::State<{ NAME_LENGTH }>>,
}

impl<'a> IntrinsicSize for SaveDialog<'a> {
    fn intrinsic_size(&self) -> Size {
        Size::new(200, 100)
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
    > for SaveDialog<'a>
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
        let is_submit = |event: &Event| match event {
            Event::Key { event, .. } => match event {
                gui::event::KeyEvent::Pressed(Key::Standard(StandardKey::Enter)) => true,
                _ => false,
            },
        };

        v.view(
            Direction::Vertical,
            [
                v.centered(
                    Direction::Horizontal,
                    v.primitive(
                        Sizing::Fill,
                        Text {
                            content: SignalRef::constant(&"Save as..."),
                            font_style: Signal::constant(
                                MonoTextStyleBuilder::new()
                                    .font(&FONT_10X20)
                                    .text_color(colors::TEXT)
                                    .underline()
                                    .build(),
                            ),
                        },
                    ),
                ),
                v.primitive(
                    Sizing::Intrinsic,
                    Spacer {
                        size: Signal::constant(Size::new(0, 20)),
                    },
                ),
                v.interactive(
                    home::FocusKey::SaveDialog,
                    move |event| {
                        if is_submit(&event) {
                            gui::Msg::CloseSaveDialog
                        } else {
                            gui::Msg::LineEditor(line::Msg::from_event(event))
                        }
                    },
                    move |_| v.primitive(Sizing::Intrinsic, LineEditor::new(self.state.clone())),
                ),
                v.spacer(),
                v.group(
                    Direction::Horizontal,
                    [
                        v.spacer(),
                        v.primitive(
                            Sizing::Intrinsic,
                            Text {
                                content: SignalRef::constant(&"[enter]"),
                                font_style: Signal::constant(
                                    MonoTextStyleBuilder::new()
                                        .font(&ascii::FONT_6X13_ITALIC)
                                        .text_color(colors::TEXT)
                                        .underline_with_color(colors::PURPLE)
                                        .build(),
                                ),
                            },
                        ),
                    ],
                ),
            ],
        )
    }
}
