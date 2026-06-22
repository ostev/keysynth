use embassy_executor::task;
use embassy_rp::gpio::{Input, Output};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Instant};
use enumflags2::BitFlags;
use keyboard_protocol::{
    Key, KeyboardStatus, StandardKey,
    layout::{self, Layout},
};

pub const NUM_ROWS: usize = 6;
pub const NUM_COLUMNS: usize = 14;

pub struct HardwareLayout<const R: usize, const C: usize> {
    pub rows: [Input<'static>; R],
    pub columns: [Output<'static>; C],
}

impl<const R: usize, const C: usize> HardwareLayout<R, C> {
    pub fn scan(
        &mut self,
        layout: &Layout<R, C>,
        debouncer: &mut Debouncer<R, C>,
    ) -> KeyboardStatus {
        let mut standard_keys = [StandardKey::None; 6];
        let mut num_standard_keys: usize = 0;

        let mut modifier_bitfield = BitFlags::empty();
        let mut special_bitfield = BitFlags::empty();

        for (column_index, column_output) in self.columns.iter_mut().enumerate() {
            column_output.set_high();

            // Small delay required for pin state to change
            embassy_time::block_for(Duration::from_micros(2));

            for (row_index, row_input) in self.rows.iter().enumerate() {
                let is_pressed = debouncer.update(row_index, column_index, row_input.is_high());

                if is_pressed {
                    let key = layout.rows[row_index][column_index];

                    match key {
                        Key::Standard(key) => {
                            // Ignore keys past the maximum allowed by the USB spec
                            if num_standard_keys < standard_keys.len() {
                                standard_keys[num_standard_keys] = key;
                                num_standard_keys += 1;
                            }
                        }
                        Key::Modifier(modifier) => {
                            modifier_bitfield |= modifier;
                        }
                        Key::Special(special) => {
                            special_bitfield |= special;
                        }
                    }
                }
            }

            column_output.set_low();
        }

        KeyboardStatus {
            keys: standard_keys,
            modifier_bitfield,
            special_bitfield,
        }
    }
}

#[derive(Copy, Clone)]
struct DebouncedKey {
    is_pressed: bool,
    last_changed: Instant,
}
impl DebouncedKey {
    pub fn new() -> DebouncedKey {
        DebouncedKey {
            is_pressed: false,
            last_changed: Instant::now(),
        }
    }
}

pub struct Debouncer<const R: usize, const C: usize> {
    keys: [[DebouncedKey; C]; R],
}

impl<const R: usize, const C: usize> Debouncer<R, C> {
    pub fn new() -> Self {
        Self {
            keys: core::array::repeat(core::array::repeat(DebouncedKey::new())),
        }
    }

    pub fn update(&mut self, row: usize, column: usize, raw_is_pressed: bool) -> bool {
        const INITIAL_DEBOUNCE_DURATION: Duration = Duration::from_millis(2);
        const DEFER_DEBOUNCE_DURATION: Duration = Duration::from_millis(3);

        let key = &mut self.keys[row][column];
        let debounced = key.is_pressed;
        let elapsed = Instant::now() - key.last_changed;

        if raw_is_pressed == debounced {
            return debounced;
        }

        let threshold = if raw_is_pressed {
            INITIAL_DEBOUNCE_DURATION
        } else {
            DEFER_DEBOUNCE_DURATION
        };

        if elapsed >= threshold {
            key.is_pressed = raw_is_pressed;
            key.last_changed = Instant::now();
            raw_is_pressed
        } else {
            // Still debouncing: keep reporting the old (debounced) state
            debounced
        }
    }
}
