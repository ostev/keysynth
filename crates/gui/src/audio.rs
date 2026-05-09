use synth::{Synth, wavetable::Wavetable};

pub const MAX_POLYPHONY: usize = 4;
pub const WAVETABLE_SIZE: usize = 2048;

pub struct Engine {
    synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE>,
}
