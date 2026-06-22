#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use core::mem::MaybeUninit;
use core::panic::PanicInfo;

use alloc::vec;
use alloc::vec::Vec;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::Peripherals;
use esp_hal::ram;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, interrupt::software::SoftwareInterrupt};

use esp_alloc::heap_allocator;
use esp_backtrace as _;

use app::gui::{colors, display};
use app::hardware::Hardware;
use app::input::encoder;
use app::input::keyboard::keyboard_interface;
use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Ticker, Timer};
use esp_println::println;
use esp_rtos::embassy::{Executor, InterruptExecutor};
use static_cell::StaticCell;

use app::{audio, input};
use app::{gui, usb};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let hardware = Hardware::new().await.unwrap();

    encoder::configure(hardware.encoder);

    {
        static HIGH_PRIORITY_EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();

        let high_priority_executor =
            HIGH_PRIORITY_EXECUTOR.init_with(|| InterruptExecutor::new(hardware.interrupt_2));
        let high_priority = high_priority_executor.start(esp_hal::interrupt::Priority::Priority3);

        // Start the input tasks as high-priority interrupts
        high_priority.spawn(input::router().unwrap());
        high_priority.spawn(keyboard_interface(hardware.keyboard).unwrap());
    }

    {
        static STACK: StaticCell<esp_hal::system::Stack<{ usb::STACK_SIZE }>> = StaticCell::new();

        let stack = STACK.init_with(|| esp_hal::system::Stack::new());

        esp_rtos::start_second_core(hardware.cpu_control, hardware.interrupt_1, stack, || {
            // The USB device loop fully occupies the second core
            usb::device_loop(hardware.usb)
        });
    }

    spawner.spawn(gui::app(hardware.display, hardware.storage).unwrap());
}
