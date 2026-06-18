#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![feature(allocator_api)]

extern crate alloc;

use embassy_executor::task;
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use keyboard_protocol::StandardKey;

use crate::{input::encoder::EncoderStatus, usb::hid::UsbKeyboardStatus};

pub mod audio;
pub mod concurrency;
pub mod gui;
pub mod hardware;
pub mod input;
pub mod storage;
// pub mod test_audio;
pub mod text;
pub mod usb;
mod utils;
