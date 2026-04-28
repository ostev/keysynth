use core::f32;

use crate::note::Note;

#[derive(Clone, Debug)]
pub struct Wavetable<const S: usize> {
    pub samples: [f32; S],
}

pub const DEFAULT_SIZE: usize = 2048;

pub type DefaultWavetable = Wavetable<DEFAULT_SIZE>;

impl<const S: usize> Wavetable<S> {
    pub fn from_fn(mut generator: impl FnMut(f32) -> f32) -> Self {
        let phase_per_index = 2.0 * f32::consts::PI / S as f32;
        Self {
            samples: core::array::from_fn(|index| generator(phase_per_index * index as f32)),
        }
    }

    pub fn from_samples(samples: [f32; S]) -> Self {
        Self { samples }
    }
}
