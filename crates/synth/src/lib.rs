#![no_std]

use crate::{note::Note, voice::Voice, wavetable::Oscillator};

mod arena;
mod math;
mod note;
mod voice;
pub mod wavetable;

pub struct Synth<'a, const N: usize, const S: usize> {
    voices: [Voice<'a, S>; N],
}

pub enum PlayError {
    VoiceNotFound,
}

// impl<'a, const N: usize, const S: usize> Synth<'a, N, S> {
//     pub fn note_on(&mut self, note: Note) {
//         let free_voice = self.voices.iter_mut().find(|voice| voice.is_active());

//         free_voice.note = Some(note);

//         Ok(())
//     }

//     pub fn note_off(&mut self, note: Note) -> Result<(), PlayError> {
//         let voice = self.voices.iter_mut().find(|voice| {
//             matches!(voice.note, Some(voice_note) if voice_note.frequency == note.frequency)
//         }).ok_or(PlayError::VoiceNotFound)?;

//         voice.note = None;

//         Ok(())
//     }

//     pub fn sample(&mut self) {}
// }
