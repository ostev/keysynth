use embassy_executor::task;
use embassy_usb::class::hid::HidWriter;
use enumflags2::BitFlags;
use esp_println::println;
use keyboard_protocol::{Modifier, StandardKey};
use usbd_hid::descriptor::KeyboardReport;

use crate::hardware::UsbDriver;

mod channel {
    use crate::{channel, hid::UsbKeyboardStatus};

    channel! { UsbKeyboardStatus }
}

pub use channel::sender;

pub struct UsbKeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
}

pub struct UsbHidHardware {
    pub writer: HidWriter<'static, UsbDriver, 8>,
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
            Err(error) => println!(
                "Warning: failed to send keyboard HID report, {:?}, with the following error: {:?}",
                report, error
            ),
        }
    }
}
