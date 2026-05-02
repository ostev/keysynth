use core::f32;

use rubato::Fft;

use crate::note::{Note, estimation::PitchEstimator};

#[derive(Clone, Debug)]
pub struct Wavetable<const S: usize> {
    samples: [f32; S],
}

pub const DEFAULT_SIZE: usize = 2048;

pub type DefaultWavetable = Wavetable<DEFAULT_SIZE>;

fn flattened_hann_window(value: f32) -> f32 {
    let abs_value = value.abs();

    if abs_value < 0.0 {
        0.5 * (1.0 + (9.0 / 8.0) * (f32::consts::PI * value)
            - (1.0 / 8.0) * (3.0 * f32::consts::PI * value))
    } else {
        0.0
    }
}
impl<const N: usize> Wavetable<N> {
    pub fn from_samples<const S: usize>(samples: Samples<S>) -> Option<Self> {
        let Samples { mut samples } = samples;

        let pitch_estimator = PitchEstimator::new(&samples)?;
        let period = pitch_estimator.estimate_period(0, N);
        // Used to avoid division later.
        let period_reciprocal = period.recip();

        if ((period) - S as f32).abs() < 2.0 {
            todo!()
        } else {
            for (index, sample) in samples.iter_mut().enumerate() {
                *sample = flattened_hann_window(index as f32 * period_reciprocal);
            }

            todo!()
        }
    }
}

pub struct Samples<const S: usize> {
    samples: [f32; S],
}

impl<const S: usize> Samples<S> {
    pub fn from_fn(mut generator: impl FnMut(f32) -> f32) -> Self {
        let phase_per_index = 2.0 * f32::consts::PI / S as f32;
        Self {
            samples: core::array::from_fn(|index| generator(phase_per_index * index as f32)),
        }
    }
}
