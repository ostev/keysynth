#![no_std]

use defmt::Format;
use enumflags2::{BitFlags, bitflags, make_bitflags};
use serde::{Deserialize, Serialize};

pub mod encoder;
pub mod layout;
pub mod uart;

/// Represents a keyboard status update from the RP2040 with encoder information.
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyboardWithEncoderStatus {
    pub keyboard: KeyboardStatus,
    pub encoder: encoder::Update,
}

/// Represents what keys are pressed on the keyboard at any given moment.
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
    pub special_bitfield: BitFlags<SpecialKey>,
}

/// Represents the difference between two keyboard statuses.
pub struct KeyboardDiff<Added: Iterator<Item = Key>, Removed: Iterator<Item = Key>> {
    pub pressed: Added,
    pub released: Removed,
}

impl KeyboardStatus {
    #[inline]
    pub fn empty() -> KeyboardStatus {
        KeyboardStatus {
            keys: [StandardKey::None; 6],
            modifier_bitfield: BitFlags::empty(),
            special_bitfield: BitFlags::empty(),
        }
    }

    pub fn is_shift(&self) -> bool {
        self.modifier_bitfield
            .intersects(make_bitflags!(Modifier::{LeftShift | RightShift}))
    }

    pub fn is_super(&self) -> bool {
        self.modifier_bitfield
            .intersects(make_bitflags!(Modifier::{LeftSuper | RightSuper}))
    }

    /// Returns the difference between this keyboard status and a previous one.
    pub fn diff(
        self,
        previous: KeyboardStatus,
    ) -> KeyboardDiff<impl Iterator<Item = Key>, impl Iterator<Item = Key>> {
        let added_modifiers = self.modifier_bitfield & !previous.modifier_bitfield;
        let removed_modifiers = previous.modifier_bitfield & !self.modifier_bitfield;

        let added_special = self.special_bitfield & !previous.special_bitfield;
        let removed_special = previous.special_bitfield & !self.special_bitfield;

        // What standard keys have been added?
        let added_standard = self
            .keys
            .into_iter()
            .filter(move |key| !previous.keys.contains(key));
        // What standard keys have been released?
        let released_standard = previous
            .keys
            .into_iter()
            .filter(move |key| !self.keys.contains(key));

        // What keys have been pressed?
        let pressed = added_standard
            .map(|key| Key::Standard(key))
            .chain(added_modifiers.into_iter().map(Key::Modifier))
            .chain(added_special.into_iter().map(Key::Special));

        // What keys have been released?
        let released = released_standard
            .map(|key| Key::Standard(key))
            .chain(removed_modifiers.into_iter().map(Key::Modifier))
            .chain(removed_special.into_iter().map(Key::Special));

        KeyboardDiff { pressed, released }
    }
}

/// Represents a modifier on the keyboard
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

/// Represents a non-standard key on the keyboard
#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Format, Debug)]
pub enum SpecialKey {
    One = 0x01,
    Two = 0x02,
    Fn = 0x80,
}

/// Represents a regular key on the keyboard
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

    // Media control
    VolumeUp = 0x80,
    VolumeDown,
    Pause = 0x48,

    // Print screen
    SysRq = 0x46,

    // Forward delete
    Delete = 0x4c,

    // Page up/down
    PageUp = 0x4b,
    PageDown = 0x4e,
}

impl StandardKey {
    #[inline]
    pub fn is_arrow(self) -> bool {
        matches!(
            self,
            StandardKey::Right | StandardKey::Left | StandardKey::Down | StandardKey::Up
        )
    }

    pub fn to_char_lower(self) -> Option<u8> {
        match self {
            StandardKey::A => Some(b'a'),
            StandardKey::B => Some(b'b'),
            StandardKey::C => Some(b'c'),
            StandardKey::D => Some(b'd'),
            StandardKey::E => Some(b'e'),
            StandardKey::F => Some(b'f'),
            StandardKey::G => Some(b'g'),
            StandardKey::H => Some(b'h'),
            StandardKey::I => Some(b'i'),
            StandardKey::J => Some(b'j'),
            StandardKey::K => Some(b'k'),
            StandardKey::L => Some(b'l'),
            StandardKey::M => Some(b'm'),
            StandardKey::N => Some(b'n'),
            StandardKey::O => Some(b'o'),
            StandardKey::P => Some(b'p'),
            StandardKey::Q => Some(b'q'),
            StandardKey::R => Some(b'r'),
            StandardKey::S => Some(b's'),
            StandardKey::T => Some(b't'),
            StandardKey::U => Some(b'u'),
            StandardKey::V => Some(b'v'),
            StandardKey::W => Some(b'w'),
            StandardKey::X => Some(b'x'),
            StandardKey::Y => Some(b'y'),
            StandardKey::Z => Some(b'z'),
            StandardKey::Key1 => Some(b'1'),
            StandardKey::Key2 => Some(b'2'),
            StandardKey::Key3 => Some(b'3'),
            StandardKey::Key4 => Some(b'4'),
            StandardKey::Key5 => Some(b'5'),
            StandardKey::Key6 => Some(b'6'),
            StandardKey::Key7 => Some(b'7'),
            StandardKey::Key8 => Some(b'8'),
            StandardKey::Key9 => Some(b'9'),
            StandardKey::Key0 => Some(b'0'),
            StandardKey::Enter => Some(b'\n'),
            StandardKey::Space => Some(b' '),
            StandardKey::Minus => Some(b'-'),
            StandardKey::Equal => Some(b'='),
            StandardKey::LeftBracket => Some(b'['),
            StandardKey::RightBracket => Some(b']'),
            StandardKey::Backslash => Some(b'\\'),
            StandardKey::Semicolon => Some(b';'),
            StandardKey::Apostrophe => Some(b'\''),
            StandardKey::Grave => Some(b'`'),
            StandardKey::Comma => Some(b','),
            StandardKey::Dot => Some(b'.'),
            StandardKey::Slash => Some(b'/'),
            _ => None,
        }
    }

    pub fn to_char_upper(self) -> Option<u8> {
        match self {
            StandardKey::A => Some(b'A'),
            StandardKey::B => Some(b'B'),
            StandardKey::C => Some(b'C'),
            StandardKey::D => Some(b'D'),
            StandardKey::E => Some(b'E'),
            StandardKey::F => Some(b'F'),
            StandardKey::G => Some(b'G'),
            StandardKey::H => Some(b'H'),
            StandardKey::I => Some(b'I'),
            StandardKey::J => Some(b'J'),
            StandardKey::K => Some(b'K'),
            StandardKey::L => Some(b'L'),
            StandardKey::M => Some(b'M'),
            StandardKey::N => Some(b'N'),
            StandardKey::O => Some(b'O'),
            StandardKey::P => Some(b'P'),
            StandardKey::Q => Some(b'Q'),
            StandardKey::R => Some(b'R'),
            StandardKey::S => Some(b'S'),
            StandardKey::T => Some(b'T'),
            StandardKey::U => Some(b'U'),
            StandardKey::V => Some(b'V'),
            StandardKey::W => Some(b'W'),
            StandardKey::X => Some(b'X'),
            StandardKey::Y => Some(b'Y'),
            StandardKey::Z => Some(b'Z'),
            StandardKey::Key1 => Some(b'!'),
            StandardKey::Key2 => Some(b'@'),
            StandardKey::Key3 => Some(b'#'),
            StandardKey::Key4 => Some(b'$'),
            StandardKey::Key5 => Some(b'%'),
            StandardKey::Key6 => Some(b'^'),
            StandardKey::Key7 => Some(b'&'),
            StandardKey::Key8 => Some(b'*'),
            StandardKey::Key9 => Some(b'('),
            StandardKey::Key0 => Some(b')'),
            StandardKey::Enter => Some(b'\n'),
            StandardKey::Space => Some(b' '),
            StandardKey::Minus => Some(b'_'),
            StandardKey::Equal => Some(b'+'),
            StandardKey::LeftBracket => Some(b'{'),
            StandardKey::RightBracket => Some(b'}'),
            StandardKey::Backslash => Some(b'|'),
            StandardKey::Semicolon => Some(b':'),
            StandardKey::Apostrophe => Some(b'"'),
            StandardKey::Grave => Some(b'~'),
            StandardKey::Comma => Some(b'<'),
            StandardKey::Dot => Some(b'>'),
            StandardKey::Slash => Some(b'?'),
            _ => None,
        }
    }

    /// Converts the key into its character representation according
    /// to whether shift is pressed.
    pub fn to_char(self, is_shift: bool) -> Option<u8> {
        if is_shift {
            self.to_char_upper()
        } else {
            self.to_char_lower()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Key {
    Standard(StandardKey),
    Special(SpecialKey),
    Modifier(Modifier),
}

impl Key {
    #[inline]
    pub fn is_arrow(self) -> bool {
        match self {
            Key::Standard(standard_key) => standard_key.is_arrow(),
            _ => false,
        }
    }
}

// === Convenience macros ===

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
