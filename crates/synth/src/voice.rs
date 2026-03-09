use crate::{note::Note, wavetable::Oscillator};

pub struct Voice<'a, const S: usize> {
    osc_a: Oscillator<'a, S>,
    osc_b: Oscillator<'a, S>,
    pub note: Note,
}

impl<'a, const S: usize> Voice<'a, S> {
    pub fn sample(&mut self, blend: f32) -> f32 {
        match &self.note {
            None => 0.0,
            Some(note) => {
                let a = self.osc_a.sample(note.frequency);
                let b = self.osc_b.sample(note.frequency);

                a * (1.0 - blend) + b * blend
            }
        }
    }
}
