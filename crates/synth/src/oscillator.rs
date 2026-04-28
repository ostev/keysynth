use crate::{note::Note, wavetable::Wavetable};

#[derive(Clone, Copy, Debug)]
pub struct WavetableOscillator<'a, const S: usize> {
    sample_rate_reciprocal: f32,
    wavetable: &'a Wavetable<S>,
    index: f32,
}

impl<'a, const S: usize> WavetableOscillator<'a, S> {
    pub const fn new(wavetable: &'a Wavetable<S>, sample_rate: f32) -> Self {
        WavetableOscillator {
            wavetable,
            sample_rate_reciprocal: 1.0 / sample_rate,
            index: 0.0,
        }
    }

    pub const fn sample(&mut self, note: Note) -> f32 {
        let increment =
            note.frequency * self.wavetable.samples.len() as f32 * self.sample_rate_reciprocal;
        let sample = self.lerp();
        self.index += increment;
        self.index %= self.wavetable.samples.len() as f32;

        sample
    }

    const fn lerp(&self) -> f32 {
        let truncated_index = self.index as usize;
        let next_index = (truncated_index + 1) % self.wavetable.samples.len();

        let weight = self.index - self.index as usize as f32;

        (1.0 - weight) * self.wavetable.samples[truncated_index]
            + weight * self.wavetable.samples[next_index]
    }
}
