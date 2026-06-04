#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, interrupt::software::SoftwareInterrupt};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_println::println;
use test_esp::hardware::Hardware;
use test_esp::hid::hid;
use test_esp::keyboard::keyboard_interface;
use test_esp::router;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    let hardware = Hardware::new().unwrap();

    esp_rtos::start(
        hardware.timer_group_0.timer0,
        hardware.context_switch_interrupt.software_interrupt0,
    );

    spawner.spawn(router().unwrap());
    // spawner.spawn(hid(hardware.hid).unwrap());
    spawner.spawn(keyboard_interface(hardware.keyboard).unwrap());

    loop {
        println!("heyyy!!!");
        // hardware.debug_uart.write("Hello!!!".as_bytes()).unwrap();
        // let mut buf: [u8; 1] = [0; 1];
        // hardware.debug_uart.read(&mut buf).unwrap();
        // println!(
        //     "heyyy!!! here's some data: {}",
        //     str::from_utf8(&buf).unwrap()
        // );
        Timer::after(Duration::from_secs(2)).await;
    }
}
