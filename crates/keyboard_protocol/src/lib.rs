#![no_std]

use defmt::Format;
use enumflags2::{BitFlags, bitflags, make_bitflags};
use serde::{Deserialize, Serialize};

pub mod layout;
pub mod uart;

#[repr(C)]
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
    pub special_bitfield: BitFlags<SpecialKey>,
}

pub struct KeyboardDiff<Added: Iterator<Item = Key>, Removed: Iterator<Item = Key>> {
    pub pressed: Added,
    pub released: Removed,
}

impl KeyboardStatus {
    pub fn new() -> KeyboardStatus {
        KeyboardStatus {
            keys: [StandardKey::None; 6],
            modifier_bitfield: BitFlags::empty(),
            special_bitfield: BitFlags::empty(),
        }
    }

    pub fn diff(
        self,
        previous: KeyboardStatus,
    ) -> KeyboardDiff<impl Iterator<Item = Key>, impl Iterator<Item = Key>> {
        let added_modifiers = self.modifier_bitfield & !previous.modifier_bitfield;
        let removed_modifiers = previous.modifier_bitfield & !self.modifier_bitfield;

        let added_special = self.special_bitfield & !previous.special_bitfield;
        let removed_special = previous.special_bitfield & !self.special_bitfield;

        let added_standard = self
            .keys
            .into_iter()
            .filter(move |key| !previous.keys.contains(key));
        let removed_standard = previous
            .keys
            .into_iter()
            .filter(move |key| !self.keys.contains(key));

        let pressed = added_standard
            .map(|key| Key::Standard(key))
            .chain(added_modifiers.into_iter().map(Key::Modifier))
            .chain(added_special.into_iter().map(Key::Special));

        let released = removed_standard
            .map(|key| Key::Standard(key))
            .chain(removed_modifiers.into_iter().map(Key::Modifier))
            .chain(removed_special.into_iter().map(Key::Special));

        KeyboardDiff { pressed, released }
    }
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Format, Debug)]
pub enum Modifier {
    LeftControl = 0x01,
    LeftShift = 0x02,
    LeftAlt = 0x04,
    LeftSuper = 0x08,
    RightControl = 0x10,
    RightShift = 0x20,
    RightAlt = 0x40,
    RightSuper = 0x80,
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Format, Debug)]
pub enum SpecialKey {
    One = 0x01,
    Two = 0x02,
    Three = 0x04,
    Four = 0x08,
    Five = 0x10,
    Six = 0x20,
    Seven = 0x40,
    Fn = 0x80,
}

impl SpecialKey {
    // pub fn to_bitfield(keys: impl Iterator<Item = SpecialKey>) -> u8 {
    //     keys.map(|key| key as u8)
    //         .reduce(|key1, key2| key1 | key2)
    //         .unwrap_or(0)
    // }

    // pub fn matches(self, bitfield: u8) -> bool {
    //     (self as u8 & bitfield) != 0
    // }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Format)]
pub enum StandardKey {
    None = 0x00,
    /// Too many keys have been pressed
    Overfull = 0x01,

    // Letters
    A = 0x04,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers/symbols
    Key1 = 0x1e,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    // Special characters
    Enter = 0x28,
    Esc,
    Backspace,
    Tab,
    Space,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Tilde,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Dot,
    Slash,
    CapsLock,

    // Function row
    F1 = 0x3a,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Insertion characters
    Home = 0x4a,
    End = 0x4d,

    // Arrows
    Right = 0x4f,
    Left,
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Key {
    Standard(StandardKey),
    Special(SpecialKey),
    Modifier(Modifier),
}
#[macro_export]
macro_rules! standard {
    ($name:ident) => {
        $crate::Key::Standard($crate::StandardKey::$name)
    };
}

#[macro_export]
macro_rules! special {
    ($name:ident) => {
        $crate::Key::Special($crate::SpecialKey::$name)
    };
}

#[macro_export]
macro_rules! modifier {
    ($name:ident) => {
        $crate::Key::Modifier($crate::Modifier::$name)
    };
}
