use crate::{Key, Modifier, modifier, special, standard};

pub struct Layout<const ROWS: usize, const COLUMNS: usize> {
    pub rows: [[Key; COLUMNS]; ROWS],
}

impl<const ROWS: usize, const COLUMNS: usize> Layout<ROWS, COLUMNS> {
    pub const NUM_ROWS: usize = ROWS;
    pub const NUM_COLUMNS: usize = COLUMNS;

    pub const fn new(rows: [[Key; COLUMNS]; ROWS]) -> Self {
        Self { rows }
    }
}

pub const STANDARD: Layout<6, 14> = Layout::new([
    [
        standard!(Esc),
        standard!(F1),
        standard!(F2),
        standard!(F3),
        standard!(F4),
        standard!(F5),
        standard!(F6),
        standard!(F7),
        standard!(F8),
        standard!(F9),
        standard!(F10),
        standard!(F11),
        standard!(F12),
        standard!(Home),
    ],
    [
        standard!(End),
        standard!(Delete),
        standard!(Grave),
        standard!(Key1),
        standard!(Key2),
        standard!(Key3),
        standard!(Key4),
        standard!(Key5),
        standard!(Key6),
        standard!(Key7),
        standard!(Key8),
        standard!(Key9),
        standard!(Key0),
        standard!(Minus),
    ],
    [
        standard!(Equal),
        standard!(Backspace),
        special!(One),
        standard!(Tab),
        standard!(Q),
        standard!(W),
        standard!(E),
        standard!(R),
        standard!(T),
        standard!(Y),
        standard!(U),
        standard!(I),
        standard!(O),
        standard!(P),
    ],
    [
        standard!(LeftBracket),
        standard!(RightBracket),
        standard!(Backslash),
        special!(Two),
        standard!(CapsLock),
        standard!(A),
        standard!(S),
        standard!(D),
        standard!(F),
        standard!(G),
        standard!(H),
        standard!(J),
        standard!(K),
        standard!(L),
    ],
    [
        standard!(Semicolon),
        standard!(Apostrophe),
        standard!(Enter),
        special!(Three),
        modifier!(LeftShift),
        standard!(Z),
        standard!(X),
        standard!(C),
        standard!(V),
        standard!(B),
        standard!(N),
        standard!(M),
        standard!(Comma),
        standard!(Dot),
    ],
    [
        standard!(Slash),
        modifier!(RightShift),
        standard!(Up),
        special!(Four),
        modifier!(LeftControl),
        // Mac-style layout for modifiers
        modifier!(LeftAlt),
        modifier!(LeftSuper),
        standard!(Space),
        // Two command keys are more useful than two option keys
        modifier!(RightSuper),
        // We need an function key to send media commands.
        // Also, a lot of Mac shortcuts rely on a function key.
        special!(Fn),
        // Two control keys are also more useful than two option keys
        modifier!(RightControl),
        standard!(Left),
        standard!(Down),
        standard!(Right),
    ],
]);
