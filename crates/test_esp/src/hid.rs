use embassy_executor::task;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_usb::class::hid::HidWriter;
use enumflags2::BitFlags;
use esp_println::println;
use esp_sync::RawMutex;
use keyberon::key_code::KeyCode;
use keyboard_protocol::{Modifier, StandardKey};
use usbd_hid::descriptor::KeyboardReport;

use crate::{hardware::UsbDriver, sender};

sender! { UsbKeyboardStatus }

pub struct UsbKeyboardStatus {
    pub keys: [StandardKey; 6],
    pub modifier_bitfield: BitFlags<Modifier>,
}

pub struct UsbHidHardware {
    pub writer: HidWriter<'static, UsbDriver, 8>,
}

#[task]
pub async fn hid(mut hardware: UsbHidHardware) {
    let receiver = CHANNEL.receiver();

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
