#![cfg_attr(not(test), no_std)]

use crate::{
    adsr::Envelope,
    filters::vcf::Vcf,
    note::{Event, Note},
    oscillator::WavetableOscillator,
    voice::Voice,
    wavetable::Wavetable,
};

pub mod adsr;
mod arena;
mod filters;
pub mod keyboard;
mod math;
pub mod note;
pub mod oscillator;
mod voice;
pub mod wavetable;

pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

pub struct Synth<'a, const N: usize, const S: usize> {
    wavetables: [&'a Wavetable<S>; 2],

    voices: [Option<Voice<'a, S>>; N],

    vcf: Vcf,
    sample_rate: f32,

    cutoff: f32,
    resonance: f32,

    voice_gain: f32,

    envelope: Envelope,
}

#[derive(Clone, Copy, Debug)]
pub enum PlayError {
    NoFreeVoice,
    NoMatchingVoice,
}

impl<'a, const N: usize, const S: usize> Synth<'a, N, S> {
    pub const fn new(sample_rate: f32, wavetables: [&'a Wavetable<S>; 2]) -> Self {
        Self {
            wavetables,
            voices: [const { None }; N],
            vcf: Vcf::new(),
            sample_rate,
            cutoff: 0.4,
            resonance: 0.2,
            voice_gain: 1.0 / (N as f32),
            envelope: Envelope {
                attack: 1.0,
                decay: 0.9,
                sustain: 0.1,
                release: 0.9,
            },
        }
    }

    pub fn note_on(&mut self, note: Event) -> Result<(), PlayError> {
        let free_voice = self.voices.iter_mut().find(|voice| voice.is_none());

        match free_voice {
            Some(voice) => {
                *voice = Some(Voice::new(
                    self.sample_rate,
                    WavetableOscillator::new(self.wavetables[0], self.sample_rate),
                    WavetableOscillator::new(self.wavetables[1], self.sample_rate),
                    note,
                    self.envelope,
                ));

                Ok(())
            }
            None => Err(PlayError::NoFreeVoice),
        }
    }

    pub fn note_off(&mut self, note: Note) -> Result<(), PlayError> {
        let voice = self
            .voices
            .iter_mut()
            .find_map(|voice| {
                voice.as_mut().and_then(|voice| {
                    if voice.note().note == note {
                        Some(voice)
                    } else {
                        None
                    }
                })
            })
            .ok_or(PlayError::NoMatchingVoice)?;

        voice.release();

        Ok(())
    }

    pub fn sample(&mut self) -> f32 {
        let (sum, count) = self
            .voices
            .iter_mut()
            .filter_map(|option_voice| match option_voice {
                Some(voice) => {
                    if voice.is_ended() {
                        *option_voice = None;
                        None
                    } else {
                        Some(voice.sample(0.0))
                    }
                }
                None => None,
            })
            .fold((0.0, 0usize), |(sum, count), sample| {
                (sum + sample, count + 1)
            });

        let output = if count == 0 {
            0.0
        } else {
            (sum * self.voice_gain).clamp(0.0, 1.0)
        };

        self.vcf
            .sample(output, self.cutoff, self.resonance)
            .clamp(0.0, 1.0)
    }

    pub fn sample_into(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.sample();
        }
    }

    pub fn sample_many<const BUFFER_SIZE: usize>(&mut self) -> [f32; BUFFER_SIZE] {
        let mut buffer = [0.0; BUFFER_SIZE];
        self.sample_into(&mut buffer);
        buffer
    }
}
