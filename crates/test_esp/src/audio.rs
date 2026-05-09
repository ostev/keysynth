use core::future;

use synth::{Synth, note::Note, wavetable::Wavetable};

pub const MAX_POLYPHONY: usize = 4;
pub const WAVETABLE_SIZE: usize = 2048;

pub const SAMPLE_RATE: u32 = synth::DEFAULT_SAMPLE_RATE;

pub struct Engine {
    synth: Synth<MAX_POLYPHONY, WAVETABLE_SIZE>,
}
