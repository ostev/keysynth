use keyboard_protocol::{Key, KeyboardStatus};

use crate::input::encoder::{self, EncoderStatus};

/// Represents a keyboard event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

/// An input event from either the encoders or the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// A change in input state from either the encoders or the keyboard.
pub enum InputChange {
    Keyboard(KeyboardStatus),
    Encoder(EncoderStatus),
}
