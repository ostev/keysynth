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

pub struct Synth<const N: usize, const S: usize> {
    pub wavetable: Wavetable<S>,

    voices: [Option<Voice<S>>; N],

    vcf: Vcf,
    sample_rate: f32,

    pub parameters: Parameters,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub cutoff: f32,
    pub resonance: f32,

    pub voice_gain: f32,

    pub envelope: Envelope,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            cutoff: 0.8,
            resonance: 0.1,
            voice_gain: 0.25,
            envelope: Envelope {
                attack: 0.3,
                decay: 0.3,
                sustain: 1.0,
                release: 0.9,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PlayError {
    NoFreeVoice,
    NoMatchingVoice,
}

pub type Sample = i16;

impl<const N: usize, const S: usize> Synth<N, S> {
    pub const fn new(sample_rate: f32, wavetable: Wavetable<S>, parameters: Parameters) -> Self {
        Self {
            wavetable,
            voices: [const { None }; N],
            vcf: Vcf::new(),
            sample_rate,
            parameters,
        }
    }

    pub fn note_on(&mut self, note: Event) -> Result<(), PlayError> {
        let (matching_voice, ended_voice, inactive_voice) = self.voices.iter_mut().fold(
            (None, None, None),
            |(matching_voice, ended_voice, inactive_voice), voice| match voice {
                Some(unwrapped_voice) => {
                    if unwrapped_voice.note().note == note.note {
                        (Some(voice), ended_voice, inactive_voice)
                    } else if !unwrapped_voice.is_active() {
                        (matching_voice, ended_voice, Some(voice))
                    } else {
                        (matching_voice, ended_voice, inactive_voice)
                    }
                }
                None => (matching_voice, Some(voice), inactive_voice),
            },
        );
        let free_voice = matching_voice.or(ended_voice.or(inactive_voice));

        match free_voice {
            Some(voice) => {
                *voice = Some(Voice::new(
                    self.sample_rate,
                    WavetableOscillator::new(self.sample_rate),
                    note,
                    self.parameters.voice_gain,
                    self.parameters.envelope,
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

    // pub fn sample(&mut self) -> Sample {
    //     let voices: heapless::Vec<&mut Voice<S>, N> = self
    //         .voices
    //         .iter_mut()
    //         .filter_map(|option_voice| match option_voice {
    //             Some(voice) => {
    //                 if voice.is_ended() {
    //                     *option_voice = None;
    //                     None
    //                 } else {
    //                     // Some(voice.sample(&self.wavetables, 0.0))
    //                     Some(voice)
    //                 }
    //             }
    //             None => None,
    //         })
    //         .collect();
    // .fold((0.0, 0usize), |(sum, count), sample| {
    //     (sum + sample, count + 1)
    // });

    // let output = if count == 0 {
    //     0.0
    // } else {
    //     (sum * self.voice_gain).clamp(-1.0, 1.0)
    // };

    // let sample = self
    //     .vcf
    //     .sample(output, self.cutoff, self.resonance)
    //     .clamp(-1.0, 1.0);
    // let integer_sample = (sample * i16::MAX as f32) as i16;

    // [integer_sample, integer_sample]
    // }

    pub fn sample_into(&mut self, buffer: &mut [Sample]) {
        let mut voices: heapless::Vec<&mut Voice<S>, N> = self
            .voices
            .iter_mut()
            .filter_map(|option_voice| {
                if option_voice.as_ref().is_some_and(|voice| voice.is_ended()) {
                    *option_voice = None;
                    None
                } else if let Some(voice) = option_voice.as_mut() {
                    Some(voice)
                } else {
                    None
                }
            })
            .collect();

        for sample in buffer.iter_mut() {
            *sample = Synth::<N, S>::sample_voices(
                &mut self.vcf,
                &self.wavetable,
                self.parameters.cutoff,
                self.parameters.resonance,
                voices.iter_mut().map(|voice| {
                    let dereferenced_voice: &mut Voice<S> = *voice;
                    dereferenced_voice
                }),
            );
        }
    }

    fn sample_voices<'a>(
        vcf: &mut Vcf,
        wavetable: &Wavetable<S>,
        cutoff: f32,
        resonance: f32,
        voices: impl IntoIterator<Item = &'a mut Voice<S>>,
    ) -> Sample {
        let voice_output = voices
            .into_iter()
            .map(|voice| voice.sample(wavetable))
            .sum();

        let with_vcf = vcf.sample(voice_output, cutoff, resonance).clamp(-1.0, 1.0);

        (with_vcf * i16::MAX as f32) as i16
    }

    pub fn sample_many<const BUFFER_SIZE: usize>(&mut self) -> [Sample; BUFFER_SIZE] {
        let mut buffer = [0; BUFFER_SIZE];
        self.sample_into(&mut buffer);
        buffer
    }

    pub fn is_active(&mut self) -> bool {
        self.voices
            .iter()
            .filter_map(Option::as_ref)
            .any(|voice| voice.is_ended())
    }
}
