use enumflags2::BitFlags;
use keyboard_protocol::{Modifier, StandardKey};
use usbd_hid::descriptor::KeyboardReport;

/// How frequently the HID device is polled by the host.
pub const POLL_MS: u8 = 4;

/// Represents the keyboard status as it will be transmitted
/// over USB.
#[derive(Clone, Copy, Debug)]
pub struct UsbKeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
}

impl UsbKeyboardStatus {
    pub const fn press(key: StandardKey) -> UsbKeyboardStatus {
        let mut keys = [StandardKey::None; 6];
        keys[0] = key;

        UsbKeyboardStatus {
            keys,
            modifier_bitfield: BitFlags::EMPTY,
        }
    }
    pub const fn empty() -> UsbKeyboardStatus {
        UsbKeyboardStatus {
            keys: [StandardKey::None; 6],
            modifier_bitfield: BitFlags::EMPTY,
        }
    }
}

impl Into<KeyboardReport> for UsbKeyboardStatus {
    fn into(self) -> KeyboardReport {
        KeyboardReport {
            // Safety: the `KeyCode`s are `#[repr(u8)]`, so we can safely
            // cast then into an array of u8s. Going the other direction could
            // cause issues, however.
            keycodes: unsafe { core::mem::transmute(self.keys) },
            leds: 0,
            modifier: self.modifier_bitfield.bits(),
            reserved: 0,
        }
    }
}
