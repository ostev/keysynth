#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::task;
use embassy_sync::channel::Channel;
use esp_println::println;

use crate::{hardware::Hardware, hid::UsbKeyboardStatus};

pub mod audio;
pub mod concurrency;
pub mod gui;
pub mod hardware;
pub mod hid;
pub mod keyboard;

pub enum KeyboardMode {
    Passthrough,
    Capture,
}

pub struct State {
    mode: KeyboardMode,
}

#[task]
pub async fn router() {
    let state = State {
        mode: KeyboardMode::Passthrough,
    };

    let keyboard = keyboard::receiver();
    let hid = hid::sender();

    loop {
        let keyboard_status = keyboard.receive().await;
        println!("status update!!!: {:?}", keyboard_status);

        match state.mode {
            KeyboardMode::Passthrough => {
                hid.send(UsbKeyboardStatus {
                    keys: keyboard_status.keys,
                    modifier_bitfield: keyboard_status.modifier_bitfield,
                })
                .await;
            }
            KeyboardMode::Capture => {}
        }
    }
}
