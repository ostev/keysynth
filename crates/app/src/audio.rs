use embassy_time::Instant;

use keyboard_protocol::StandardKey;
use synth::{Parameters, Sample, Synth, note::Note, wavetable::Wavetable};
use usbd_audio::AudioClass;

use crate::{
    gui,
    input::{
        self,
        encoder::{self, STEP},
        event::KeyEvent,
    },
    usb::{self},
};

/// The maximum number of notes that can be played simultaneously.
pub const MAX_POLYPHONY: usize = 4;

/// The number of samples in the synthesizer wavetable.

pub const WAVETABLE_SIZE: usize = 2048;

/// The lowest note produced by the keyboard layout.
const BASE_NOTE: Note = Note::C3;

/// An event that modifies the synthesizer state.
pub enum Event {
    /// Starts or stops a note.
    Note { note: Note, is_pressed: bool },
    /// Updates a parameter controlled by the secondary or tertiary encoders.
    PanelEncoderUpdate {
        direction: encoder::Direction,
        panel: encoder::Panel,
        id: encoder::PanelId,
    },
    /// Updates the voice gain, controlled by the primary encoder.
    VoiceGainEncoderUpdate { direction: encoder::Direction },
    /// Sets the voice gain directly.
    SetVoiceGain { gain: f32 },
}

impl Event {
    /// Converts an input event into a synthesizer event, returning `None` if the input event does
    /// not affect the synthesizer.
    pub fn from_event(panel: encoder::Panel, event: input::event::Event) -> Option<Event> {
        match event {
            input::event::Event::Key { event, .. } => Event::from_key_event(event),
            input::event::Event::Encoder { id, direction } => match id {
                encoder::Id::Primary => Some(Event::VoiceGainEncoderUpdate { direction }),
                encoder::Id::Panel(panel_id) => Some(Event::PanelEncoderUpdate {
                    direction,
                    panel,
                    id: panel_id,
                }),
            },
        }
    }

    /// Converts an input event into a synthesizer event, returning `None` if the key is not mapped
    /// to a synth function
    fn from_key_event(event: KeyEvent) -> Option<Event> {
        match event {
            KeyEvent::Pressed(keyboard_protocol::Key::Standard(key)) => {
                let note = note_from_key(key)?;
                Some(Event::Note {
                    note,
                    is_pressed: true,
                })
            }

            KeyEvent::Released(keyboard_protocol::Key::Standard(key)) => {
                let note = note_from_key(key)?;
                Some(Event::Note {
                    note,
                    is_pressed: false,
                })
            }

            _ => None,
        }
    }
}

/// Converts a key into its corresponding position
///
/// Returns [`None`] for keys that are not part of the musical keyboard layout.
fn synth_key_from_keyboard_key(
    key: keyboard_protocol::StandardKey,
) -> Option<synth::keyboard::Key> {
    let (row, column) = match key {
        // 1234 row -> columns 1..10
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

        // QWERTY row -> columns 1..10
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

        // ASDF row -> columns 1..10
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

        // ZXCVB row -> columns 1..10
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

/// Converts a keyboard key into the note it represents, returning [`None`]
/// if the key is not mapped to a note.
fn note_from_key(key: keyboard_protocol::StandardKey) -> Option<Note> {
    let synth_key = synth_key_from_keyboard_key(key)?;

    synth::keyboard::note_of(synth_key, BASE_NOTE)
}

/// Represents the state of the audio engine
pub struct State {
    synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE>,
}

impl State {
    /// Initialise the synthesiser with default parameters.
    pub fn new() -> State {
        let default_wavetable = Wavetable::from_fn(libm::sinf);

        let synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE> =
            Synth::new(SAMPLE_RATE as f32, default_wavetable, Parameters::default());

        State { synth }
    }

    pub fn set_wavetable(&mut self, wavetable: Wavetable<WAVETABLE_SIZE>) {
        self.synth.wavetable = wavetable;
    }

    pub fn sample(&mut self) -> AudioPacket {
        self.synth.sample_many()
    }

    /// Convenience method to apply multiple events to the audio engine.
    pub fn apply_events(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.apply_event(event);
        }
    }

    /// Applies a single event to the audio engine.
    pub fn apply_event(&mut self, event: Event) {
        match event {
            Event::Note { note, is_pressed } => {
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
            Event::PanelEncoderUpdate {
                direction,
                panel,
                id,
            } => {
                match panel {
                    encoder::Panel::CutoffResonance => match id {
                        encoder::PanelId::One => {
                            self.synth.parameters.cutoff = (self.synth.parameters.cutoff
                                + STEP * f32::from(direction))
                            .clamp(0.0, 1.0);
                        }
                        encoder::PanelId::Two => {
                            self.synth.parameters.resonance = (self.synth.parameters.resonance
                                + STEP * f32::from(direction))
                            .clamp(0.0, 1.0);
                        }
                    },
                    encoder::Panel::AttackDecay => match id {
                        encoder::PanelId::One => {
                            self.synth.parameters.envelope.attack =
                                (self.synth.parameters.envelope.attack
                                    + STEP * f32::from(direction))
                                .clamp(0.0, 1.0);
                        }
                        encoder::PanelId::Two => {
                            self.synth.parameters.envelope.decay =
                                (self.synth.parameters.envelope.decay
                                    + STEP * f32::from(direction))
                                .clamp(0.0, 1.0);
                        }
                    },
                    encoder::Panel::SustainRelease => match id {
                        encoder::PanelId::One => {
                            self.synth.parameters.envelope.sustain =
                                (self.synth.parameters.envelope.sustain
                                    + STEP * f32::from(direction))
                                .clamp(0.0, 1.0);
                        }
                        encoder::PanelId::Two => {
                            self.synth.parameters.envelope.release =
                                (self.synth.parameters.envelope.release
                                    + STEP * f32::from(direction))
                                // Unlike the others, release can go past 1.0
                                .clamp(0.0, synth::MAX_RELEASE);
                        }
                    },
                };

                let _ = gui::set_synth_parameters(self.synth.parameters);
            }
            Event::VoiceGainEncoderUpdate { direction } => {
                pub const GAIN_STEP: f32 = 0.02;

                self.synth.parameters.voice_gain = (self.synth.parameters.voice_gain
                    + GAIN_STEP * f32::from(direction))
                .clamp(0.0, 1.0);

                let _ = gui::set_synth_parameters(self.synth.parameters);
            }
            Event::SetVoiceGain { gain } => {
                self.synth.parameters.voice_gain = gain.clamp(0.0, 1.0);

                let _ = gui::set_synth_parameters(self.synth.parameters);
            }
        }
    }
}

type AudioPacket = [Sample; 48];

/// The interval in milliseconds between USB audio polls.
pub const AUDIO_REFRESH_MS: u8 = 4;
/// The sample rate in Hz.
pub const SAMPLE_RATE: u32 = 48_000;

pub struct Hardware {
    pub device: AudioClass<'static, usb::Bus>,
}
