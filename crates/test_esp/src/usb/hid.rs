use embassy_executor::task;
use enumflags2::{BitFlags, make_bitflags};
use keyboard_protocol::{Modifier, StandardKey};
use usbd_hid::descriptor::KeyboardReport;

pub const POLL_MS: u8 = 4;

#[derive(Clone, Copy, Debug)]
pub struct UsbKeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
}

impl UsbKeyboardStatus {
    pub fn press(key: StandardKey) -> UsbKeyboardStatus {
        let mut keys = [StandardKey::None; 6];
        keys[0] = key;

        UsbKeyboardStatus {
            keys,
            modifier_bitfield: BitFlags::empty(),
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

// pub struct UsbHidHardware {
// pub writer: HidWriter<'static, usb::Bus, 8>,
// }

// #[task]
// pub async fn hid(mut hardware: UsbHidHardware) {
//     let receiver = channel::receiver();

//     loop {
//         let UsbKeyboardStatus {
//             keys,
//             modifier_bitfield,
//         } = receiver.receive().await;

//         let report = KeyboardReport {
//             // Safety: the `KeyCode`s are `#[repr(u8)]`, so we can safely
//             // cast then into an array of u8s. Going the other direction could
//             // cause issues, however.
//             keycodes: unsafe { core::mem::transmute(keys) },
//             leds: 0,
//             modifier: modifier_bitfield.bits(),
//             reserved: 0,
//         };
//         match hardware.writer.write_serialize(&report).await {
//             Ok(()) => {}
//             // Err(error) => println!(
//             //     "Warning: failed to send keyboard HID report, {:?}, with the following error: {:?}",
//             //     report, error
//             // ),
//             Err(error) => {}
//         }
//     }
// }
