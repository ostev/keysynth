use core::f32;

pub struct Wavetable<const S: usize> {
    samples: [f32; S],
}

pub const DEFAULT_SIZE: usize = 300;

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

pub const DEFAULT_SAMPLE_RATE: f32 = 44_100.0;

pub struct Oscillator<'a, const S: usize> {
    sample_rate_reciprocal: f32,
    wavetable: &'a Wavetable<S>,
    index: f32,
}

impl<'a, const S: usize> Oscillator<'a, S> {
    pub fn new(wavetable: &'a Wavetable<S>, sample_rate: f32) -> Self {
        Oscillator {
            wavetable,
            sample_rate_reciprocal: 1.0 / sample_rate,
            index: 0.0,
        }
    }

    pub fn sample(&mut self, frequency: f32) -> f32 {
        let increment =
            frequency * self.wavetable.samples.len() as f32 * self.sample_rate_reciprocal;
        let sample = self.lerp();
        self.index += increment;
        self.index %= self.wavetable.samples.len() as f32;

        sample
    }

    fn lerp(&self) -> f32 {
        let truncated_index = self.index as usize;
        let next_index = (truncated_index + 1) % self.wavetable.samples.len();

        let weight = self.index - self.index as usize as f32;

        (1.0 - weight) * self.wavetable.samples[truncated_index]
            + weight * self.wavetable.samples[next_index]
    }
}
