use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use keyboard_protocol::{Key, KeyboardStatus, Modifier, StandardKey};

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
    }
}

pub(super) fn handler_lazy<Msg: Into<gui::Msg>>(
    event: Event,
    down: impl FnOnce() -> Msg,
    up: impl FnOnce() -> Msg,
    enter: impl FnOnce() -> Msg,
    none: impl FnOnce() -> Msg,
) -> Msg {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Standard(key)) => match key {
                StandardKey::Down => down(),
                StandardKey::Up => up(),
                StandardKey::Enter => enter(),
                _ => none(),
            },
            _ => none(),
        },
    }
}

pub(super) fn on_keydown<Msg: Into<gui::Msg>>(
    unmodified: impl Fn(Key) -> Option<Msg>,
    with_super: impl Fn(Key) -> Option<Msg>,
) -> impl Fn(Event) -> Option<gui::Msg> {
    move |event| match event {
        Event::Key { event, keyboard } => match event {
            KeyEvent::Pressed(key) => {
                if keyboard.modifier_bitfield.contains(Modifier::LeftSuper)
                    || keyboard.modifier_bitfield.contains(Modifier::RightSuper)
                {
                    unmodified(key).map(|msg| msg.into())
                } else {
                    with_super(key).map(|msg| msg.into())
                }
            }
            KeyEvent::Released(_) => None,
        },
    }
}
