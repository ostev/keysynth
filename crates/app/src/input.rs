use embassy_executor::task;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use esp_println::println;
use keyboard_protocol::{Key, KeyboardStatus, KeyboardWithEncoderStatus, SpecialKey, StandardKey};

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

    pub fn apply<const N: usize>(&mut self, change: InputChange) -> heapless::Vec<Event, N> {
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
                    .collect()
            }
            InputChange::Encoder(encoder_status) => encoder_status
                .deltas
                .into_iter()
                .enumerate()
                .filter_map(|(index, delta)| {
                    if delta > 0 {
                        Some((index, encoder::Direction::Increase))
                    } else if delta < 0 {
                        Some((index, encoder::Direction::Decrease))
                    } else {
                        None
                    }
                })
                .map(|(index, direction)| {
                    let id = match index {
                        0 => encoder::Id::Primary,
                        1 => encoder::Id::Panel(encoder::PanelId::One),
                        2 => encoder::Id::Panel(encoder::PanelId::Two),
                        _ => panic!("Only three encoders are supported!"),
                    };

                    Event::Encoder { id, direction }
                })
                .collect(),
        }
    }
}

fn is_toggle_mode(event: &Event) -> bool {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Special(SpecialKey::One)) => true,
            _ => false,
        },
        _ => false,
    }
}

#[task]
pub async fn router() {
    let mut mode = KeyboardMode::Passthrough;

    let keyboard = keyboard::receiver();
    let encoder = encoder::receiver();

    let gui = gui::input_sender();
    let audio = usb::audio_sender();

    let mut state = State::new();

    loop {
        let notification = select(keyboard.receive(), encoder.receive()).await;

        let changes: heapless::Vec<InputChange, 3> = match notification {
            Either::First(KeyboardWithEncoderStatus {
                keyboard: keyboard_status,
                encoder: encoder_update,
            }) => {
                if encoder_update.is_zero() {
                    heapless::Vec::from_array([InputChange::Keyboard(keyboard_status)])
                } else {
                    heapless::Vec::from_array([
                        InputChange::Keyboard(keyboard_status),
                        InputChange::Encoder(EncoderStatus {
                            deltas: [0, encoder_update.deltas[0], encoder_update.deltas[1]],
                        }),
                    ])
                }
            }
            Either::Second(delta) => {
                heapless::Vec::from_array([InputChange::Encoder(EncoderStatus {
                    deltas: [delta, 0, 0],
                })])
            }
        };

        const MAX_EVENTS_PER_CHANGE: usize = 30;

        let events = changes
            .into_iter()
            .map(|change| state.apply::<MAX_EVENTS_PER_CHANGE>(change).into_iter())
            .flatten();

        match mode {
            KeyboardMode::Passthrough => match notification {
                Either::First(KeyboardWithEncoderStatus {
                    keyboard: keyboard_status,
                    encoder: _encoder_update,
                }) => {
                    unsafe {
                        usb::set_keyboard_status(UsbKeyboardStatus {
                            keys: keyboard_status.keys,
                            modifier_bitfield: keyboard_status.modifier_bitfield,
                        });
                    }

                    for event in events {
                        if is_toggle_mode(&event) {
                            mode = mode.toggle();
                        }

                        // match event {
                        //     Event::Encoder { id, direction } => {
                        //         // Handle volume (first encoder)
                        //         {
                        //             const VOLUME_STEP: i32 = 40;

                        //             let key = match direction {
                        //                 encoder::Direction::Increase => StandardKey::VolumeDown,
                        //                 encoder::Direction::Decrease => StandardKey::VolumeUp,
                        //             };

                        //             hid.send(UsbKeyboardStatus::press(key)).await;
                        //         }

                        //         // Handle scrolling up/down (fourth encoder)
                        //         {
                        //             const SCROLL_STEP: i32 = 1;

                        //             let steps = delta[0] / SCROLL_STEP;

                        //             let key = if steps < 0 {
                        //                 Some(StandardKey::Up)
                        //             } else if steps > 0 {
                        //                 Some(StandardKey::Down)
                        //             } else {
                        //                 None
                        //             };

                        //             if let Some(key) = key {
                        //                 for _ in 0..(steps as u32) {
                        //                     hid.send(UsbKeyboardStatus::press(key)).await;
                        //                 }
                        //             }
                        //         }

                        //         // Handle scrubbing left/right (third encoder)
                        //         {
                        //             const SCRUB_STEP: i32 = 4;

                        //             let steps = delta[0] / SCRUB_STEP;

                        //             let key = if steps < 0 {
                        //                 Some(StandardKey::Left)
                        //             } else if steps > 0 {
                        //                 Some(StandardKey::Right)
                        //             } else {
                        //                 None
                        //             };

                        //             if let Some(key) = key {
                        //                 for _ in 0..(steps as u32) {
                        //                     hid.send(UsbKeyboardStatus::press(key)).await;
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    }
                }
                Either::Second(_) => {
                    for event in events {
                        if is_toggle_mode(&event) {
                            mode = mode.toggle();
                        }
                    }
                }
            },
            KeyboardMode::Capture => {
                for event in events {
                    if is_toggle_mode(&event) {
                        mode = mode.toggle();
                    } else if gui::is_capturing(&event) {
                        gui.send(event).await;
                    } else if let Some(audio_event) =
                        audio::Event::from_event(gui::current_panel(), event)
                    {
                        match &audio_event {
                            audio::Event::Note { note, is_pressed } => {
                                if *is_pressed {
                                    gui::set_new_note(*note);
                                }
                            }
                            _ => {}
                        }
                        audio.send(audio_event).await;
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
