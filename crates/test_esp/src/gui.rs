use embassy_executor::task;
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::{Rgb565, RgbColor},
    text::Text,
};
use embedded_gui::{
    app::App,
    interactive::FocusState,
    layout::{Direction, Sizing},
};
use esp_println::println;
use esp_sync::RawMutex;
use keyboard_protocol::{Key, KeyboardDiff, KeyboardStatus};
use st7789v2::{DriverResult, St7789v2};

pub mod display;
mod editor;

use crate::{gui::display::DisplayHardware, receiver, sender};

sender!(InputEvent);

pub enum InputEvent {
    Keyboard(KeyboardStatus),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum FocusKey {
    Hello,
}

enum Page {
    Home,
    Editor(editor::Model),
}

struct Gui {
    page: Page,
}

impl App for Gui {
    type Target = display::Driver;

    type Msg = Msg;

    type FocusKey = FocusKey;
    type Event = Event;

    fn new() -> Self {
        Self { page: Page::Home }
    }

    fn initial_focus_key() -> Self::FocusKey {
        FocusKey::Hello
    }

    fn background_color() -> Rgb565 {
        Rgb565::WHITE
    }

    fn update(&mut self, msg: Self::Msg) -> Option<(FocusKey, FocusState)> {
        // match msg {
        //     Msg::Key(key_event) => todo!(),
        // }
        None
    }

    fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory<Self::FocusKey, Self::Event, Self::Msg>,
    ) -> embedded_gui::view::View<'a, Self::Target, Self::FocusKey, Self::Event, Self::Msg> {
        v.view(
            Direction::Horizontal,
            [
                v.spacer(),
                // v.primitive(
                //     Sizing::Intrinsic,
                //     Text {
                //         content: self.text.to_ref(),
                //         font_style: self.font_style.to_ref(),
                //     },
                // ),
                // v.spacer(),
                // v.primitive(
                //     Sizing::Intrinsic,
                //     Text {
                //         content: self.text.to_ref(),
                //         font_style: self.font_style.to_ref(),
                //     },
                // ),
                // v.component(
                //     Sizing::Fill,
                //     Button {
                //         text: SignalRef::owned("Say hi!"),
                //         font_style: self.font_style.to_ref(),
                //         size: SignalRef::owned(Size::new(128, 32)),
                //     },
                //     [],
                // ),
            ],
        )
    }
}

enum Msg {}

enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

enum Event {
    Key(KeyEvent),
}

struct InputState {
    keyboard: KeyboardStatus,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            keyboard: KeyboardStatus::new(),
        }
    }

    async fn receive_msgs(&mut self) -> impl Iterator<Item = Event> {
        let input_event = CHANNEL.receiver().receive().await;
        match input_event {
            InputEvent::Keyboard(keyboard_status) => {
                let diff = keyboard_status.diff(self.keyboard);

                let pressed = diff.pressed.map(KeyEvent::Pressed);
                let released = diff.released.map(KeyEvent::Released);

                pressed.chain(released).map(Event::Key)
            }
        }
    }
}

#[task]
pub async fn app(mut hardware: DisplayHardware) {
    let mut gui = Gui::new();

    hardware.driver.display_on().unwrap();
    hardware.driver.set_brightness(0xff).unwrap();

    let Ok(_) = hardware.driver.clear(Gui::background_color());
    hardware.driver.full_flush().unwrap();

    let mut internal_state = embedded_gui::app::InternalState::new(Gui::initial_focus_key());

    let mut input_state = InputState::new();

    loop {
        let Ok(_) = embedded_gui::app::render(&gui, &mut internal_state, &mut hardware.driver);

        match hardware.driver.flush() {
            Ok(_) => {}
            Err(error) => {
                println!(
                    "Warning: error when writing to display. Here's the error: {:?}",
                    error
                )
            }
        }

        let events = input_state.receive_msgs().await;

        embedded_gui::app::dispatch(&mut gui, &mut internal_state, events);
    }
}
