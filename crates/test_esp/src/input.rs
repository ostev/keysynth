use embassy_executor::task;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use esp_println::println;
use keyboard_protocol::{Key, KeyboardStatus, SpecialKey, StandardKey};

use crate::{
    audio, gui,
    input::{
        encoder::EncoderStatus,
        event::{Event, InputChange, KeyEvent},
    },
    usb::{self, hid::UsbKeyboardStatus},
};

pub mod encoder;
pub mod event;
pub mod keyboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardMode {
    Passthrough,
    Capture,
}

impl KeyboardMode {
    pub fn toggle(self) -> KeyboardMode {
        match self {
            KeyboardMode::Capture => KeyboardMode::Passthrough,
            KeyboardMode::Passthrough => KeyboardMode::Capture,
        }
    }
}

struct State {
    keyboard: KeyboardStatus,
}

impl State {
    pub fn new() -> State {
        State {
            keyboard: KeyboardStatus::empty(),
        }
    }

    pub fn apply(&mut self, change: InputChange) -> impl Iterator<Item = Event> {
        match change {
            InputChange::Keyboard(keyboard_status) => {
                let diff = keyboard_status.diff(self.keyboard);
                self.keyboard = keyboard_status;

                let pressed = diff.pressed.map(KeyEvent::Pressed);
                let released = diff.released.map(KeyEvent::Released);

                let keyboard = keyboard_status;

                pressed
                    .chain(released)
                    .map(move |event| Event::Key { event, keyboard })
            }
        }
    }
}

fn is_toggle_mode(event: &Event) -> bool {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Special(SpecialKey::One)) => true,
            _ => false,
        },
    }
}

#[task]
pub async fn router() {
    let mut mode = KeyboardMode::Passthrough;

    let keyboard = keyboard::receiver();
    let encoder = encoder::receiver();

    let gui = gui::sender();
    let audio = usb::audio_sender();

    let mut state = State::new();

    loop {
        let notification = select(keyboard.receive(), encoder.receive()).await;

        let change = match notification {
            Either::First(keyboard_status) => InputChange::Keyboard(keyboard_status),
            Either::Second(encoder_status) => todo!(),
        };

        // let events = {
        //     let mut events = heapless::Vec::<Event, 30>::new();
        //     for event in state.apply(change) {
        //         match events.push(event) {
        //             Ok(_) => {}
        //             Err(_) => break,
        //         }
        //     }
        //     events
        // };
        let events = state.apply(change);
        // println!("Loop!");

        match mode {
            KeyboardMode::Passthrough => match notification {
                Either::First(keyboard_status) => {
                    unsafe {
                        usb::set_keyboard_status(UsbKeyboardStatus {
                            keys: keyboard_status.keys,
                            modifier_bitfield: keyboard_status.modifier_bitfield,
                        });
                    }

                    for event in events {
                        if is_toggle_mode(&event) {
                            println!("Toggle!");
                            mode = mode.toggle();
                        }
                    }
                }
                Either::Second(_) => todo!(),
            },
            KeyboardMode::Capture => {
                for event in events {
                    if is_toggle_mode(&event) {
                        println!("Toggle!");
                        mode = mode.toggle();
                    } else if gui::is_capturing(&event) {
                        gui.send(event).await;
                    } else {
                        audio.send(audio::Event::Input(event)).await;
                    }
                }
            }
        }

        // match input_event {
        //     Either::First(keyboard_status) => {
        //         match mode {
        //             KeyboardMode::Passthrough => {
        //                 hid.send(UsbKeyboardStatus {
        //                     keys: keyboard_status.keys,
        //                     modifier_bitfield: keyboard_status.modifier_bitfield,
        //                 })
        //                 .await;
        //             }
        //             KeyboardMode::Capture => {
        //                 // if  gui::is_capturing_all_keyboard_input() {
        //                 // gui.send(input::event::InputChange::Keyboard(keyboard_status))
        //                 //     .await
        //             }
        //         }
        //     }
        //     Either::Second(EncoderStatus { delta }) => match mode {
        //         KeyboardMode::Passthrough => {
        //             // Handle volume (first encoder)
        //             {
        //                 const VOLUME_STEP: i32 = 40;

        //                 let steps = delta[0] / VOLUME_STEP;

        //                 let key = if steps < 0 {
        //                     Some(StandardKey::VolumeDown)
        //                 } else if steps > 0 {
        //                     Some(StandardKey::VolumeUp)
        //                 } else {
        //                     None
        //                 };

        //                 if let Some(key) = key {
        //                     for _ in 0..(steps as u32) {
        //                         hid.send(UsbKeyboardStatus::press(key)).await;
        //                     }
        //                 }
        //             }

        //             // Handle scrolling up/down (fourth encoder)
        //             {
        //                 const SCROLL_STEP: i32 = 1;

        //                 let steps = delta[0] / SCROLL_STEP;

        //                 let key = if steps < 0 {
        //                     Some(StandardKey::Up)
        //                 } else if steps > 0 {
        //                     Some(StandardKey::Down)
        //                 } else {
        //                     None
        //                 };

        //                 if let Some(key) = key {
        //                     for _ in 0..(steps as u32) {
        //                         hid.send(UsbKeyboardStatus::press(key)).await;
        //                     }
        //                 }
        //             }

        //             // Handle scrubbing left/right (third encoder)
        //             {
        //                 const SCRUB_STEP: i32 = 4;

        //                 let steps = delta[0] / SCRUB_STEP;

        //                 let key = if steps < 0 {
        //                     Some(StandardKey::Left)
        //                 } else if steps > 0 {
        //                     Some(StandardKey::Right)
        //                 } else {
        //                     None
        //                 };

        //                 if let Some(key) = key {
        //                     for _ in 0..(steps as u32) {
        //                         hid.send(UsbKeyboardStatus::press(key)).await;
        //                     }
        //                 }
        //             }
        //         }
        //         KeyboardMode::Capture => todo!(),
        //     },
        // }
    }
}
