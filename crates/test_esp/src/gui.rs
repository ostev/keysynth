use alloc::{borrow::Cow, format, string::String};
use embassy_executor::task;
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::{Rgb565, RgbColor},
};
use embedded_gui::{
    app::{App, Change, State},
    component::{any_component, button::Button},
    interactive::FocusState,
    layout::{Direction, Sizing},
    primitive::{any_primitive, text::Text},
    signal::{Reactive, Signal},
};
use esp_println::println;
use esp_storage::FlashStorageError;
use esp_sync::RawMutex;
use keyboard_protocol::{Key, KeyboardDiff, KeyboardStatus};
use st7789v2::{DriverResult, St7789v2};

pub mod display;
mod editor;
mod effect;
mod event;
mod message;

use crate::{
    gui::{display::DisplayHardware, editor::source::Source, effect::Effect, message::Message},
    storage::{self, Name, Storage, StorageHardware},
    text::ByteString,
};

mod channel {
    use crate::{channel, gui::event};

    channel!(event::InputChange);
}

pub use channel::sender;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum FocusKey {
    Hello,
}

#[derive(Reactive)]
#[any_component(target = display::Driver, event = event::Event, msg = Msg, focus_key = FocusKey)]
enum AnyComponent<'a> {
    Button(Button<'a, display::Color, &'a str>),
}

#[derive(Reactive)]
#[any_primitive(target = display::Driver)]
enum AnyPrimitive<'a> {
    EditorView(editor::View<'a>),
    Spacer(embedded_gui::primitive::spacer::Spacer<'a>),
    Text(Text<'a, display::Color, &'a str>),
}

#[derive(Reactive)]
enum Page {
    Home,
    Editor,
}

// TODO: implement derive macro for enums
impl State for Page {
    fn mark_resolved(&mut self) {
        match self {
            Page::Home => {}
            Page::Editor => {}
        }
    }
}

#[derive(Reactive, State)]
struct Gui {
    page: Page,
    editor: Signal<Option<editor::State>>,
    message: Signal<Option<Message>>,
}

impl App for Gui {
    type Target = display::Driver;

    type Msg = Msg;

    type FocusKey = FocusKey;
    type Event = event::Event;
    type AnyComponent<'a> = AnyComponent<'a>;
    type AnyPrimitive<'a> = AnyPrimitive<'a>;
    type Effect = Effect;

    fn new() -> Self {
        Self {
            page: Page::Editor,
            message: Signal::new(None),
            editor: Signal::new(None),
        }
    }

    fn initial_focus_key() -> Self::FocusKey {
        FocusKey::Hello
    }

    fn background_color() -> Rgb565 {
        Rgb565::WHITE
    }

    fn update(&mut self, msg: Self::Msg) -> Change<Msg, FocusKey, Effect> {
        match (&mut self.page, msg) {
            (Page::Editor, Msg::Editor(editor_msg)) => {
                self.editor.update(|editor_option| {
                    editor_option
                        .as_mut()
                        .map(|editor| editor.update(editor_msg))
                });
            }
            (_, Msg::LoadCompleted(result)) => match result {
                Ok(source) => self
                    .editor
                    .update(|editor| *editor = Some(editor::State::new(source))),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while loading from flash! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },
            (_, Msg::SaveCompleted(result)) => match result {
                Ok(name) => {
                    let name = str::from_utf8(&name);

                    match name {
                        Ok(name) => {
                            self.message.set(Some(Message::now(Cow::Owned(format!(
                                "Saved to {}!",
                                name
                            )))));
                        }
                        Err(_) => {
                            self.message.set(Some(Message::now(Cow::Borrowed(
                                "The name of the file that was just saved to flash is invalid! Corruption has occured.",
                            ))))
                        }
                    }
                }
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while saving to flash! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },
            _ => {}
        }

        Change::none()
    }

    fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory<Self::Event, Self::Msg, Self::FocusKey>,
    ) -> embedded_gui::view::View<
        'a,
        Self::Target,
        Self::Event,
        Self::Msg,
        Self::FocusKey,
        Self::AnyComponent<'a>,
        Self::AnyPrimitive<'a>,
    > {
        match &self.page {
            Page::Home => v.view(Direction::Horizontal, [v.spacer()]),
            Page::Editor => {
                // let editor_widget = v.interactive(
                //     FocusKey::Hello,
                //     |event| Msg::Editor(editor::Msg::from_event(event)),
                //     |_| v.component(Sizing::Fill, editor.component(), []),
                // );

                // v.view(Direction::Horizontal, [editor_widget])
                todo!()
            }
        }
    }
}

pub enum Msg {
    Editor(editor::Msg),
    SaveCompleted(Result<Name, sequential_storage::Error<FlashStorageError>>),
    LoadCompleted(Result<Source, effect::LoadError>),
}

impl From<editor::Msg> for Msg {
    fn from(editor_msg: editor::Msg) -> Msg {
        Msg::Editor(editor_msg)
    }
}

#[task]
pub async fn app(mut display: DisplayHardware, mut storage: StorageHardware) {
    let mut gui = Gui::new();

    display.driver.display_on().unwrap();
    display.driver.set_brightness(0xff).unwrap();

    let Ok(_) = display.driver.clear(Gui::background_color());
    display.driver.full_flush().unwrap();

    let mut internal_state = embedded_gui::app::InternalState::new(Gui::initial_focus_key());

    let mut input_state = event::InputState::new();

    let mut effect_context = effect::Context {
        storage: Storage::new(storage),
    };

    loop {
        let Ok(_) = embedded_gui::app::render(&mut gui, &mut internal_state, &mut display.driver);

        match display.driver.flush() {
            Ok(_) => {}
            Err(error) => {
                println!(
                    "Warning: error when writing to display. Here's the error: {:?}",
                    error
                )
            }
        }

        let events = input_state.receive_msgs(channel::receiver()).await;

        embedded_gui::app::dispatch(&mut gui, &mut effect_context, &mut internal_state, events)
            .await;
    }
}
