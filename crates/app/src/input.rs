use embassy_executor::task;
use embassy_futures::select::{Either, select};
use keyboard_protocol::{Key, KeyboardStatus, KeyboardWithEncoderStatus, SpecialKey};

use crate::{
    gui,
    input::{
        encoder::EncoderStatus,
        event::{Event, InputChange, KeyEvent},
    },
    usb::audio,
    usb::{self, hid::UsbKeyboardStatus},
};

pub mod encoder;
pub mod event;
pub mod keyboard;

/// Determines how keyboard input is handled by the router.
///
/// - `Passthrough` forwards keyboard input directly to USB.
/// - `Capture` converts keyboard and encoder input into GUI and audio events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardMode {
    Passthrough,
    Capture,
}

impl KeyboardMode {
    /// Switches between passthrough and capture modes.
    pub fn toggle(self) -> KeyboardMode {
        match self {
            KeyboardMode::Capture => KeyboardMode::Passthrough,
            KeyboardMode::Passthrough => KeyboardMode::Capture,
        }
    }

    /// Toggles the keyboard mode if the right button is pressed.
    pub fn apply(self, event: &Event) -> KeyboardMode {
        match event {
            Event::Key { event, .. } => match event {
                KeyEvent::Pressed(Key::Special(SpecialKey::One)) => self.toggle(),
                _ => self,
            },
            _ => self,
        }
    }
}

/// Tracks the previous input state so incoming hardware updates can be
/// converted into discrete input events.
struct State {
    keyboard: KeyboardStatus,
}

impl State {
    /// Creates a new state with no keys pressed.
    pub fn new() -> State {
        State {
            keyboard: KeyboardStatus::empty(),
        }
    }
    /// Applies an input change and returns the discrete events that occured.
    pub fn apply<const N: usize>(&mut self, change: InputChange) -> heapless::Vec<Event, N> {
        match change {
            InputChange::Keyboard(keyboard_status) => {
                // Diff keyboard events
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
            // Convert encoder events into directional updates.
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

/// Routes keyboard and encoder input to the appropriate subsystem.
///
/// The router receives updates from the keyboard matrix and encoder
/// tasks, converts them into discrete events, and dispatches them
/// according to the current [`KeyboardMode`].
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

        // We cap the number of events per change to stop memory usage from increasing too much
        // and keep performance consistent.
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
                        mode = mode.apply(&event);
                    }
                }
                Either::Second(_) => {
                    for event in events {
                        mode = mode.apply(&event);
                    }
                }
            },
            KeyboardMode::Capture => {
                for event in events {
                    mode = mode.apply(&event);

                    if gui::is_capturing(&event) {
                        gui.send(event).await;
                    } else if let Some(audio_event) =
                        audio::Event::from_event(gui::current_panel(), event)
                    {
                        audio.send(audio_event).await;
                    }
                }
            }
        }
    }
}
