#![no_std]

use crate::{voice::Voice, wavetable::Oscillator};

mod arena;
mod math;
mod note;
mod voice;
pub mod wavetable;

pub struct Synth<'a, const N: usize, const S: usize> {
    voices: [Voice<'a, S>; N],
}

impl<'a, const N: usize, const S: usize> Synth<'a, N, S> {
    pub fn note_on(&mut self) {}
}
