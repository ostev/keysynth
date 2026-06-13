use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};
use keyboard_protocol::{Key, KeyboardStatus, StandardKey};

use crate::gui;

pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

pub enum Event {
    Key {
        event: KeyEvent,
        keyboard: KeyboardStatus,
    },
}

pub enum InputChange {
    Keyboard(KeyboardStatus),
}

pub struct InputState {
    keyboard: KeyboardStatus,
}

impl InputState {
    pub fn new() -> InputState {
        InputState {
            keyboard: KeyboardStatus::empty(),
        }
    }

    pub async fn receive_msgs<M: RawMutex, const N: usize>(
        &mut self,
        receiver: Receiver<'_, M, InputChange, N>,
    ) -> impl Iterator<Item = Event> {
        let input_event = receiver.receive().await;

        match input_event {
            InputChange::Keyboard(keyboard_status) => {
                let diff = keyboard_status.diff(self.keyboard);
                self.keyboard = keyboard_status;

                let pressed = diff.pressed.map(KeyEvent::Pressed);
                let released = diff.released.map(KeyEvent::Released);

                let keyboard = keyboard_status;

                pressed
                    .chain(released)
                    .map(move |event| Event::Key { event, keyboard })
            }
        }
    }
}

pub fn handler<Msg: Into<gui::Msg>>(
    event: Event,
    down: Msg,
    up: Msg,
    enter: Msg,
    none: Msg,
) -> Msg {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Standard(key)) => match key {
                StandardKey::Down => down,
                StandardKey::Up => up,
                StandardKey::Enter => enter,
                _ => none,
            },
            _ => none,
        },
    }
}

pub fn handler_lazy<Msg: Into<gui::Msg>>(
    event: Event,
    down: impl FnOnce() -> Msg,
    up: impl FnOnce() -> Msg,
    enter: impl FnOnce() -> Msg,
    none: impl FnOnce() -> Msg,
) -> Msg {
    match event {
        Event::Key { event, .. } => match event {
            KeyEvent::Pressed(Key::Standard(key)) => match key {
                StandardKey::Down => down(),
                StandardKey::Up => up(),
                StandardKey::Enter => enter(),
                _ => none(),
            },
            _ => none(),
        },
    }
}
