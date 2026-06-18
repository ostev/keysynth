use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use keyboard_protocol::{Key, KeyboardStatus};

pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

pub enum Event {
    Key {
        event: KeyEvent,
        keyboard: KeyboardStatus,
    },
}

pub enum InputChange {
    Keyboard(KeyboardStatus),
}
