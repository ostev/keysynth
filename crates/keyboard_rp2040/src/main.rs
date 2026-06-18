#![no_std]
#![no_main]

use core::{
    char::MAX,
    ptr::{self, addr_of_mut},
};
use defmt::println;
use embassy_executor::{Executor, Spawner, task};
use embassy_rp::{
    bind_interrupts,
    clocks::ClockConfig,
    dma,
    gpio::{AnyPin, Input, Level, Output, Pin, Pull},
    multicore::Stack,
    peripherals::{DMA_CH0, UART0, UART1},
    pio::{self, Pio, program::pio_asm},
    uart::{self, BufferedInterruptHandler, BufferedUartTx, UartTx},
};
use embassy_sync::{
    blocking_mutex::{CriticalSectionMutex, raw::CriticalSectionRawMutex},
    channel::{Channel, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_io::Write;
use enumflags2::BitFlags;
use keyboard_protocol::{
    Key, KeyboardStatus, Modifier, SpecialKey,
    StandardKey::{self, A},
    layout::{self, Layout},
    uart::BAUDRATE,
};
use modular_bitfield::{
    bitfield,
    specifiers::{B7, B15},
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

static CHANNEL: Channel<CriticalSectionRawMutex, KeyboardStatus, 64> = Channel::new();

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

const NUM_ROWS: usize = 6;
const NUM_COLUMNS: usize = 14;

#[cortex_m_rt::entry]
fn main() -> ! {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::default());

    let keyboard_hardware: HardwareLayout<NUM_ROWS, NUM_COLUMNS> = HardwareLayout {
        rows: [
            Input::new(peripherals.PIN_19, Pull::Down),
            Input::new(peripherals.PIN_20, Pull::Down),
            Input::new(peripherals.PIN_21, Pull::Down),
            Input::new(peripherals.PIN_22, Pull::Down),
            Input::new(peripherals.PIN_23, Pull::Down),
            Input::new(peripherals.PIN_24, Pull::Down),
        ],
        columns: [
            Output::new(peripherals.PIN_18, Level::Low),
            Output::new(peripherals.PIN_17, Level::Low),
            Output::new(peripherals.PIN_16, Level::Low),
            Output::new(peripherals.PIN_15, Level::Low),
            Output::new(peripherals.PIN_14, Level::Low),
            Output::new(peripherals.PIN_13, Level::Low),
            Output::new(peripherals.PIN_12, Level::Low),
            Output::new(peripherals.PIN_11, Level::Low),
            Output::new(peripherals.PIN_10, Level::Low),
            Output::new(peripherals.PIN_9, Level::Low),
            Output::new(peripherals.PIN_8, Level::Low),
            Output::new(peripherals.PIN_7, Level::Low),
            Output::new(peripherals.PIN_6, Level::Low),
            Output::new(peripherals.PIN_5, Level::Low),
        ],
    };

    embassy_rp::multicore::spawn_core1(
        peripherals.CORE1,
        unsafe { &mut *addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| spawner.spawn(keyboard(keyboard_hardware).unwrap()));
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());

    let uart = UartTx::new(
        peripherals.UART1,
        peripherals.PIN_4,
        peripherals.DMA_CH0,
        Irqs,
        {
            let mut config = uart::Config::default();
            config.baudrate = BAUDRATE;

            config
        },
    );

    executor0.run(|spawner| spawner.spawn(uart_communicator(uart).unwrap()))
}

#[task]
async fn uart_communicator(mut uart: UartTx<'static, uart::Async>) {
    let receiver = CHANNEL.receiver();
    let mut counter = 0;

    loop {
        let status = receiver.receive().await;

        let mut output_buffer = [0; core::mem::size_of::<KeyboardStatus>() + 2];
        {
            let message_end = output_buffer.len() - 1;
            let mut message_portion = &mut output_buffer[1..message_end];
            postcard::to_slice(&status, &mut message_portion).unwrap();
        }

        output_buffer[0] = keyboard_protocol::uart::START_BYTE;
        output_buffer[output_buffer.len() - 1] = keyboard_protocol::uart::END_BYTE;

        match uart.write(&output_buffer).await {
            Ok(_) => {}
            Err(error) => defmt::warn!(
                "Failed to send keyboard status over UART! Here's the error {:?}",
                error
            ),
        }
    }
}

#[task]
async fn keyboard(mut hardware: HardwareLayout<NUM_ROWS, NUM_COLUMNS>) {
    let sender = CHANNEL.sender();
    let mut previous_status = None;

    let mut debouncer = Debouncer::new();

    loop {
        let status = scan_keyboard(&layout::STANDARD, &mut debouncer, &mut hardware);

        if Some(status) != previous_status {
            sender.send(status).await;
        }

        previous_status = Some(status);
    }
}

struct HardwareLayout<const R: usize, const C: usize> {
    rows: [Input<'static>; R],
    columns: [Output<'static>; C],
}

impl<const R: usize, const C: usize> HardwareLayout<R, C> {
    // fn rows_as_bitmask(&self) -> u32 {
    //     self.rows.iter().fold(0, |mask, (pin, _)| mask | 1 << pin)
    // }
}

// #[bitfield]
#[derive(Copy, Clone)]
struct DebouncedKey {
    // row: u8,
    // column: u8,
    // is_pressed: bool,
    // elapsed_since_change: u8,
    // /// The state of the key is stored in the lowest significant bit, while
    // /// the elapsed time in half-milliseconds since it changed is stored in the upper 7 bits.
    // is_pressed_and_elasped_half_ms_since_change: u8,
    is_pressed: bool,
    last_changed: Instant,
}
impl DebouncedKey {
    // pub const fn new(is_pressed: bool, elasped_half_ms_since_change: u8) -> DebouncedKey {
    //     DebouncedKey {
    //         is_pressed_and_elasped_half_ms_since_change: is_presse
    //     }
    // }
    pub fn new(is_pressed: bool) -> DebouncedKey {
        DebouncedKey {
            is_pressed: false,
            last_changed: Instant::now(),
        }
    }
}

struct Debouncer<const R: usize, const C: usize> {
    keys: [[DebouncedKey; C]; R],
}

impl<const R: usize, const C: usize> Debouncer<R, C> {
    pub fn new() -> Self {
        Self {
            keys: core::array::repeat(core::array::repeat(DebouncedKey::new(false))),
            // keys: [[DebouncedKey::new(false); C]; R],
        }
    }

    pub fn update(&mut self, row: usize, column: usize, raw_is_pressed: bool) -> bool {
        const INITIAL_DEBOUNCE_DURATION: Duration = Duration::from_millis(2);
        const DEFER_DEBOUNCE_DURATION: Duration = Duration::from_millis(3);
        // const _: () = {
        //     // The debounce duration must be less than the maximum elapsed
        //     // time we can store minus one, otherwise we can't count it.
        //     // We use the maximum value as a specific state value, so it must be
        //     // less than the max minus one.
        //     assert!(INITIAL_DEBOUNCE_CYCLE_DURATION < MAX_ELAPSED_CYCLES - 1);
        //     assert!(DEFER_DEBOUNCE_CYCLE_DURATION < MAX_ELAPSED_CYCLES - 1);
        // };

        let key = &mut self.keys[row][column];
        let debounced = key.is_pressed;
        let elapsed = Instant::now() - key.last_changed;

        // if is_pressed {
        //     if previous_is_pressed {
        //         if previous_elapsed_cycles >= INITIAL_DEBOUNCE_CYCLE_DURATION {
        //             true
        //         } else {
        //             key.set_elapsed_cycles(u16::min(
        //                 previous_elapsed_cycles + 1,
        //                 INITIAL_DEBOUNCE_CYCLE_DURATION,
        //             ));
        //             false
        //         }
        //     } else {
        //         key.set_is_pressed(true);
        //         key.set_elapsed_cycles(0);

        //         false
        //     }
        // } else {
        //     if !previous_is_pressed {
        //         if previous_elapsed_cycles >= DEFER_DEBOUNCE_CYCLE_DURATION {
        //             false
        //         } else {
        //             key.set_elapsed_cycles(u16::min(
        //                 previous_elapsed_cycles + 1,
        //                 DEFER_DEBOUNCE_CYCLE_DURATION,
        //             ));
        //             true
        //         }
        //     } else {
        //         key.set_is_pressed(false);
        //         key.set_elapsed_cycles(0);
        //         true
        //     }
        // }

        if raw_is_pressed == debounced {
            return debounced;
        }

        // raw != debounced -> start/count debounce timer
        let threshold = if raw_is_pressed {
            INITIAL_DEBOUNCE_DURATION
        } else {
            DEFER_DEBOUNCE_DURATION
        };

        // let next = u16::min(elapsed.saturating_add(1), threshold);
        // key.set_elapsed_cycles(next);

        if elapsed >= threshold {
            // key.set_is_pressed(raw_is_pressed);
            // key.set_elapsed_cycles(threshold);
            key.is_pressed = raw_is_pressed;
            key.last_changed = Instant::now();
            raw_is_pressed
        } else {
            // still debouncing: keep reporting the old (debounced) state
            debounced
        }
    }
}

fn scan_keyboard<const R: usize, const C: usize>(
    layout: &Layout<R, C>,
    debouncer: &mut Debouncer<R, C>,
    hardware: &mut HardwareLayout<R, C>,
) -> KeyboardStatus {
    // let row_bitmask = hardware.rows_as_bitmask();

    // let sio_ptr = rp_pac::SIO.as_ptr();
    // let sio_in = unsafe { sio_ptr.add(0x004) } as *const u32;
    // let sio_out = unsafe { sio_ptr.add(0x010) } as *mut u32;
    // let sio_out_set = unsafe { sio_ptr.add(0x014) } as *mut u32;
    // let sio_out_clr = unsafe { sio_ptr.add(0x018) } as *mut u32;
    // let sio_out_xor = unsafe { sio_ptr.add(0x01c) } as *mut u32;

    // for column in hardware.columns.iter_mut() {
    //     column.set_high();
    // }

    // embassy_time::block_for(Duration::from_millis(5));

    // for row in hardware.rows.iter() {
    // defmt::println!("level: {}", row.get_level());
    //     if (row.is_high()) {
    //         defmt::println!("is high");
    //     }
    // }

    // let active_keys =
    //     hardware
    //         .columns
    //         .iter_mut()
    //         .enumerate()
    //         .flat_map(|(column_index, column_output)| {
    //             // let debouncer = &debouncer;
    //             // // Only the pin to be toggled on
    //             // let output_bitmask: u32 = 1 << pin;

    //             // // TODO: investigate whether these are atomic as a group?
    //             // // This is a potential safety issue.
    //             // unsafe {
    //             //     // Turn all row pins off atomically.
    //             //     ptr::write(sio_out_clr, row_bitmask);
    //             //     // Turn only the output pin on atomically.
    //             //     ptr::write(sio_out_set, output_bitmask);
    //             // }
    //         });

    let mut standard_keys = [StandardKey::None; 6];
    let mut num_standard_keys: usize = 0;

    let mut modifier_bitfield = BitFlags::empty();
    let mut special_bitfield = BitFlags::empty();

    for (column_index, column_output) in hardware.columns.iter_mut().enumerate() {
        column_output.set_high();

        // Small delay required for pin state to change
        embassy_time::block_for(Duration::from_micros(2));

        for (row_index, row_input) in hardware.rows.iter().enumerate() {
            let is_pressed = debouncer.update(row_index, column_index, row_input.is_high());

            if is_pressed {
                let key = layout.rows[row_index][column_index];

                match key {
                    Key::Standard(key) => {
                        // Ignore keys past the maximum allowed by the USB spec
                        if num_standard_keys < standard_keys.len() {
                            standard_keys[num_standard_keys] = key;
                            num_standard_keys += 1;
                        }
                    }
                    Key::Modifier(modifier) => {
                        modifier_bitfield |= modifier;
                    }
                    Key::Special(special) => {
                        special_bitfield |= special;
                    }
                }
            }
        }

        column_output.set_low();
    }

    KeyboardStatus {
        keys: standard_keys,
        modifier_bitfield,
        special_bitfield,
    }
}
