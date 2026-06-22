use keyboard_protocol::Key;

use crate::{
    gui,
    input::event::{Event, KeyEvent},
};

/// Event handler for key presses.
pub(super) fn on_keydown<Msg: Into<gui::Msg>>(
    unmodified: impl Fn(Key) -> Option<Msg>,
    with_super: impl Fn(Key) -> Option<Msg>,
) -> impl Fn(Event) -> Option<gui::Msg> {
    move |event| match event {
        Event::Key { event, keyboard } => match event {
            KeyEvent::Pressed(key) => {
                if keyboard.is_super() {
                    with_super(key).map(|msg| msg.into())
                } else {
                    unmodified(key).map(|msg| msg.into())
                }
            }
            KeyEvent::Released(_) => None,
        },
        Event::Encoder { .. } => None,
    }
}
