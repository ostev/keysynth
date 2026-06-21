use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use enumflags2::make_bitflags;
use esp_println::println;
use keyboard_protocol::{Key, KeyboardStatus, Modifier, StandardKey};
use synth::note::Note;

use crate::{
    gui,
    input::event::{Event, KeyEvent},
};

pub(super) fn handler<Msg: Into<gui::Msg>>(
    event: Event,
    down: Msg,
    up: Msg,
    enter: Msg,
) -> Option<Msg> {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Standard(key)) => match key {
                StandardKey::Down => Some(down),
                StandardKey::Up => Some(up),
                StandardKey::Enter => Some(enter),
                _ => None,
            },
            _ => None,
        },
        Event::Encoder { id, direction } => None,
    }
}

// pub(super) fn handler_lazy<Msg: Into<gui::Msg>>(
//     event: Event,
//     down: impl FnOnce() -> Msg,
//     up: impl FnOnce() -> Msg,
//     enter: impl FnOnce() -> Msg,
//     none: impl FnOnce() -> Msg,
// ) -> Msg {
//     match event {
//         Event::Key { event, .. } => match event {
//             KeyEvent::Pressed(Key::Standard(key)) => match key {
//                 StandardKey::Down => down(),
//                 StandardKey::Up => up(),
//                 StandardKey::Enter => enter(),
//                 _ => none(),
//             },
//             _ => none(),
//         },
//         _ => None,
//     }
// }

pub(super) fn on_keydown<Msg: Into<gui::Msg>>(
    unmodified: impl Fn(Key) -> Option<Msg>,
    with_super: impl Fn(Key) -> Option<Msg>,
) -> impl Fn(Event) -> Option<gui::Msg> {
    move |event| match event {
        Event::Key { event, keyboard } => match event {
            KeyEvent::Pressed(key) => {
                if keyboard
                    .modifier_bitfield
                    .intersects(make_bitflags!(Modifier::{LeftSuper | RightSuper}))
                {
                    with_super(key).map(|msg| msg.into())
                } else {
                    unmodified(key).map(|msg| msg.into())
                }
            }
            KeyEvent::Released(_) => None,
        },
        Event::Encoder { id, direction } => None,
    }
}
