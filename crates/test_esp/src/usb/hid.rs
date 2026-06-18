use embassy_executor::task;
use embassy_usb::class::hid::HidWriter;
use enumflags2::BitFlags;
use keyboard_protocol::{Modifier, StandardKey};
use usbd_hid::descriptor::KeyboardReport;

mod channel {
    use crate::{channel, usb::hid::UsbKeyboardStatus};

    channel! { UsbKeyboardStatus }
}

pub use channel::sender;

use crate::usb;

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
}

pub struct UsbHidHardware {
    pub writer: HidWriter<'static, usb::Driver, 8>,
}

#[task]
pub async fn hid(mut hardware: UsbHidHardware) {
    let receiver = channel::receiver();

    loop {
        let UsbKeyboardStatus {
            keys,
            modifier_bitfield,
        } = receiver.receive().await;

        let report = KeyboardReport {
            // Safety: the `KeyCode`s are `#[repr(u8)]`, so we can safely
            // cast then into an array of u8s. Going the other direction could
            // cause issues, however.
            keycodes: unsafe { core::mem::transmute(keys) },
            leds: 0,
            modifier: modifier_bitfield.bits(),
            reserved: 0,
        };
        match hardware.writer.write_serialize(&report).await {
            Ok(()) => {}
            // Err(error) => println!(
            //     "Warning: failed to send keyboard HID report, {:?}, with the following error: {:?}",
            //     report, error
            // ),
            Err(error) => {}
        }
    }
}
