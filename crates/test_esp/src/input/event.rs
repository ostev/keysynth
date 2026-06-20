use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use keyboard_protocol::{Key, KeyboardStatus};

use crate::input::encoder::{self, EncoderStatus};

pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

pub enum Event {
    Key {
        event: KeyEvent,
        keyboard: KeyboardStatus,
    },
    Encoder {
        id: encoder::Id,
        direction: encoder::Direction,
    },
}

pub enum InputChange {
    Keyboard(KeyboardStatus),
    Encoder(EncoderStatus),
}
