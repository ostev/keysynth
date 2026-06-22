use circular_buffer::CircularBuffer;
use embassy_executor::task;
use esp_hal::{Blocking, uart::UartRx};
use esp_println::println;
use keyboard_protocol::KeyboardWithEncoderStatus;

mod channel {
    use keyboard_protocol::KeyboardWithEncoderStatus;

    use crate::channel;

    channel! { KeyboardWithEncoderStatus }
}

pub use channel::receiver;

pub struct KeyboardHardware {
    pub uart: UartRx<'static, Blocking>,
}

/// Handles UART communication from the RP2040.
#[task]
pub async fn keyboard_interface(hardware: KeyboardHardware) -> ! {
    const STATUS_SIZE: usize = core::mem::size_of::<KeyboardWithEncoderStatus>();

    let sender = channel::sender();
    let mut uart = hardware.uart.into_async();

    let mut buffer = CircularBuffer::<{ STATUS_SIZE + 2 }, u8>::new();

    loop {
        let mut receive_data = [0; STATUS_SIZE + 2];

        match uart.read_async(&mut receive_data).await {
            Ok(length) => {
                buffer.extend_from_slice(&receive_data[0..length]);
            }
            Err(error) => {
                println!("Warning: error receiving data from RP2040! {}", error)
            }
        }

        // Decode the message if the buffer contains the message start and end marker bytes
        // in the right positions.
        if buffer.len() >= STATUS_SIZE + 2 {
            if buffer[0] == keyboard_protocol::uart::START_BYTE
                && buffer[STATUS_SIZE + 1] == keyboard_protocol::uart::END_BYTE
            {
                let mut serialized = [0; STATUS_SIZE];

                for (index, byte) in buffer.iter().skip(1).take(STATUS_SIZE).enumerate() {
                    serialized[index] = *byte;
                }

                match postcard::from_bytes::<KeyboardWithEncoderStatus>(&serialized) {
                    Ok(status) => sender.send(status).await,
                    Err(err) => {
                        println!(
                            "Warning: failed to decode keyboard status (the array of bytes {:?}) from RP2040, with the following error: {:?}.",
                            receive_data, err
                        )
                    }
                };
            }
        }
    }
}
