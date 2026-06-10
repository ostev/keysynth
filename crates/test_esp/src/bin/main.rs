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

use alloc::vec::Vec;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::Peripherals;
use esp_hal::ram;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, interrupt::software::SoftwareInterrupt};

use esp_alloc::heap_allocator;
use esp_backtrace as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_println::println;
use test_esp::hardware::Hardware;
use test_esp::hid::hid;
use test_esp::keyboard::keyboard_interface;
use test_esp::router;

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

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    println!("Hello");

    let hardware = Hardware::new().await.unwrap();

    let mut its_a_vec = Vec::new();
    its_a_vec.push(3);

    println!("Helloooo");

    // let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // let context_switch_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // let timer_group_0 = TimerGroup::new(peripherals.TIMG0);

    // let cpu_control = CpuControl::new(peripherals.CPU_CTRL);

    // esp_rtos::start(
    //     hardware.timer_group_0.timer0,
    //     hardware.context_switch_interrupt.software_interrupt0,
    // );

    // spawner.spawn(router().unwrap());
    // // spawner.spawn(hid(hardware.hid).unwrap());
    // spawner.spawn(keyboard_interface(hardware.keyboard).unwrap());

    loop {
        println!("heyyy!!! {}", its_a_vec.last().unwrap_or(&0));
        // its_a_vec.push(its_a_vec.last().unwrap_or(&0) + 1);
        // hardware.debug_uart.write("Hello!!!".as_bytes()).unwrap();
        // let mut buf: [u8; 1] = [0; 1];
        // hardware.debug_uart.read(&mut buf).unwrap();
        // println!(
        //     "heyyy!!! here's some data: {}",
        //     str::from_utf8(&buf).unwrap()
        // );
        // Timer::after(Duration::from_secs(2)).await;
    }
}
