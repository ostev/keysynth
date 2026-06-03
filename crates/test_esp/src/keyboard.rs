use embassy_executor::task;
use esp_hal::{Async, uart::UartRx};
use esp_println::println;
use keyboard_protocol::KeyboardStatus;

mod channel {
    use keyboard_protocol::KeyboardStatus;

    use crate::channel;

    channel! { KeyboardStatus }
}

pub use channel::receiver;

pub struct KeyboardHardware {
    pub uart: UartRx<'static, Async>,
}

#[task]
pub async fn keyboard_interface(mut hardware: KeyboardHardware) -> ! {
    let sender = channel::sender();

    loop {
        const STATUS_SIZE: usize = core::mem::size_of::<KeyboardStatus>();

        let mut status_bytes = [0; STATUS_SIZE];

        match hardware.uart.read_async(&mut status_bytes).await {
            Ok(length) => {
                if length == STATUS_SIZE {
                    match postcard::from_bytes::<KeyboardStatus>(&status_bytes) {
                        Ok(status) => sender.send(status).await,
                        Err(err) => {
                            println!(
                                "Warning: failed to decode keyboard status (the array of bytes {:?}) from RP2040, with the following error: {:?}.",
                                status_bytes, err
                            )
                        }
                    }
                } else {
                    println!(
                        "Warning: buffer of length {length} received from keyboard UART when expected \
a buffer of length {STATUS_SIZE}. Here's the buffer received: {:?}",
                        status_bytes
                    )
                }
            }
            Err(_) => todo!(),
        }
    }
}
