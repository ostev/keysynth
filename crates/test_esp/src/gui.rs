use core::{
    ops::Deref,
    sync::atomic::{self, AtomicBool},
};

use alloc::{borrow::Cow, format, string::String};
use embassy_executor::task;
use embassy_futures::select::{Either, select};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoTextStyleBuilder,
    pixelcolor::{Rgb565, RgbColor},
    primitives::Rectangle,
};
use embedded_gui::{
    app::{App, Change, State},
    component::{any_component, background::Background, button::Button, group::Group},
    interactive::FocusState,
    layout::{Direction, Sizing},
    primitive::{any_primitive, owned_text::OwnedText, text::Text},
    signal::{Reactive, Signal, SignalRef, Source},
};
use esp_println::println;
use esp_storage::FlashStorageError;
use esp_sync::RawMutex;
use keyboard_protocol::{Key, KeyboardDiff, KeyboardStatus, Modifier, StandardKey::P};
use st7789v2::{DriverResult, St7789v2};

mod background;
pub mod colors;
pub mod display;
mod editor;
mod effect;
pub mod event;
mod home;
mod message;
mod select_file;
mod text_bar;

pub use effect::current_panel;

use crate::{
    concurrency::receive_all,
    gui::{
        display::DisplayHardware,
        editor::line::{self, LineEditor},
        effect::Effect,
        home::{
            Home,
            dial::{self, Dial},
        },
        message::Message,
        select_file::file_list::{FileEntry, FileList},
        text_bar::TextBar,
    },
    input::{
        self,
        encoder::Panel,
        event::{Event, KeyEvent},
    },
    storage::{LoadError, Storage, StorageHardware},
    text::{NAME_SIZE, Name},
};

mod input_channel {
    use crate::{channel, input};

    channel!(input::event::Event);
}

pub use input_channel::sender as input_sender;

// mod synth_paremeters_channel {
//     use crate::channel;

//     channel!(synth::Parameters);
// }

// pub use synth_paremeters_channel::sender as synth_parameters_sender;

static SYNTH_PARAMETERS: embassy_sync::signal::Signal<RawMutex, synth::Parameters> =
    embassy_sync::signal::Signal::new();

pub fn set_synth_parameters(parameters: synth::Parameters) {
    SYNTH_PARAMETERS.signal(parameters);
}

static IS_CAPTURING_ALL_KEYBOARD_INPUT: AtomicBool = AtomicBool::new(false);

fn is_capturing_all_keyboard_input() -> bool {
    IS_CAPTURING_ALL_KEYBOARD_INPUT.load(atomic::Ordering::Relaxed)
}

pub fn is_capturing(event: &Event) -> bool {
    match event {
        Event::Key { keyboard, event } => {
            keyboard.is_super()
                || match event {
                    KeyEvent::Pressed(key) => key.is_arrow(),
                    KeyEvent::Released(key) => key.is_arrow(),
                }
                || is_capturing_all_keyboard_input()
        }
        Event::Encoder { .. } => false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum FocusKey {
    Editor,
    SelectFile(select_file::FocusKey),
    Home(home::FocusKey),
}

impl From<home::FocusKey> for FocusKey {
    fn from(key: home::FocusKey) -> Self {
        FocusKey::Home(key)
    }
}

impl From<select_file::FocusKey> for FocusKey {
    fn from(key: select_file::FocusKey) -> Self {
        FocusKey::SelectFile(key)
    }
}

#[derive(Reactive)]
#[any_component(target = display::Driver, event = input::event::Event, msg = Msg, focus_key = FocusKey)]
enum AnyComponent<'a> {
    Button(Button<'a, display::Color, &'a str>),
    Group(Group),
    Editor(editor::Editor<'a>),
    TextBar(TextBar),
    Background(Background<display::Color>),
    FileEntry(FileEntry),
    FileList(FileList<'a>),
    DialControl(dial::Control),
    DialPanel(dial::Panel),
}

#[derive(Reactive)]
#[any_primitive(target = display::Driver)]
enum AnyPrimitive<'a> {
    EditorView(editor::View<'a>),
    Spacer(embedded_gui::primitive::spacer::Spacer),
    TextStr(Text<'a, display::Color, &'a str>),
    OwnedText(OwnedText<display::Color, { NAME_SIZE }>),
    LineEditor(LineEditor<'a, { NAME_SIZE }>),
    Dial(Dial),
}

enum Page {
    SelectFile(Source<select_file::SelectFile>),
    Editor,
    Home(Source<home::Home>),
}

// TODO: implement derive macro for enums
impl State for Page {
    fn mark_resolved(&mut self) {
        match self {
            Page::SelectFile(state) => state.mark_resolved(),
            Page::Editor => {}
            Page::Home(state) => state.mark_resolved(),
        }
    }
}

#[derive(Reactive, State)]
struct Gui {
    page: Source<Page>,

    synth_parameters: Source<synth::Parameters>,

    editor: Source<editor::State>,

    message: Source<Option<Message>>,
}

impl Gui {
    fn set_page(&mut self, page: Page) -> Change<Msg, FocusKey, Effect> {
        let (is_capturing, change) = match &page {
            Page::SelectFile(_) => (
                false,
                Change::new()
                    .with_effect(Effect::FetchFiles)
                    .with_focus_key(FocusKey::SelectFile(select_file::FocusKey::default())),
            ),
            Page::Editor => (true, Change::new().with_focus_key(FocusKey::Editor)),
            Page::Home(_) => (
                false,
                Change::new()
                    .with_effect(Effect::SetPanel(Panel::default()))
                    .with_focus_key(FocusKey::Home(home::FocusKey::default())),
            ),
        };

        self.page.set(page);

        IS_CAPTURING_ALL_KEYBOARD_INPUT.store(is_capturing, atomic::Ordering::Relaxed);

        change
    }
}

impl App for Gui {
    type Target = display::Driver;

    type Msg = Msg;

    type FocusKey = FocusKey;
    type Event = input::event::Event;
    type AnyComponent<'a> = AnyComponent<'a>;
    type AnyPrimitive<'a> = AnyPrimitive<'a>;
    type Effect = Effect;

    fn new() -> Self {
        Self {
            page: Source::new(Page::Home(Source::new(Home::default()))),
            message: Source::new(None),

            synth_parameters: Source::new(synth::Parameters::default()),
            editor: Source::new(editor::State::default()),
        }
    }

    fn initial_focus_key() -> Self::FocusKey {
        FocusKey::Editor
    }

    fn background_color() -> Rgb565 {
        Rgb565::BLACK
    }

    fn update(&mut self, msg: Self::Msg) -> Change<Msg, FocusKey, Effect> {
        match (&mut *self.page, msg) {
            (Page::Home(state), Msg::Home(home_msg)) => {
                return state.update(|home| home.update(home_msg));
            }
            (Page::Editor, Msg::Editor(editor_msg)) => {
                return self.editor.update(|editor| editor.update(editor_msg));
            }
            (Page::SelectFile(state), Msg::SelectFile(home_msg)) => {
                return state.update(|home| home.update(home_msg));
            }

            (_, Msg::LoadCompleted(result)) => match result {
                Ok(source) => {
                    self.editor
                        .update(|editor| *editor = editor::State::new(source));

                    return self.set_page(Page::Editor);
                }
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
            (_, Msg::DeleteCompleted(result)) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while deleting! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },
            (_, Msg::RenameCompleted(result)) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while renaming! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },

            (_, Msg::ChangeFocus(focus)) => {
                return Change::new().with_focus_key(focus);
            }

            (_, Msg::SetSynthParameters(parameters)) => {
                self.synth_parameters.set(parameters);
            }

            _ => {}
        }

        return Change::new();
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
        match &*self.page {
            // Page::Home(home) => home.view(v, self.synth_parameters.signal()),
            Page::Home(_) => {
                println!("Rendering home page");

                v.view(
                    Direction::Horizontal,
                    [v.primitive(
                        Sizing::Fill,
                        Text {
                            content: SignalRef::constant(&"Home page is under construction!"),
                            font_style: Signal::constant(
                                MonoTextStyleBuilder::new()
                                    .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
                                    .text_color(display::Color::WHITE)
                                    .build(),
                            ),
                        },
                    )],
                )
            }
            Page::SelectFile(state) => state.view(v),
            Page::Editor => {
                let editor = v.interactive(
                    FocusKey::Editor,
                    |event| editor::Msg::from_event(event).map(Msg::Editor),
                    |_| {
                        v.component(
                            Sizing::Fill,
                            editor::Editor::new(self.editor.signal_ref()),
                            [],
                        )
                    },
                );

                v.view(Direction::Vertical, [editor])
            }
        }
    }
}

enum Msg {
    Editor(editor::Msg),
    Home(home::Msg),
    SelectFile(select_file::Msg),

    SaveCompleted(Result<Name, LoadError>),
    DeleteCompleted(Result<(), LoadError>),
    RenameCompleted(Result<(), LoadError>),
    LoadCompleted(Result<editor::source::Source, LoadError>),

    SetSynthParameters(synth::Parameters),

    ChangeFocus(FocusKey),
}

impl From<editor::Msg> for Msg {
    fn from(editor_msg: editor::Msg) -> Msg {
        Msg::Editor(editor_msg)
    }
}
impl From<select_file::Msg> for Msg {
    fn from(editor_msg: select_file::Msg) -> Msg {
        Msg::SelectFile(editor_msg)
    }
}
impl From<home::Msg> for Msg {
    fn from(home_msg: home::Msg) -> Msg {
        Msg::Home(home_msg)
    }
}

#[task]
pub async fn app(display: DisplayHardware, storage: StorageHardware) {
    let mut gui = Gui::new();

    // display.driver.display_on().unwrap();
    // display.driver.set_brightness(0xff).unwrap();

    // let Ok(_) = display.driver.clear(Gui::background_color());
    // display.driver.full_flush().await;

    let mut driver = display::Driver::init(display, display::Orientation::Vertical).await;
    driver.clear_async(colors::ERROR).await;
    driver.clear_async(colors::LITERAL).await;
    driver.clear_async(colors::ERROR).await;
    println!("Initial clear complete!");

    // driver.clear(Gui::background_color());
    // driver.full_flush().await;

    let mut internal_state = embedded_gui::app::InternalState::new(Gui::initial_focus_key());

    let mut effect_context = effect::Context {
        storage: Storage::new(storage),
    };

    let Ok(_) = embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, true);
    driver.full_flush().await;

    let input_receiver = input_channel::receiver();

    loop {
        // match display.driver.full_flush().await {
        //     Ok(_) => {}
        //     Err(error) => {
        //         println!(
        //             "Warning: error when writing to display. Here's the error: {:?}",
        //             error
        //         )
        //     }
        // }

        let events = select(receive_all(&input_receiver), SYNTH_PARAMETERS.wait()).await;

        match events {
            Either::First(input_events) => {
                embedded_gui::app::dispatch(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    input_events,
                )
                .await;
            }
            Either::Second(synth_parameters) => {
                embedded_gui::app::dispatch_msg(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    Msg::SetSynthParameters(synth_parameters),
                )
                .await;
            }
        }
        println!("Rendering...");
        let Ok(_) = embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, false);
        println!("Render complete!");
        println!("Flushing display...");
        driver.clear_async(colors::ERROR).await;
        // driver.full_flush().await;
        println!("Flush complete!");
    }
}
