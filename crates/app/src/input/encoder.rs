use core::cell::OnceCell;

use bytemuck::NoUninit;
use embassy_sync::blocking_mutex::Mutex;
use esp_hal::{
    gpio::{Input, InputConfig, InputPin, Pull},
    handler,
    interrupt::Priority,
    pcnt::{
        Pcnt,
        channel::{CtrlMode, EdgeMode},
        unit::Unit,
    },
};
use esp_sync::RawMutex;

mod channel {
    use super::Delta;
    use crate::channel;

    channel!(Delta, 32);
}
pub use channel::receiver;

/// The amount each encoder detent changes a parameter by.
pub const STEP: f32 = 0.01;

/// Identifies the currently selected parameter panel.

#[derive(Clone, Copy, Debug, NoUninit, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Panel {
    /// Filter cutoff and resonance controls.
    #[default]
    CutoffResonance,

    /// Envelope attack and decay controls.
    AttackDecay,

    /// Envelope sustain and release controls.
    SustainRelease,
}

/// Identifies one of the rotary encoders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    /// The primary encoder.
    Primary,
    /// One of the two panel encoders.
    Panel(PanelId),
}

/// Identifies a panel encoder.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelId {
    /// The secondary encoder.
    One,
    /// The tertiary encoder.
    Two,
}

/// The direction an encoder was rotated.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Clockwise rotation.
    Increase,
    /// Counter-clockwise rotation.
    Decrease,
}

impl From<Direction> for f32 {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Increase => 1.0,
            Direction::Decrease => -1.0,
        }
    }
}

/// Hardware resources required to configure an encoder.
pub struct Hardware<A: InputPin + 'static, B: InputPin + 'static> {
    pub counter: Pcnt<'static>,
    pub a: A,
    pub b: B,
}

/// Relative movement reported by each encoder since the last update.
#[derive(Clone, Copy, Debug)]
pub struct EncoderStatus {
    pub deltas: [Delta; 3],
}

/// Signed encoder movement measured in detents.
pub type Delta = i16;

const THRESHOLD: i16 = 3;
const FILTER: u16 = 12;

/// Configures an encoder unit with thresholds, interrupts and filtering.
fn configure_unit<'d, const N: usize>(
    unit: &mut Unit<'d, N>,
    a: impl InputPin + 'd,
    b: impl InputPin + 'd,
) {
    let config = InputConfig::default().with_pull(Pull::Up);
    let input_a = Input::new(a, config);
    let input_b = Input::new(b, config);

    unit.set_low_limit(Some(-THRESHOLD)).unwrap();
    unit.set_high_limit(Some(THRESHOLD)).unwrap();
    unit.set_filter(Some(FILTER)).unwrap();

    unit.channel0.set_ctrl_signal(input_a.peripheral_input());
    unit.channel0.set_edge_signal(input_b.peripheral_input());
    unit.channel0
        .set_ctrl_mode(CtrlMode::Reverse, CtrlMode::Keep);
    unit.channel0
        .set_input_mode(EdgeMode::Increment, EdgeMode::Decrement);

    unit.channel1.set_ctrl_signal(input_b);
    unit.channel1.set_edge_signal(input_a);
    unit.channel1
        .set_ctrl_mode(CtrlMode::Reverse, CtrlMode::Keep);
    unit.channel1
        .set_input_mode(EdgeMode::Decrement, EdgeMode::Increment);

    unit.listen();
}

/// Configures the global encoder driver.
///
/// # Panics
/// Panics if the encoder driver has already been configured.
pub fn configure<A: InputPin + 'static, B: InputPin + 'static>(mut hardware: Hardware<A, B>) {
    hardware.counter.set_interrupt_handler(interrupt_handler);
    configure_unit(&mut hardware.counter.unit0, hardware.a, hardware.b);

    COUNTER.lock(|cell| {
        cell.set(hardware.counter)
            .unwrap_or_else(|_| panic!("The counter was already initialised!"))
    });
}

static COUNTER: Mutex<RawMutex, OnceCell<Pcnt<'static>>> = Mutex::new(OnceCell::new());

/// Interrupt handler for encoder movement.
#[handler(priority = Priority::Priority3)]
fn interrupt_handler() {
    let sender = channel::sender();
    let events = unsafe {
        COUNTER.lock_mut(|counter| {
            let counter = counter.get_mut().unwrap();
            let unit = &counter.unit0;

            let events = unit.events();

            unit.reset_interrupt();
            unit.clear();

            events
        })
    };

    let delta = if events.low_limit {
        -1
    } else if events.high_limit {
        1
    } else {
        0
    };
    // If the channel is full, we'll drop encoder ticks. This is fine.
    let _ = sender.try_send(delta);
}
