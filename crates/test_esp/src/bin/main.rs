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

use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Ticker, Timer};
use esp_println::println;
use esp_rtos::embassy::{Executor, InterruptExecutor};
use static_cell::StaticCell;
use test_esp::gui::{colors, display};
use test_esp::hardware::Hardware;
use test_esp::input::encoder;
use test_esp::input::keyboard::keyboard_interface;

use test_esp::{audio, input};
use test_esp::{gui, usb};

esp_bootloader_esp_idf::esp_app_desc!();

// fn init_heap() {
//     const HEAP_SIZE: usize = 32 * 1024;
//     static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

//     let ptr = core::ptr::addr_of_mut!(HEAP);

//     unsafe {
//         esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
//             ptr as *mut u8,
//             HEAP_SIZE,
//             esp_alloc::MemoryCapability::Internal.into(),
//         ));
//     }
// }

// #[allow(
//     clippy::large_stack_frames,
//     reason = "it's not unusual to allocate larger buffers etc. in main"
// )]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let hardware = Hardware::new().await.unwrap();

    // {
    //     static HIGH_PRIORITY_EXECUTOR: StaticCell<InterruptExecutor<1>> = StaticCell::new();

    //     let medium_priority_executor =
    //         HIGH_PRIORITY_EXECUTOR.init_with(|| InterruptExecutor::new(hardware.interrupt_2));
    //     let medium_priority =
    //         medium_priority_executor.start(esp_hal::interrupt::Priority::Priority2);

    //     // input_spawner.spawn(encoder(hardware.encoder).unwrap());
    // }

    encoder::configure(hardware.encoder);

    {
        static HIGH_PRIORITY_EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();

        let high_priority_executor =
            HIGH_PRIORITY_EXECUTOR.init_with(|| InterruptExecutor::new(hardware.interrupt_2));
        let high_priority = high_priority_executor.start(esp_hal::interrupt::Priority::Priority3);

        high_priority.spawn(input::router().unwrap());
        high_priority.spawn(keyboard_interface(hardware.keyboard).unwrap());
        // high_priority.spawn(
        //     usb_device(
        //         high_priority,
        //         hardware.cpu_control,
        //         hardware.interrupt_1,
        //         hardware.usb,
        //     )
        //     .unwrap(),
        // );
        high_priority.spawn(say_hi().unwrap());
    }

    {
        static STACK: StaticCell<esp_hal::system::Stack<{ usb::STACK_SIZE }>> = StaticCell::new();

        let stack = STACK.init_with(|| esp_hal::system::Stack::new());

        esp_rtos::start_second_core(hardware.cpu_control, hardware.interrupt_1, stack, || {
            usb::device_loop(hardware.usb)
        });
    }

    spawner.spawn(gui::app(hardware.display, hardware.storage).unwrap());

    // let mut driver = display::Driver::init(hardware.display, display::Orientation::Vertical).await;

    // driver.clear_async(colors::PURPLE).await;
    // println!("cleared!");
}

#[task]
async fn say_hi() -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(500));

    loop {
        println!("HEYAAAA!!!!");
        ticker.next().await;
    }
}
