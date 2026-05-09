use crate::{
    adsr::{Adsr, Envelope, TargetRatios},
    note,
    oscillator::WavetableOscillator,
    wavetable::Wavetable,
};

#[derive(Clone, Debug)]
pub struct Voice<const S: usize> {
    osc_a: WavetableOscillator<S>,
    osc_b: WavetableOscillator<S>,
    note: note::Event,

    adsr: Adsr,
}

impl<const S: usize> Voice<S> {
    pub fn new(
        sample_rate: f32,
        osc_a: WavetableOscillator<S>,
        osc_b: WavetableOscillator<S>,
        note: note::Event,
        envelope: Envelope,
    ) -> Self {
        Self {
            osc_a,
            osc_b,
            note,
            adsr: Adsr::new(
                sample_rate,
                envelope,
                TargetRatios {
                    attack: 0.3,
                    decay_release: 0.0001,
                },
            ),
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

    pub fn sample(&mut self, wavetables: &[Wavetable<S>; 2], blend: f32) -> f32 {
        let a = self.osc_a.sample(self.note.note, &wavetables[0]);
        let b = self.osc_b.sample(self.note.note, &wavetables[1]);

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
