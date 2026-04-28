use crate::{
    adsr::{Adsr, Envelope},
    note,
    oscillator::WavetableOscillator,
};

#[derive(Clone, Debug)]
pub struct Voice<'a, const S: usize> {
    osc_a: WavetableOscillator<'a, S>,
    osc_b: WavetableOscillator<'a, S>,
    note: note::Event,

    adsr: Adsr,
}

impl<'a, const S: usize> Voice<'a, S> {
    pub const fn new(
        sample_rate: f32,
        osc_a: WavetableOscillator<'a, S>,
        osc_b: WavetableOscillator<'a, S>,
        note: note::Event,
        envelope: Envelope,
    ) -> Self {
        Self {
            osc_a,
            osc_b,
            note,
            adsr: Adsr::new(sample_rate, envelope),
        }
    }

    /// Is the voice in the attack, decay or sustain stage?
    pub const fn is_active(&self) -> bool {
        self.adsr.is_active()
    }

    /// Is the voice not in the attack, decay, sustain or release stage?
    pub const fn is_ended(&self) -> bool {
        self.adsr.is_ended()
    }

    pub const fn sample(&mut self, blend: f32) -> f32 {
        let a = self.osc_a.sample(self.note.note);
        let b = self.osc_b.sample(self.note.note);

        let sample = a * (1.0 - blend) + b * blend;

        sample * self.adsr.process()
    }

    pub const fn release(&mut self) {
        self.adsr.release();
    }

    pub const fn note(&self) -> note::Event {
        self.note
    }
}
