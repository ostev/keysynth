use core::f32;

/// Represents a collection of wavetable samples
#[derive(Clone, Debug)]
pub struct Wavetable<const S: usize> {
    pub samples: [f32; S],
}

pub const DEFAULT_SIZE: usize = 2048;

pub type DefaultWavetable = Wavetable<DEFAULT_SIZE>;

impl<const S: usize> Wavetable<S> {
    /// Create a wavetable by sampling the provided function
    pub fn from_fn(mut generator: impl FnMut(f32) -> f32) -> Self {
        let phase_per_index = 2.0 * f32::consts::PI / S as f32;
        Self {
            samples: core::array::from_fn(|index| generator(phase_per_index * index as f32)),
        }
    }

    /// Attempt to create a wavetable by sampling the provided fallible function, returning early
    /// if it errors
    pub fn try_from_fn<E>(mut generator: impl FnMut(f32) -> Result<f32, E>) -> Result<Self, E> {
        let phase_per_index = 2.0 * f32::consts::PI / S as f32;

        let mut samples = [0.0; S];

        for (index, entry) in samples.iter_mut().enumerate() {
            match generator(phase_per_index * index as f32) {
                Ok(sample) => *entry = sample,
                Err(err) => return Err(err),
            }
        }

        Ok(Self { samples })
    }

    pub fn from_samples(samples: [f32; S]) -> Self {
        Self { samples }
    }
}
