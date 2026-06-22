mod clipboard;
mod cursor;
mod gap_buffer;
mod history;
pub mod line;
mod position;
pub mod source;
mod state;
mod view;

use embedded_gui::component::Component;
use embedded_gui::layout::Direction;
use embedded_gui::layout::IntrinsicSize;
use embedded_gui::layout::Sizing;
use embedded_gui::signal::Reactive;
use embedded_gui::signal::Signal;
use embedded_gui::signal::SignalRef;
use embedded_gui::size::Size;

pub use source::MAX_SIZE;
pub use state::EditorState as State;
pub use state::Msg;
pub use view::EditorView as View;

use crate::gui;
use crate::gui::display;
use crate::gui::text_bar::OwnedTextBar;
use crate::input::event::Event;

/// A multiline text editor.
#[derive(Reactive)]
pub struct Editor<'a> {
    state: SignalRef<'a, State>,
}

impl<'a> IntrinsicSize for Editor<'a> {
    fn intrinsic_size(&self) -> embedded_gui::size::Size {
        Size::new(display::WIDTH as u16, display::HEIGHT as u16)
    }
}

impl<'a> Editor<'a> {
    pub const fn new(state: SignalRef<'a, State>) -> Self {
        Self { state }
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
    > for Editor<'a>
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
        const BAR_HEIGHT: u16 = 20;
        let text = self.state.map(|state| {
            heapless::String::from_utf8(state.source.name.clone())
                .unwrap_or(heapless::format!("<corrupted>").unwrap())
        });

        v.view(
            Direction::Vertical,
            [
                // Bar at the top showing the file name
                v.component(Sizing::Constrained(BAR_HEIGHT), OwnedTextBar { text }, []),
                // The actual rendered text editor view
                v.interactive(gui::FocusKey::Editor, Msg::from_event, |focus_state| {
                    v.primitive(
                        Sizing::Fill,
                        View {
                            state: self.state.clone(),
                            is_active: Signal::constant(focus_state.is_focused()),
                        },
                    )
                }),
            ],
        )
    }
}
