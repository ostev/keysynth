use crate::{
    adsr::{Adsr, Envelope, TargetRatios},
    note,
    oscillator::{self, WavetableOscillator},
    wavetable::Wavetable,
};

#[derive(Clone, Debug)]
pub struct Voice<const S: usize> {
    oscillator: WavetableOscillator<S>,
    note: note::Event,

    gain: f32,

    adsr: Adsr,
}

impl<const S: usize> Voice<S> {
    pub fn new(
        sample_rate: f32,
        oscillator: WavetableOscillator<S>,
        note: note::Event,
        gain: f32,
        envelope: Envelope,
    ) -> Self {
        Self {
            oscillator,
            note,
            gain,
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

    pub fn sample(&mut self, wavetable: &Wavetable<S>) -> f32 {
        let sample = self.oscillator.sample(self.note.note, wavetable);

        sample * self.adsr.process() * self.gain
    }

    pub const fn release(&mut self) {
        self.adsr.release();
    }

    pub const fn note(&self) -> note::Event {
        self.note
    }
}
