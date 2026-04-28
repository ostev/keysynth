use libm::powf;

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
}
