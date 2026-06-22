use defmt::println;
use embedded_hal::digital::InputPin;
use keyboard_protocol::encoder::Update;
use rotary_encoder_hal::{DefaultPhase, Direction, Rotary};

/// Represents a rotary encoder unit
pub struct Unit<A: InputPin, B: InputPin> {
    rotary: Rotary<A, B, DefaultPhase>,
}

impl<A: InputPin, B: InputPin> Unit<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            rotary: Rotary::new(a, b),
        }
    }
}

pub struct Hardware<A1: InputPin, B1: InputPin, A2: InputPin, B2: InputPin> {
    one: Unit<A1, B1>,
    two: Unit<A2, B2>,
}

impl<A1: InputPin, B1: InputPin, A2: InputPin, B2: InputPin> Hardware<A1, B1, A2, B2> {
    pub fn new(one: Unit<A1, B1>, two: Unit<A2, B2>) -> Self {
        Self { one, two }
    }

    /// Scan the rotary encode units, returning an update if there is one.
    pub fn scan(&mut self) -> Update {
        let delta_one = match self.one.rotary.update() {
            Ok(direction) => direction_to_delta(direction),
            Err(_) => {
                println!("Warning: rotary encoder one experienced an error!");
                0
            }
        };

        let delta_two = match self.two.rotary.update() {
            Ok(direction) => direction_to_delta(direction),
            Err(_) => {
                println!("Warning: rotary encoder two experienced an error!");
                0
            }
        };

        Update::new([delta_one, delta_two])
    }
}

fn direction_to_delta(direction: Direction) -> i16 {
    match direction {
        Direction::Clockwise => 1,
        Direction::CounterClockwise => -1,
        Direction::None => 0,
    }
}
