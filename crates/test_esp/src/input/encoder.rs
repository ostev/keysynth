use core::cell::OnceCell;

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
use static_cell::StaticCell;
use usbd_hid::descriptor::MediaKey::Mute;

pub struct EncoderHardware {
    pub counter: Pcnt<'static>,
}

mod channel {
    use super::EncoderStatus;
    use crate::channel;

    channel! { EncoderStatus }
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderStatus {
    pub delta: [i32; 3],
}

const THRESHOLD: i16 = 2;
const FILTER: u16 = 10;

pub fn configure_unit<'d, const N: usize>(
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

#[handler(priority = Priority::Priority3)]
fn interrupt_handler() {
    // let mut u0_ref = UNIT0.borrow_ref_mut(cs);
    // if let Some(ref mut u0) = *u0_ref {
    //     // Check what exact event fired this interrupt vector
    //     let events = u0.get_events();

    //     if events.high_limit {
    //         OVERFLOW_COUNT.fetch_add(1, Ordering::Relaxed);
    //     } else if events.low_limit {
    //         OVERFLOW_COUNT.fetch_sub(1, Ordering::Relaxed);
    //     }

    //     // CRITICAL: Clear the interrupt bit so it can fire again
    //     u0.reset_interrupt();
    // }
    unsafe {
        COUNTER.lock_mut(|counter| {
            let counter = counter.get_mut().unwrap();
            // let units = [counter.unit0, counter.unit1, counter.unit2];

            // for unit in units {}
        })
    }
}

static STATE_CHANGE: Signal<RawMutex, ()> = Signal::new();

static COUNTER: Mutex<RawMutex, OnceCell<Pcnt<'static>>> = Mutex::new(OnceCell::new());

pub fn start(mut hardware: EncoderHardware) {
    critical_section::with(|_| {
        hardware.counter.set_interrupt_handler(interrupt_handler);

        COUNTER.lock(|cell| {
            cell.set(hardware.counter)
                .unwrap_or_else(|_| panic!("The counter was already initialised!"))
        });
    });
}

// #[task]
// pub async fn encoder(hardware: EncoderHardware) -> ! {
//     let EncoderHardware { mut counter } = hardware;

//     counter.set_interrupt_handler(interrupt_handler);
//     COUNTER.init(counter);

//     // let mut encoder_state = [0; 3];

//     loop {
//         // STATE_CHANGE.wait().await;

//         // let new_encoder_state = [
//         //     counter.unit0.value(),
//         //     counter.unit1.value(),
//         //     counter.unit2.value(),
//         // ];
//         // let delta = core::array::from_fn(|index| {
//         //     new_encoder_state[index] as i32 - encoder_state[index] as i32
//         // });

//         // encoder_state = new_encoder_state;

//         // channel::sender().send(EncoderStatus { delta }).await;
//     }
// }
