use crate::{note, wavetable::Oscillator};

#[derive(Clone, Copy)]
pub struct Voice<'a, const S: usize> {
    osc_a: Oscillator<'a, S>,
    osc_b: Oscillator<'a, S>,
    note: note::Event,
}

impl<'a, const S: usize> Voice<'a, S> {
    pub const fn new(
        osc_a: Oscillator<'a, S>,
        osc_b: Oscillator<'a, S>,
        note: note::Event,
    ) -> Self {
        Self { osc_a, osc_b, note }
    }

    pub const fn sample(&mut self, blend: f32) -> f32 {
        let a = self.osc_a.sample(self.note.note);
        let b = self.osc_b.sample(self.note.note);

        a * (1.0 - blend) + b * blend
    }

    pub const fn note(&self) -> note::Event {
        self.note
    }
}
