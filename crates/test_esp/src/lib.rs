#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_sync::channel::Channel;

use crate::{hardware::Hardware, keyboard::Keyboard};

mod audio;
pub mod hardware;
mod hid;
mod keyboard;

pub struct App {
    audio: audio::Engine,
    keyboard: Keyboard,
}

impl App {
    pub fn new() {}
}
