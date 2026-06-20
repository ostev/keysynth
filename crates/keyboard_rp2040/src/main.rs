#![no_std]
#![no_main]

use crate::keyboard::HardwareLayout;
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
    usb::In,
};
use embassy_sync::{
    blocking_mutex::{CriticalSectionMutex, raw::CriticalSectionRawMutex},
    channel::{Channel, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::InputPin;
use embedded_io::Write;
use enumflags2::BitFlags;
use keyboard_protocol::{
    Key, KeyboardStatus, KeyboardWithEncoderStatus, Modifier, SpecialKey,
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

mod encoders;
mod keyboard;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::default());

    let keyboard_hardware: HardwareLayout<{ keyboard::NUM_ROWS }, { keyboard::NUM_COLUMNS }> =
        HardwareLayout {
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

    let encoder_hardware = encoders::Hardware::new(
        encoders::Unit::new(
            Input::new(peripherals.PIN_27, Pull::Up),
            Input::new(peripherals.PIN_26, Pull::Up),
        ),
        encoders::Unit::new(
            Input::new(peripherals.PIN_29, Pull::Up),
            Input::new(peripherals.PIN_28, Pull::Up),
        ),
    );

    embassy_rp::multicore::spawn_core1(
        peripherals.CORE1,
        unsafe { &mut *addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner.spawn(poll_loop(keyboard_hardware, encoder_hardware).unwrap())
            });
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

    loop {
        let status = receiver.receive().await;

        let mut output_buffer = [0; core::mem::size_of::<KeyboardWithEncoderStatus>() + 2];
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

static CHANNEL: Channel<CriticalSectionRawMutex, KeyboardWithEncoderStatus, 64> = Channel::new();

#[task]
pub async fn poll_loop(
    mut keyboard_hardware: HardwareLayout<{ keyboard::NUM_ROWS }, { keyboard::NUM_COLUMNS }>,
    mut encoder_hardware: encoders::Hardware<
        Input<'static>,
        Input<'static>,
        Input<'static>,
        Input<'static>,
    >,
) {
    let sender = CHANNEL.sender();
    let mut previous_keyboard_status = None;

    let mut debouncer = keyboard::Debouncer::new();

    loop {
        // First scan the keyboard matrix
        let keyboard_status = keyboard_hardware.scan(&layout::STANDARD, &mut debouncer);

        // Then scan the encoders
        let encoder_update = encoder_hardware.scan();

        // If either changed, send the status update!
        if !encoder_update.is_zero() || Some(keyboard_status) != previous_keyboard_status {
            sender
                .send(KeyboardWithEncoderStatus {
                    keyboard: keyboard_status,
                    encoder: encoder_update,
                })
                .await;
        }

        previous_keyboard_status = Some(keyboard_status);
    }
}
