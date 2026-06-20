use core::cell::OnceCell;

use bytemuck::NoUninit;
use embassy_executor::task;
use embassy_sync::{blocking_mutex::Mutex, once_lock::OnceLock, signal::Signal};
use esp_hal::{
    gpio::{Input, InputConfig, InputPin, Pull},
    handler,
    interrupt::{InterruptHandler, Priority},
    pcnt::{
        Pcnt,
        channel::{CtrlMode, EdgeMode},
        unit::Unit,
    },
};
use esp_sync::RawMutex;

pub use channel::receiver;

pub const STEP: f32 = 0.01;

#[derive(Clone, Copy, Debug, NoUninit, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Panel {
    #[default]
    CutoffResonance,
    AttackDecay,
    SustainRelease,
}

#[derive(Clone, Copy, Debug)]
pub enum Id {
    Primary,
    Panel(PanelId),
}

#[derive(Clone, Copy, Debug)]
pub enum PanelId {
    One,
    Two,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Increase,
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

pub struct Hardware<A: InputPin + 'static, B: InputPin + 'static> {
    pub counter: Pcnt<'static>,
    pub a: A,
    pub b: B,
}

mod channel {
    use super::Delta;
    use crate::channel;

    channel! { Delta}
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderStatus {
    pub deltas: [Delta; 3],
}

pub type Delta = i16;

const THRESHOLD: i16 = 2;
const FILTER: u16 = 10;

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

pub fn configure<A: InputPin + 'static, B: InputPin + 'static>(mut hardware: Hardware<A, B>) {
    hardware.counter.set_interrupt_handler(interrupt_handler);
    configure_unit(&mut hardware.counter.unit0, hardware.a, hardware.b);

    COUNTER.lock(|cell| {
        cell.set(hardware.counter)
            .unwrap_or_else(|_| panic!("The counter was already initialised!"))
    });
}

static COUNTER: Mutex<RawMutex, OnceCell<Pcnt<'static>>> = Mutex::new(OnceCell::new());

#[handler(priority = Priority::Priority3)]
fn interrupt_handler() {
    let sender = channel::sender();
    unsafe {
        COUNTER.lock_mut(|counter| {
            let counter = counter.get_mut().unwrap();
            let unit = &counter.unit0;

            let delta = unit.value();
            let events = unit.events();

            if events.low_limit | events.high_limit {
                // If the channel's full, then we'll drop encoder ticks. This is fine.
                let _ = sender.try_send(delta);
            }
        })
    }
}
