use crate::{note::Note, wavetable::Wavetable};

#[derive(Clone, Copy, Debug)]
pub struct WavetableOscillator<const S: usize> {
    sample_rate_reciprocal: f32,
    index: f32,
}

impl<const S: usize> WavetableOscillator<S> {
    pub const fn new(sample_rate: f32) -> Self {
        WavetableOscillator {
            sample_rate_reciprocal: 1.0 / sample_rate,
            index: 0.0,
        }
    }

    pub const fn sample(&mut self, note: Note, wavetable: &Wavetable<S>) -> f32 {
        let increment =
            note.frequency * wavetable.samples.len() as f32 * self.sample_rate_reciprocal;
        let sample = self.lerp(wavetable);
        self.index += increment;
        self.index %= wavetable.samples.len() as f32;

        sample
    }

    const fn lerp(&self, wavetable: &Wavetable<S>) -> f32 {
        let truncated_index = self.index as usize;
        let next_index = (truncated_index + 1) % wavetable.samples.len();

        let weight = self.index - self.index as usize as f32;

        (1.0 - weight) * wavetable.samples[truncated_index] + weight * wavetable.samples[next_index]
    }
}
