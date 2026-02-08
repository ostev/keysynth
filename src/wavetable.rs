pub struct Wavetable {
    pub samples: [f32; WAVETABLE_SIZE],
    /// Frequency of the samples stored in the wavetable
    pub base_frequency: f32,
}

pub const WAVETABLE_SIZE: usize = 4096;

pub struct Oscillator<'a> {
    sample_rate_reciprocal: f32,
    wavetable: &'a Wavetable,
    index: f32,
}

impl<'a> Oscillator<'a> {
    pub fn new(wavetable: &'a Wavetable, sample_rate: f32) -> Self {
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

        let weight = self.index - self.index.trunc();

        (1.0 - weight) * self.wavetable.samples[truncated_index]
            + weight * self.wavetable.samples[next_index]
    }
}
