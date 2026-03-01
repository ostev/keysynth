use crate::{note::Note, wavetable::Oscillator};

pub struct Location {
    pub x: u16,
    pub y: u16,
}

pub struct Voice<'a, const S: usize> {
    oscillators: [(Oscillator<'a, S>, Location); 4],
    pub location: Location,
    pub note: Option<Note>,
}

impl<'a, const S: usize> Voice<'a, S> {
    pub fn sample(&mut self, blend: Location) -> f32 {
        match self.note {
            None => 0.0,
            Some(note) => {
                let samples = self.oscillators.iter_mut().map(|(oscillator, location)| {
                    (
                        oscillator.sample(note.frequency),
                        (location.x as i32 - blend.x as i32),
                    )
                });
            }
        }
    }
}
