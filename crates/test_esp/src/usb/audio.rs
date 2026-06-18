use core::sync::atomic::AtomicBool;

use embassy_executor::{SendSpawner, task};
use embassy_sync::{lazy_lock::LazyLock, once_lock::OnceLock};
use embassy_time::{Duration, Ticker};
use embassy_usb::{
    class::uac1::{self, source::AudioSourceEpIn},
    driver::EndpointError,
};
use esp_println::println;
use synth::Sample;

use crate::{audio, concurrency::receive_all, usb};

mod channel {
    use crate::{channel, usb::audio::AudioPacket};

    channel!(AudioPacket, 4);
}

pub use channel::sender;

pub struct Hardware {
    pub audio_endpoint: uac1::source::AudioSourceEpIn<'static, usb::Driver>,
    pub feedback_endpoint: uac1::source::AudioSourceEpIn<'static, usb::Driver>,
}

pub type AudioPacket = [Sample; ((AUDIO_REFRESH_MS as f32 / 1000.0) * SAMPLE_RATE as f32) as usize];

pub const AUDIO_REFRESH_MS: u8 = 2;
pub const SAMPLE_RATE: u32 = 48_000;

#[task]
async fn feedback(mut feedback_endpoint: AudioSourceEpIn<'static, usb::Driver>) {
    // Convert the sample rate to USB's 3-byte 10.14 sample rate format by multiply it by
    // 2^14 and then taking the left three bytes in little-endian order.
    let feedback_buffer = &(SAMPLE_RATE << 14).to_le_bytes()[..3];

    let mut ticker = Ticker::every(Duration::from_millis(AUDIO_REFRESH_MS.into()));

    loop {
        feedback_endpoint.wait_enabled().await;

        match feedback_endpoint.write(feedback_buffer).await {
            Ok(_) => {
                // Wait until the next audio frame to send the updated sample rate
                ticker.next().await;
            }
            Err(error) => {
                println!(
                    "Error: failed to write to audio feedback buffer ({:?})",
                    error
                );
                // Short delay before retrying in case the error is transient
                embassy_time::Timer::after_micros(100).await;
            }
        }
    }
}

#[task]
pub async fn usb_audio(spawner: SendSpawner, hardware: Hardware) {
    spawner.spawn(feedback(hardware.feedback_endpoint).unwrap());

    let mut endpoint = hardware.audio_endpoint;

    let receiver = channel::receiver();

    let set_enabled = |is_enabled| audio::sender().send(audio::Event::SetEnabled(is_enabled));

    loop {
        endpoint.wait_enabled().await;
        set_enabled(true).await;

        let packets = receive_all(&receiver).await;

        for packet in packets {
            let slice: &AudioPacket = &packet;

            // We don't care if there's an error, since we'll write the next sample soon enough anyway
            match endpoint
                .write_as_chunks(bytemuck::cast_slice(slice), true)
                .await
            {
                Ok(_) => {}
                Err(error) => match error {
                    EndpointError::BufferOverflow => {
                        println!("Warning: USB audio buffer overflow!");
                    }
                    EndpointError::Disabled => {
                        set_enabled(false).await;
                    }
                },
            }
        }
    }
}
