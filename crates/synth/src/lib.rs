#![no_std]

use crate::{
    note::{Event, Note},
    voice::Voice,
    wavetable::{Oscillator, Wavetable},
};

mod arena;
pub mod keyboard;
mod math;
pub mod note;
mod voice;
pub mod wavetable;

pub struct Synth<'a, const N: usize, const S: usize> {
    wavetables: [&'a Wavetable<S>; 2],

    voices: [Option<Voice<'a, S>>; N],

    sample_rate: f32,
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
            voices: [None; N],
            sample_rate,
        }
    }

    pub fn note_on(&mut self, note: Event) -> Result<(), PlayError> {
        let free_voice = self.voices.iter_mut().find(|voice| voice.is_none());

        match free_voice {
            Some(voice) => {
                *voice = Some(Voice::new(
                    Oscillator::new(self.wavetables[0], self.sample_rate),
                    Oscillator::new(self.wavetables[1], self.sample_rate),
                    note,
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
            .find(|voice| match voice {
                Some(voice) => voice.note().note == note,
                None => false,
            })
            .ok_or(PlayError::NoMatchingVoice)?;

        *voice = None;

        Ok(())
    }

    pub fn sample(&mut self) -> f32 {
        let active_voices = self.voices.iter_mut().filter_map(Option::as_mut);

        let (sum, count) = active_voices
            .map(|voice| voice.sample(0.5))
            .fold((0.0, 0usize), |(sum, count), sample| {
                (sum + sample, count + 1)
            });

        sum / (count as f32)
    }
}
