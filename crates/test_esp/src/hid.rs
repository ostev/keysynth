use embassy_executor::task;
use embassy_sync::channel::Receiver;
use embassy_usb::class::hid::HidWriter;
use esp_println::println;
use esp_sync::RawMutex;
use keyberon::key_code::KeyCode;
use usbd_hid::descriptor::KeyboardReport;

use crate::{
    hardware::UsbDriver,
    keyboard::{KeyboardStatus, StandardKey},
};

pub struct UsbKeyboardStatus {
    keys: [StandardKey; 6],
    modifier_bitfield: u8,
}

pub struct UsbHidHardware {
    pub writer: HidWriter<'static, UsbDriver, 8>,
}

#[task]
async fn hid(
    mut hardware: UsbHidHardware,
    receiver: Receiver<'static, RawMutex, UsbKeyboardStatus, 1>,
) {
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
            modifier: modifier_bitfield,
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
