#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for Envelope {
    fn default() -> Envelope {
        Envelope {
            attack: 0.02,
            decay: 0.1,
            sustain: 0.8,
            release: 0.9,
        }
    }
}

impl Envelope {
    const fn into_samples(self, sample_rate: f32) -> SamplesEnvelope {
        let attack = self.attack * sample_rate;
        let decay = self.decay * sample_rate;
        let release = self.release * sample_rate;

        SamplesEnvelope {
            attack_samples: attack,
            attack_reciprocal: attack.recip(),

            decay_samples: decay,
            decay_reciprocal: decay.recip(),

            sustain: self.sustain,

            release_samples: release,
            release_reciprocal: release.recip(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SamplesEnvelope {
    attack_samples: f32,
    attack_reciprocal: f32, // We store reciprocals to avoid division later

    decay_samples: f32,
    decay_reciprocal: f32,

    sustain: f32,

    release_samples: f32,
    release_reciprocal: f32,
}

/// States that need keep track of time by samples since start
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Attack(f32),
    Decay(f32),
    Sustain,
    Release(f32),
    Ended,
}

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    envelope: SamplesEnvelope,
    state: State,
}

impl Adsr {
    pub const fn new(sample_rate: f32, envelope: Envelope) -> Adsr {
        Adsr {
            envelope: envelope.into_samples(sample_rate),
            state: State::Attack(0.0),
        }
    }

    pub const fn set_envelope(&mut self, sample_rate: f32, envelope: Envelope) {
        self.envelope = envelope.into_samples(sample_rate);
    }

    pub const fn is_active(&self) -> bool {
        match self.state {
            State::Ended | State::Release(_) => false,
            _ => true,
        }
    }

    pub const fn is_ended(&self) -> bool {
        match self.state {
            State::Ended => true,
            _ => false,
        }
    }

    pub const fn release(&mut self) {
        self.state = State::Release(0.0);
    }

    pub const fn process(&mut self) -> f32 {
        match self.state {
            State::Attack(time) => {
                self.state = if time >= self.envelope.attack_samples {
                    State::Decay(0.0)
                } else {
                    State::Attack(time + 1.0)
                };

                time as f32 * self.envelope.attack_reciprocal
            }
            State::Decay(time) => {
                self.state = if time >= self.envelope.decay_samples {
                    State::Sustain
                } else {
                    State::Decay(time + 1.0)
                };

                1.0 - (1.0 - self.envelope.sustain as f32) * (time * self.envelope.decay_reciprocal)
            }
            State::Sustain => self.envelope.sustain,
            State::Release(time) => {
                self.state = if time >= self.envelope.release_samples {
                    State::Ended
                } else {
                    State::Release(time + 1.0)
                };

                self.envelope.sustain * (1.0 - time * self.envelope.release_reciprocal)
            }
            State::Ended => 0.0,
        }
    }
}
