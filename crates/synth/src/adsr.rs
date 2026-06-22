use micromath::F32Ext;

/// Represents an attack-decay-sustain-release (ADSR) envelope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    /// A parameter between 0.0 and 1.0 controlling how fast the sound intensifies.
    pub attack: f32,
    /// A parameter between 0.0 and 1.0 controlling how long the sound lasts at that
    /// maximum intensity.
    pub decay: f32,
    /// A parameter between 0.0 and 1.0 controlling how intensity the sound is while
    /// it is being held.
    pub sustain: f32,
    /// A parameter greater than 0.0 controlling how long the sound lasts after it has
    /// been released.
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

/// Represents an enevelope in terms of samples rather than arbitrary units.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SamplesEnvelope {
    attack_rate: f32,
    attack_coefficient: f32,
    attack_base: f32,

    decay_rate: f32,
    decay_coefficient: f32,
    decay_base: f32,

    release_rate: f32,
    release_coefficient: f32,
    release_base: f32,

    sustain_level: f32,
}
impl SamplesEnvelope {
    fn calculate_coefficient(rate: f32, target_ratio: f32) -> f32 {
        if rate > 0.0 {
            ((-((target_ratio + 1.0) / target_ratio).ln()) / rate).exp()
        } else {
            0.0
        }
    }

    /// Creates a `SamplesEnvelope` from an envelope, a supplied sample rate and a set of target
    /// ratios.
    fn new(envelope: Envelope, sample_rate: f32, target_ratios: TargetRatios) -> SamplesEnvelope {
        let attack_rate = envelope.attack * sample_rate;
        let attack_coefficient = Self::calculate_coefficient(attack_rate, target_ratios.attack);
        let attack_base = (1.0 + target_ratios.attack) * (1.0 - attack_coefficient);

        let decay_rate = envelope.decay * sample_rate;
        let decay_coefficient =
            Self::calculate_coefficient(decay_rate, target_ratios.decay_release);
        let decay_base =
            (envelope.sustain - target_ratios.decay_release) * (1.0 - decay_coefficient);

        let release_rate = envelope.release * sample_rate;
        let release_coefficient =
            Self::calculate_coefficient(release_rate, target_ratios.decay_release);
        let release_base = -target_ratios.decay_release * (1.0 - release_coefficient);

        SamplesEnvelope {
            attack_rate,
            attack_coefficient,
            attack_base,

            decay_rate,
            decay_coefficient,
            decay_base,

            release_rate,
            release_coefficient,
            release_base,

            sustain_level: envelope.sustain,
        }
    }
}

/// Controls some of the shape of the curve of the envelope.
/// It represents how much the envelope is trying to overshoot
/// its target during exponential calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetRatios {
    pub attack: f32,
    pub decay_release: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Attack,
    Decay,
    Sustain,
    Release,
    Ended,
}

/// An ADSR envelope generator
#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    envelope: SamplesEnvelope,
    state: State,
    sample_rate: f32,
    output: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32, envelope: Envelope, target_ratios: TargetRatios) -> Adsr {
        Adsr {
            sample_rate,
            envelope: SamplesEnvelope::new(envelope, sample_rate, target_ratios),
            state: State::Attack,
            output: 0.0,
        }
    }

    pub fn set_envelope_and_ratios(&mut self, envelope: Envelope, target_ratios: TargetRatios) {
        self.envelope = SamplesEnvelope::new(envelope, self.sample_rate, target_ratios);
    }

    /// Whether the ADSR is actively being held.
    pub const fn is_active(&self) -> bool {
        match self.state {
            State::Ended | State::Release => false,
            _ => true,
        }
    }

    /// Whether the ADSR has completely finished (i.e. moved past the release stage).
    pub const fn is_ended(&self) -> bool {
        match self.state {
            State::Ended => true,
            _ => false,
        }
    }

    /// Release the note immediately.
    pub const fn release(&mut self) {
        self.state = State::Release;
    }

    /// Update the ADSR
    pub fn process(&mut self) -> f32 {
        match self.state {
            State::Attack => {
                self.output = {
                    let output =
                        self.envelope.attack_base + self.output * self.envelope.attack_coefficient;
                    if output < 1.0 {
                        output
                    } else {
                        // We're finished, so move to decay
                        self.state = State::Decay;

                        1.0
                    }
                };
            }
            State::Decay => {
                self.output = {
                    let output =
                        self.envelope.decay_base + self.output * self.envelope.decay_coefficient;
                    if output > self.envelope.sustain_level {
                        output
                    } else {
                        // Move to sustain
                        self.state = State::Sustain;

                        self.envelope.sustain_level
                    }
                };
            }
            State::Sustain => {
                // We're only released from sustain when the `release` method is called.
            }
            State::Release => {
                self.output = {
                    let output = self.envelope.release_base
                        + self.output * self.envelope.release_coefficient;

                    if output > 0.0 {
                        output
                    } else {
                        // We're done!
                        self.state = State::Ended;

                        0.0
                    }
                };
            }
            State::Ended => {}
        };

        self.output
    }
}
