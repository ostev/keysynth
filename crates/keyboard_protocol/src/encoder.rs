use defmt::Format;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug, Format)]
pub struct Update {
    pub deltas: [i16; 2],
}

impl Update {
    pub const fn new(deltas: [i16; 2]) -> Update {
        Update { deltas }
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.deltas.into_iter().all(|delta| delta == 0)
    }
}
