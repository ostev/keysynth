#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![feature(generic_const_exprs)]

extern crate alloc;

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
pub mod storage;
pub mod text;
mod utils;

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
        mode: KeyboardMode::Capture,
    };

    let keyboard = keyboard::receiver();
    let hid = hid::sender();
    let gui = gui::sender();

    loop {
        let keyboard_status = keyboard.receive().await;

        match state.mode {
            KeyboardMode::Passthrough => {
                hid.send(UsbKeyboardStatus {
                    keys: keyboard_status.keys,
                    modifier_bitfield: keyboard_status.modifier_bitfield,
                })
                .await;
            }
            KeyboardMode::Capture => {
                gui.send(gui::event::InputChange::Keyboard(keyboard_status))
                    .await
            }
        }
    }
}
