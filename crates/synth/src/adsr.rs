use micromath::F32Ext;

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

    pub const fn is_active(&self) -> bool {
        match self.state {
            State::Ended | State::Release => false,
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
        self.state = State::Release;
    }

    pub fn process(&mut self) -> f32 {
        match self.state {
            State::Attack => {
                self.output = {
                    let output =
                        self.envelope.attack_base + self.output * self.envelope.attack_coefficient;
                    if output < 1.0 {
                        output
                    } else {
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
                        self.state = State::Sustain;

                        self.envelope.sustain_level
                    }
                };
            }
            State::Sustain => {}
            State::Release => {
                self.output = {
                    let output = self.envelope.release_base
                        + self.output * self.envelope.release_coefficient;

                    if output > 0.0 {
                        output
                    } else {
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
