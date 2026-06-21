use libm::powf;
use micromath::F32Ext;

pub(crate) mod estimation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub note: Note,
    /// A timestamp in microseconds. Ensure that the timebase is consistent.
    pub timestamp: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Note {
    pub frequency: f32,
}

impl Note {
    pub const C1: Note = Note::new(32.70320);

    pub const B2: Note = Note::new(123.4708);
    pub const C2: Note = Note::new(65.40639);

    pub const C3: Note = Note::new(130.8128);

    pub const A4: Note = Note::new(440.0);
    pub const C4: Note = Note::new(261.6256);

    pub const C5: Note = Note::new(523.2511);

    pub const C6: Note = Note::new(1046.502);

    pub const C7: Note = Note::new(2093.005);

    pub const C8: Note = Note::new(4186.009);

    pub const fn new(frequency: f32) -> Note {
        Note { frequency }
    }

    pub fn from_semitones(semitones: f32, reference: Note) -> Note {
        Note::new(reference.frequency * powf(2.0, semitones / 12.0))
    }

    /// Convert a note from its frequency to its corresponding MIDI number
    pub fn midi_number(self) -> Option<u8> {
        // Standard equal-temperament formula
        let midi = (69.0 + 12.0 * libm::log2f(self.frequency / 440.0)).round();

        if midi >= 0.0 && midi <= 127.0 {
            // It's within the MIDI note range
            Some(midi as u8)
        } else {
            None
        }
    }

    /// Convert a note into its human-readable name
    pub fn to_name(self) -> Option<heapless::String<4>> {
        let midi = self.midi_number()?;

        let octave = (midi / 12).saturating_sub(1);
        // Get the remainder
        let note = midi % 12;

        Some(heapless::format!("{}{}", NAMES[note as usize], octave).unwrap())
    }
}

const NAMES: [&'static str; 12] = [
    // Notes without an accidental are padded with a space to ensure consistency.
    "C ", "C#", "D ", "D#", "E ", "F ", "F#", "G ", "G#", "A ", "A#", "B ",
];

mod test {
    use super::*;

    #[test]
    fn notes_match_names() {
        assert_eq!(Note::C1.to_name(), Some(heapless::format!("C1").unwrap()));
        assert_eq!(Note::C2.to_name(), Some(heapless::format!("C2").unwrap()));
        assert_eq!(Note::B2.to_name(), Some(heapless::format!("B2").unwrap()));
        assert_eq!(Note::C3.to_name(), Some(heapless::format!("C3").unwrap()));
        assert_eq!(Note::C4.to_name(), Some(heapless::format!("C4").unwrap()));
        assert_eq!(Note::A4.to_name(), Some(heapless::format!("A4").unwrap()));
        assert_eq!(Note::C5.to_name(), Some(heapless::format!("C5").unwrap()));
        assert_eq!(Note::C6.to_name(), Some(heapless::format!("C6").unwrap()));
        assert_eq!(Note::C7.to_name(), Some(heapless::format!("C7").unwrap()));
        assert_eq!(Note::C8.to_name(), Some(heapless::format!("C8").unwrap()));
    }
}
