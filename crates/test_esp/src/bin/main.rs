#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bumpalo::Bump;
use calc::interpreter::Value;

use esp_hal::{
    clock::CpuClock,
    main,
    rtc_cntl::Rtc,
    time::{self, Duration, Instant},
    timer::{systimer::SystemTimer, timg::TimerGroup},
};
use esp_println::println;

use rpds::{List, ht_map, list};
use synth::wavetable::{self, DEFAULT_SIZE, DefaultWavetable, Oscillator};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static TEST_FILE: &'static str = include_str!("../../test.calc");

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let clock = Rtc::new(peripherals.LPWR);

    println!("Parsing...");
    let ast_arena = Bump::new();
    let parsed = calc::parser::parse(&ast_arena, TEST_FILE).unwrap();

    println!("Evaluating...");

    let start = clock.current_time_us();

    let table = DefaultWavetable::from_fn(|x| {
        let value = {
            let outer_scope = list![ht_map!["x" => Value::Number(x)]];

            let (dynamic_value, _) = calc::interpreter::eval(&parsed, outer_scope).unwrap();

            match dynamic_value {
                Value::Number(value) => value,
                _ => panic!("Didn't return a number!"),
            }
        };

        value
    });

    let end = clock.current_time_us();

    let duration = (end - start) as f64 / 1e6;

    println!("Done! It took {} seconds.", duration);
    println!("({} per sample)", duration / DEFAULT_SIZE as f64);

    let mut oscillator = Oscillator::new(&table, wavetable::DEFAULT_SAMPLE_RATE);

    loop {
        let sample = oscillator.sample(261.63);
        println!("Sample: {}", sample);

        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
        // println!("Hiii!!")
    }
}
