#![no_std]

use crate::{
    asdr::Envelope,
    filters::vcf::Vcf,
    note::{Event, Note},
    oscillator::WavetableOscillator,
    voice::Voice,
    wavetable::Wavetable,
};

mod arena;
pub mod asdr;
mod filters;
pub mod keyboard;
mod math;
pub mod note;
pub mod oscillator;
mod voice;
pub mod wavetable;

pub struct Synth<'a, const N: usize, const S: usize> {
    wavetables: [&'a Wavetable<S>; 2],

    voices: [Option<Voice<'a, S>>; N],

    vcf: Vcf,
    sample_rate: f32,

    cutoff: f32,
    resonance: f32,

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
            cutoff: 0.1,
            resonance: 0.2,
            envelope: Envelope {
                attack: 2.0,
                decay: 0.8,
                sustain: 0.9,
                release: 0.2,
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
            (sum / (count as f32)).clamp(0.0, 1.0)
        };

        self.vcf
            .sample(output, self.cutoff, self.resonance)
            .clamp(0.0, 1.0)
        // output
    }
}
