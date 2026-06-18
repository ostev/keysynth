use circular_buffer::CircularBuffer;
use embassy_executor::task;
use esp_hal::{Async, Blocking, uart::UartRx};
use esp_println::println;
use keyboard_protocol::KeyboardStatus;

mod channel {
    use keyboard_protocol::KeyboardStatus;

    use crate::channel;

    channel! { KeyboardStatus }
}

pub use channel::receiver;

pub struct KeyboardHardware {
    pub uart: UartRx<'static, Blocking>,
}

#[task]
pub async fn keyboard_interface(hardware: KeyboardHardware) -> ! {
    const STATUS_SIZE: usize = core::mem::size_of::<KeyboardStatus>();

    let sender = channel::sender();
    let mut uart = hardware.uart.into_async();

    let mut buffer = CircularBuffer::<{ STATUS_SIZE + 2 }, u8>::new();

    loop {
        // println!("Start uart loop!");
        let mut receive_data = [0; STATUS_SIZE + 2];

        // println!("Wait for receive uart");

        match uart.read_async(&mut receive_data).await {
            Ok(length) => {
                buffer.extend_from_slice(&receive_data[0..length]);
            }
            Err(error) => {
                println!("Warning: key receive error! {}", error)
            }
        }
        // println!("receive uart!");
        // println!("buffer: {:?}", buffer);

        if buffer.len() >= STATUS_SIZE + 2 {
            // println!("Greater than!");
            // for (start_index, start_byte) in buffer.iter().enumerate() {
            if buffer[0] == keyboard_protocol::uart::START_BYTE
                && buffer[STATUS_SIZE + 1] == keyboard_protocol::uart::END_BYTE
            {
                let mut serialized = [0; STATUS_SIZE];

                for (index, byte) in buffer.iter().skip(1).take(STATUS_SIZE).enumerate() {
                    serialized[index] = *byte;
                }

                match postcard::from_bytes::<KeyboardStatus>(&serialized) {
                    Ok(status) => sender.send(status).await,
                    Err(err) => {
                        // println!(
                        //     "Warning: failed to decode keyboard status (the array of bytes {:?}) from RP2040, with the following error: {:?}.",
                        //     receive_data, err
                        // )
                    }
                };
            }
            // }
        }

        // println!("End uart loop!");
    }
}
