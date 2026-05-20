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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    fn init() -> Self {
        Self { page: Page::Home }
    }

    fn default_focus_state() -> FocusState {
        FocusState::Unfocused
    }

    fn default_focus_key() -> Self::FocusKey {
        FocusKey::Hello
    }

    fn background_color() -> Rgb565 {
        Rgb565::WHITE
    }

    fn update(&mut self, msg: Self::Msg) {
        match msg {
            Msg::Key(key_event) => todo!(),
        }
    }

    fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory,
    ) -> embedded_gui::view::View<'a, Self::Target, Self::FocusKey> {
        // v.view(
        //     Direction::Horizontal,
        //     [
        //         v.spacer(),
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
        // ],
        // )
    }
}

enum Msg {
    Key(KeyEvent),
}

enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

struct InputState {
    keyboard: KeyboardStatus,
}

impl InputState {
    const fn new() -> InputState {
        InputState {
            keyboard: KeyboardStatus::new(),
        }
    }

    async fn receive_msgs(&mut self) -> impl Iterator<Item = Msg> {
        let input_event = CHANNEL.receiver().receive().await;
        match input_event {
            InputEvent::Keyboard(keyboard_status) => {
                let diff = keyboard_status.diff(&self.keyboard);

                let pressed = diff.pressed.map(KeyEvent::Pressed);
                let released = diff.released.map(KeyEvent::Released);

                pressed.chain(released).map(Msg::Key)
            }
        }
    }
}

#[task]
pub async fn app(mut hardware: DisplayHardware) {
    let gui = Gui::init();

    hardware.driver.display_on().unwrap();
    hardware.driver.set_brightness(0xff).unwrap();

    let Ok(_) = hardware.driver.clear(Gui::background_color());
    hardware.driver.full_flush().unwrap();

    let mut view_factory = embedded_gui::view::Factory::new();

    let mut render = || {
        let Ok(_) = embedded_gui::app::render(&gui, &mut view_factory, &mut hardware.driver);

        match hardware.driver.flush() {
            Ok(_) => {}
            Err(error) => {
                println!(
                    "Warning: error when writing to display. Here's the error: {:?}",
                    error
                )
            }
        }
    };

    let input_state = InputState::new();

    render();

    loop {
        let msgs = input_state.receive_msgs();

        for msg in msgs {
            gui.update(msg);
        }

        render()
    }
}
