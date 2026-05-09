use bytemuck::{Contiguous, NoUninit, Pod, Zeroable};
use embassy_executor::task;
use embassy_sync::channel::Sender;
use esp_hal::{Async, uart::UartRx};
use esp_println::println;
use esp_sync::RawMutex;
use keyberon::key_code::KeyCode;
use serde::{Deserialize, Serialize};

pub const MAX_KEYS_PRESSED: usize = 10;

pub struct Keyboard {
    keys_pressed: [KeyCode; 6],
    modifiers_pressed: [KeyCode; 6],
    hardware: KeyboardHardware,
}

pub struct KeyboardHardware {
    pub uart: UartRx<'static, Async>,
}

#[repr(C)]
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: u8,
    pub special_bitfield: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, NoUninit, Clone, Copy, PartialEq, Eq, Debug)]
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

impl Modifier {
    pub fn to_bitfield(modifiers: impl Iterator<Item = Modifier>) -> u8 {
        modifiers
            .map(|modifier| modifier as u8)
            .reduce(|modifier1, modifier2| modifier1 | modifier2)
            .unwrap_or(0)
    }

    pub fn matches(self, bitfield: u8) -> bool {
        (self as u8 & bitfield) != 0
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, NoUninit, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpecialKey {
    SpecialOne = 0x01,
    SpecialTwo = 0x02,
    SpecialThree = 0x04,
    SpecialFour = 0x08,
    SpecialFive = 0x10,
    SpecialSix = 0x20,
    SpecialSeven = 0x40,
    SpecialEight = 0x80,
}

impl SpecialKey {
    pub fn to_bitfield(keys: impl Iterator<Item = SpecialKey>) -> u8 {
        keys.map(|key| key as u8)
            .reduce(|key1, key2| key1 | key2)
            .unwrap_or(0)
    }

    pub fn matches(self, bitfield: u8) -> bool {
        (self as u8 & bitfield) != 0
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, NoUninit, Clone, Copy, PartialEq, Eq, Debug)]
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
    LeftBrace,
    RightBrace,
    Backslash,
    Tilde,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Dot,
    Slash,

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

    // Arrows
    Right,
    Left,
    Down,
    Up,
}

const MAX_KEYBOARD_MESSAGE_SIZE: usize = 6 + 8; // Maximum number of keys and modifiers pressed

#[task]
async fn keyboard_interface(
    mut hardware: KeyboardHardware,
    sender: Sender<'static, RawMutex, KeyboardStatus, 1>,
) {
    loop {
        let mut status_bytes = [0; MAX_KEYBOARD_MESSAGE_SIZE];
        match hardware.uart.read_async(&mut status_bytes).await {
            Ok(length) => {
                if length == MAX_KEYBOARD_MESSAGE_SIZE {
                    match postcard::from_bytes::<KeyboardStatus>(&status_bytes) {
                        Ok(status) => sender.send(status).await,
                        Err(err) => {
                            println!(
                                "Warning: failed to decode keyboard status (the array of bytes {:?}) from RP2040, with the following error: {:?}.",
                                status_bytes, err
                            )
                        }
                    }
                } else {
                    println!(
                        "Warning: buffer of length {length} received from keyboard UART when expected \
a buffer of length {MAX_KEYBOARD_MESSAGE_SIZE}. Here's the buffer received: {:?}",
                        status_bytes
                    )
                }
            }
            Err(_) => todo!(),
        }
    }
}
