#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
#![feature(allocator_api)]

extern crate alloc;

pub mod audio;
pub mod concurrency;
pub mod gui;
pub mod hardware;
pub mod input;
pub mod storage;
pub mod text;
pub mod usb;
