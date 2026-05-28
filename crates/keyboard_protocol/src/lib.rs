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
    #[inline]
    pub fn empty() -> KeyboardStatus {
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

impl StandardKey {
    pub fn to_char_lower(self) -> Option<char> {
        match self {
            StandardKey::A => Some('a'),
            StandardKey::B => Some('b'),
            StandardKey::C => Some('c'),
            StandardKey::D => Some('d'),
            StandardKey::E => Some('e'),
            StandardKey::F => Some('f'),
            StandardKey::G => Some('g'),
            StandardKey::H => Some('h'),
            StandardKey::I => Some('i'),
            StandardKey::J => Some('j'),
            StandardKey::K => Some('k'),
            StandardKey::L => Some('l'),
            StandardKey::M => Some('m'),
            StandardKey::N => Some('n'),
            StandardKey::O => Some('o'),
            StandardKey::P => Some('p'),
            StandardKey::Q => Some('q'),
            StandardKey::R => Some('r'),
            StandardKey::S => Some('s'),
            StandardKey::T => Some('t'),
            StandardKey::U => Some('u'),
            StandardKey::V => Some('v'),
            StandardKey::W => Some('w'),
            StandardKey::X => Some('x'),
            StandardKey::Y => Some('y'),
            StandardKey::Z => Some('z'),
            StandardKey::Key1 => Some('1'),
            StandardKey::Key2 => Some('2'),
            StandardKey::Key3 => Some('3'),
            StandardKey::Key4 => Some('4'),
            StandardKey::Key5 => Some('5'),
            StandardKey::Key6 => Some('6'),
            StandardKey::Key7 => Some('7'),
            StandardKey::Key8 => Some('8'),
            StandardKey::Key9 => Some('9'),
            StandardKey::Key0 => Some('0'),
            StandardKey::Enter => Some('\n'),
            StandardKey::Space => Some(' '),
            StandardKey::Minus => Some('-'),
            StandardKey::Equal => Some('='),
            StandardKey::LeftBracket => Some('['),
            StandardKey::RightBracket => Some(']'),
            StandardKey::Backslash => Some('\\'),
            StandardKey::Semicolon => Some(';'),
            StandardKey::Apostrophe => Some('\''),
            StandardKey::Grave => Some('`'),
            StandardKey::Comma => Some(','),
            StandardKey::Dot => Some('.'),
            StandardKey::Slash => Some('/'),
            _ => None,
        }
    }

    pub fn to_char_upper(self) -> Option<char> {
        match self {
            StandardKey::A => Some('A'),
            StandardKey::B => Some('B'),
            StandardKey::C => Some('C'),
            StandardKey::D => Some('D'),
            StandardKey::E => Some('E'),
            StandardKey::F => Some('F'),
            StandardKey::G => Some('G'),
            StandardKey::H => Some('H'),
            StandardKey::I => Some('I'),
            StandardKey::J => Some('J'),
            StandardKey::K => Some('K'),
            StandardKey::L => Some('L'),
            StandardKey::M => Some('M'),
            StandardKey::N => Some('N'),
            StandardKey::O => Some('O'),
            StandardKey::P => Some('P'),
            StandardKey::Q => Some('Q'),
            StandardKey::R => Some('R'),
            StandardKey::S => Some('S'),
            StandardKey::T => Some('T'),
            StandardKey::U => Some('U'),
            StandardKey::V => Some('V'),
            StandardKey::W => Some('W'),
            StandardKey::X => Some('X'),
            StandardKey::Y => Some('Y'),
            StandardKey::Z => Some('Z'),
            StandardKey::Key1 => Some('!'),
            StandardKey::Key2 => Some('@'),
            StandardKey::Key3 => Some('#'),
            StandardKey::Key4 => Some('$'),
            StandardKey::Key5 => Some('%'),
            StandardKey::Key6 => Some('^'),
            StandardKey::Key7 => Some('&'),
            StandardKey::Key8 => Some('*'),
            StandardKey::Key9 => Some('('),
            StandardKey::Key0 => Some(')'),
            StandardKey::Enter => Some('\n'),
            StandardKey::Space => Some(' '),
            StandardKey::Minus => Some('_'),
            StandardKey::Equal => Some('+'),
            StandardKey::LeftBracket => Some('{'),
            StandardKey::RightBracket => Some('}'),
            StandardKey::Backslash => Some('|'),
            StandardKey::Semicolon => Some(':'),
            StandardKey::Apostrophe => Some('"'),
            StandardKey::Grave => Some('~'),
            StandardKey::Comma => Some('<'),
            StandardKey::Dot => Some('>'),
            StandardKey::Slash => Some('?'),
            _ => None,
        }
    }
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
