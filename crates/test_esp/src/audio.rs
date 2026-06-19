use core::{future, mem::MaybeUninit};

use alloc::sync::Arc;
use embassy_executor::task;
use embassy_time::{Duration, Instant, Ticker, Timer};
// use embassy_usb::{
//     class::uac1::{self, source::AudioSourceEpIn},
//     driver::EndpointError,
// };
use esp_hal::{interrupt::software::SoftwareInterrupt, peripherals::CPU_CTRL};
use esp_println::println;
use esp_rtos::embassy::Executor;
use keyboard_protocol::StandardKey;
use static_cell::StaticCell;
use synth::{
    Sample, Synth,
    note::{self, Note},
    wavetable::Wavetable,
};
use usb_device::class::UsbClass;
use usbd_audio::AudioClass;

use crate::{
    concurrency::try_receive_all,
    input::{self, event::KeyEvent},
    usb::{self},
};

pub const MAX_POLYPHONY: usize = 4;
pub const WAVETABLE_SIZE: usize = 2048;

const BASE_NOTE: Note = Note::C3;

pub enum Event {
    Input(input::event::Event),
}

fn synth_key_from_keyboard_key(
    key: keyboard_protocol::StandardKey,
) -> Option<synth::keyboard::Key> {
    let (row, column) = match key {
        // Top row: numbers 1..0 -> columns 1..10
        StandardKey::Key1 => (0, 0),
        StandardKey::Key2 => (0, 1),
        StandardKey::Key3 => (0, 2),
        StandardKey::Key4 => (0, 3),
        StandardKey::Key5 => (0, 4),
        StandardKey::Key6 => (0, 5),
        StandardKey::Key7 => (0, 6),
        StandardKey::Key8 => (0, 7),
        StandardKey::Key9 => (0, 8),
        StandardKey::Key0 => (0, 9),

        // TY row -> columns 1..10
        StandardKey::Q => (1, 0),
        StandardKey::W => (1, 1),
        StandardKey::E => (1, 2),
        StandardKey::R => (1, 3),
        StandardKey::T => (1, 4),
        StandardKey::Y => (1, 5),
        StandardKey::U => (1, 6),
        StandardKey::I => (1, 7),
        StandardKey::O => (1, 8),
        StandardKey::P => (1, 9),

        //  row -> columns 1..10
        StandardKey::A => (2, 0),
        StandardKey::S => (2, 1),
        StandardKey::D => (2, 2),
        StandardKey::F => (2, 3),
        StandardKey::G => (2, 4),
        StandardKey::H => (2, 5),
        StandardKey::J => (2, 6),
        StandardKey::K => (2, 7),
        StandardKey::L => (2, 8),
        StandardKey::Semicolon => (2, 9),

        //  row -> columns 1..10
        StandardKey::Z => (3, 0),
        StandardKey::X => (3, 1),
        StandardKey::C => (3, 2),
        StandardKey::V => (3, 3),
        StandardKey::B => (3, 4),
        StandardKey::N => (3, 5),
        StandardKey::M => (3, 6),
        StandardKey::Comma => (3, 7),
        StandardKey::Dot => (3, 8),
        StandardKey::Slash => (3, 9),

        // Ignore any other keys
        _ => return None,
    };

    Some(synth::keyboard::Key::new(row, column))
}

fn note_from_key(
    keyboard: &synth::keyboard::Keyboard,
    key: keyboard_protocol::StandardKey,
) -> Option<Note> {
    let synth_key = synth_key_from_keyboard_key(key)?;
    keyboard.note_of(synth_key, BASE_NOTE)
}

fn note_from_input_event(
    keyboard: &synth::keyboard::Keyboard,
    event: input::event::Event,
) -> Option<(Note, bool)> {
    match event {
        input::event::Event::Key {
            event: key_event, ..
        } => match key_event {
            KeyEvent::Pressed(keyboard_protocol::Key::Standard(key)) => {
                let note = note_from_key(keyboard, key)?;
                Some((note, true))
            }

            KeyEvent::Released(keyboard_protocol::Key::Standard(key)) => {
                let note = note_from_key(keyboard, key)?;
                Some((note, false))
            }

            _ => None,
        },
    }
}

/// Asynchronously writes a USB audio packet to the provided audio endpoint, waiting if it isn't enabled.
// async fn write_audio_packet(
//     endpoint: &mut uac1::source::AudioSourceEpIn<'static, usb::Bus>,
//     packet: &AudioPacket,
// ) {
//     // println!("synth Waiting for enable");
//     endpoint.wait_enabled().await;

//     loop {
//         // We don't particularly care if there's an error, since we'll write the next sample soon enough anyway.
//         // In debug mode, we'll log it.
//         match endpoint
//             .write_as_chunks(bytemuck::cast_slice(packet), true)
//             .await
//         {
//             Ok(_) => {
//                 break;
//                 // println!("Synth AA")
//             }
//             Err(error) => {
//                 match error {
//                     EndpointError::BufferOverflow => {
//                         println!("Warning: USB audio buffer overflow!");
//                     }
//                     EndpointError::Disabled => {
//                         println!("Warning: USB audio buffer disabled!")
//                     }
//                 }
//                 Timer::after_micros(100).await;
//             }
//         }
//     }
// }

pub struct State {
    synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE>,
    keyboard: synth::keyboard::Keyboard,
}
impl State {
    pub fn new() -> State {
        let default_wavetable = Wavetable::from_fn(libm::sinf);

        let synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE> = Synth::new(
            SAMPLE_RATE as f32,
            [default_wavetable.clone(), default_wavetable],
        );

        // synth
        //     .note_on(note::Event {
        //         note: Note::C3,
        //         timestamp: Instant::now().as_micros(),
        //     })
        //     .unwrap();

        let keyboard = synth::keyboard::Keyboard::new((0, 0), (10, 4)).unwrap();

        State { synth, keyboard }
    }

    pub fn sample(&mut self) -> AudioPacket {
        self.synth.sample_many()
    }

    pub fn apply_events(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.apply_event(event);
        }
    }

    pub fn apply_event(&mut self, event: Event) {
        match event {
            Event::Input(input) => {
                if let Some((note, is_pressed)) = note_from_input_event(&self.keyboard, input) {
                    // We discard the error as we don't really care if there aren't any free voices.
                    // The note just won't play.
                    let _ = if is_pressed {
                        let note_event = synth::note::Event {
                            note,
                            timestamp: Instant::now().as_micros(),
                        };

                        self.synth.note_on(note_event)
                    } else {
                        self.synth.note_off(note)
                    };
                }
            }
        }
    }
}

// fn synth(hardware: Hardware) {
//     let keyboard = synth::keyboard::Keyboard::new((0, 0), (10, 4)).unwrap();

//     let default_wavetable = Wavetable::from_fn(libm::sinf);

//     let mut synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE> = Synth::new(
//         SAMPLE_RATE as f32,
//         [default_wavetable.clone(), default_wavetable],
//     );

//     let receiver = channel::receiver();

//     let mut ticker = Ticker::every(Duration::from_millis(AUDIO_REFRESH_MS.into()));

//     let mut count = 0;
//     let mut total_time = 0;

//     let mut synth_count = 0;
//     let mut synth_time = 0;

//     loop {
//         let start = Instant::now();
//         // let events = try_receive_all(&receiver);
//         let event = receiver.try_receive();

//         // for event in events {

//         // }

//         let start_synth = Instant::now();
//         let samples: AudioPacket = synth.sample_many();
//         synth_time += start_synth.elapsed().as_micros();
//         synth_count += 1;

//         // write_audio_packet(&mut endpoint, &samples).await;

//         let time = start.elapsed();
//         // println!("Time taken: {}", time.as_micros());
//         total_time += time.as_micros();
//         count += 1;

//         if count >= 300 {
//             println!("Average time: {} us", total_time / count);
//             count = 0;
//             total_time = 0;
//         }

//         if synth_count >= 300 {
//             println!("Average synth time: {} us", synth_time / synth_count);
//             synth_count = 0;
//             synth_time = 0;
//         }

//         // out.send(samples).await;

//         // ticker.next().await;
//     }
// }

// type AudioPacket = [Sample; ((AUDIO_REFRESH_MS as f32 / 1000.0) * SAMPLE_RATE as f32) as usize];
type AudioPacket = [Sample; 48];

pub const AUDIO_REFRESH_MS: u8 = 4;
pub const SAMPLE_RATE: u32 = 48_000;

pub struct Hardware {
    pub device: AudioClass<'static, usb::Bus>, // pub audio_endpoint: uac1::source::AudioSourceEpIn<'static, usb::Bus>,
                                               // pub feedback_endpoint: uac1::source::AudioSourceEpIn<'static, usb::Bus>,
}

// pub fn start(
//     cpu_control: CPU_CTRL<'static>,
//     interrupt: SoftwareInterrupt<'static, 1>,
//     hardware: Hardware,
// ) {
//     static STACK: StaticCell<esp_hal::system::Stack<{ STACK_SIZE }>> = StaticCell::new();

//     let stack = STACK.init_with(|| esp_hal::system::Stack::new());

//     esp_rtos::start_second_core(cpu_control, interrupt, stack, move || {
// static EXECUTOR: StaticCell<Executor> = StaticCell::new();

// let executor = EXECUTOR.init_with(|| Executor::new());

// executor.run(|spawner| {
// spawner.spawn(feedback(hardware.feedback_endpoint).unwrap());
// spawner.spawn(synth(hardware.audio_endpoint).unwrap());
// spawner.spawn(say_hi().unwrap());
// })
//     });
// }

// #[task]
// async fn say_hi() -> ! {
//     let mut ticker = Ticker::every(Duration::from_millis(500));

//     loop {
//         println!("Hi");
//         ticker.next().await;
//     }
// }

// #[task]
// async fn feedback(mut feedback_endpoint: AudioSourceEpIn<'static, usb::Bus>) {
//     // Convert the sample rate to USB's 3-byte 10.14 sample rate format by multiply it by
//     // 2^14 and then taking the left three bytes in little-endian order.
//     let feedback_buffer = &(SAMPLE_RATE << 14).to_le_bytes()[..3];

//     let mut ticker = Ticker::every(Duration::from_millis(AUDIO_REFRESH_MS.into()));

//     loop {
//         // println!("Waiting for enable");
//         feedback_endpoint.wait_enabled().await;

//         match feedback_endpoint.write(feedback_buffer).await {
//             Ok(_) => {
//                 // println!("AAAA");
//                 // Wait until the next audio frame to send the updated sample rate
//                 ticker.next().await;
//             }
//             Err(error) => {
//                 println!(
//                     "Error: failed to write to audio feedback buffer ({:?})",
//                     error
//                 );
//                 // Short delay before retrying in case the error is transient
//                 embassy_time::Timer::after_micros(100).await;
//             }
//         }
//     }
// }
