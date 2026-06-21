use core::{
    ops::Deref,
    sync::atomic::{self, AtomicBool},
};

use alloc::{borrow::Cow, format, string::String};
use embassy_executor::task;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_time::{Duration, Instant};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::{Rgb565, RgbColor},
    primitives::Rectangle,
};
use embedded_gui::{
    app::{App, Change, State},
    component::{any_component, background::Background, button::Button, group::Group},
    draw::LocalTarget,
    interactive::FocusState,
    layout::{Direction, Sizing},
    position::Position,
    primitive::{any_primitive, owned_text::OwnedText, text::Text},
    signal::{Reactive, Signal, SignalRef, Source},
    size::Size,
};
use esp_println::println;
use esp_storage::FlashStorageError;
use esp_sync::RawMutex;
use heapless::sorted_linked_list::SortedLinkedList;
use keyboard_protocol::{
    Key, KeyboardDiff, KeyboardStatus, Modifier,
    StandardKey::{self, P},
};

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
use synth::note::Note;

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
        select_file::{
            SelectFile,
            file_list::{FileEntry, FileList},
        },
        text_bar::{OwnedTextBar, TextBar},
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

static SYNTH_PARAMETERS: embassy_sync::signal::Signal<RawMutex, synth::Parameters> =
    embassy_sync::signal::Signal::new();

pub fn set_synth_parameters(parameters: synth::Parameters) {
    SYNTH_PARAMETERS.signal(parameters);
}

static NEW_NOTE: embassy_sync::signal::Signal<RawMutex, Note> = embassy_sync::signal::Signal::new();

pub fn set_new_note(note: Note) {
    NEW_NOTE.signal(note);
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
    TextBar(TextBar<'a, &'a str>),
    OwnedTextBar4(OwnedTextBar<4>),
    OwnedTextBarName(OwnedTextBar<{ NAME_SIZE }>),
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
    TextCow(Text<'a, display::Color, Cow<'a, str>>),
    OwnedText4(OwnedText<display::Color, 4>),
    OwnedTextPreview(OwnedText<display::Color, { home::PREVIEW_LINE_LENGTH }>),
    OwnedTextName(OwnedText<display::Color, { NAME_SIZE }>),
    LineEditor(LineEditor<'a, { NAME_SIZE }>),
    Dial(Dial),
}

enum Page {
    SelectFile(select_file::SelectFile),
    Editor,
    Home(home::Home),
}

impl Page {
    fn change(&mut self, page: Page) -> Change<Msg, FocusKey, Effect> {
        let (is_capturing, change) = match &page {
            Page::SelectFile(_) => (
                true,
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

        *self = page;

        IS_CAPTURING_ALL_KEYBOARD_INPUT.store(is_capturing, atomic::Ordering::Relaxed);

        change
    }
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
    new_note: Source<Option<(Note, Instant)>>,

    editor: Source<Option<editor::State>>,

    message: Source<Option<Message>>,
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
            page: Source::new(Page::Home(Home::default())),
            // page: Source::new(Page::SelectFile(SelectFile::default())),
            message: Source::new(None),

            synth_parameters: Source::new(synth::Parameters::default()),
            new_note: Source::new(None),
            editor: Source::new(None),
        }
    }

    fn initial_focus_key() -> Self::FocusKey {
        FocusKey::Home(home::FocusKey::default())
    }

    fn background_color() -> Rgb565 {
        Rgb565::BLACK
    }

    fn default_event_handler(&self, event: Event) -> Option<Msg> {
        event::on_keydown(
            |_| None,
            |key| match key {
                Key::Standard(StandardKey::E) => {
                    self.editor.as_ref().map(|_| Msg::ChangePage(Page::Editor))
                }
                Key::Standard(StandardKey::O) => {
                    Some(Msg::ChangePage(Page::SelectFile(SelectFile::default())))
                }
                Key::Standard(StandardKey::S) => Some(Msg::SaveFile),
                _ => None,
            },
        )(event)
    }

    fn update(&mut self, msg: Self::Msg) -> Change<Msg, FocusKey, Effect> {
        const NEW_NOTE_DISPLAY_DURATION: Duration = Duration::from_secs(1);

        if let Some((_, start)) = *self.new_note {
            if start.elapsed() > NEW_NOTE_DISPLAY_DURATION {
                self.new_note.set(None);
            }
        }

        const MESSAGE_DISPLAY_DURATION: Duration = Duration::from_secs(2);

        let is_message_old = self
            .message
            .as_ref()
            .map(|message| message.timestamp.elapsed() > MESSAGE_DISPLAY_DURATION)
            .unwrap_or(false);

        if is_message_old {
            self.message.set(None);
        }

        match msg {
            Msg::Page(page_msg) => {
                return self.page.update(|page| {
                    match (page, page_msg) {
                        (Page::Home(state), PageMsg::Home(home_msg)) => {
                            return state.update(home_msg);
                        }
                        (Page::Editor, PageMsg::Editor(editor_msg)) => {
                            return self.editor.update(|editor| {
                                editor
                                    .as_mut()
                                    .map(|editor| editor.update(editor_msg))
                                    .unwrap_or(Change::new())
                            });
                        }
                        (Page::SelectFile(home), PageMsg::SelectFile(home_msg)) => {
                            home.update(home_msg);
                        }

                        _ => {}
                    }
                    Change::new()
                });
            }
            Msg::LoadCompleted(result) => match result {
                Ok(source) => {
                    self.editor
                        .update(|editor| *editor = Some(editor::State::new(source)));

                    return self.page.change(Page::Editor);
                }
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while loading from flash! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },
            Msg::SaveCompleted(result) => match result {
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
            Msg::DeleteCompleted(result) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while deleting! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },
            Msg::RenameCompleted(result) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while renaming! Please try again. The error is: {:?}",
                        error
                    ));
                    self.message.set(Some(Message::now(text)));
                }
            },

            Msg::ChangeFocus(focus) => {
                return Change::new().with_focus_key(focus);
            }
            Msg::ChangePage(new_page) => {
                return self.page.change(new_page);
            }

            Msg::SetSynthParameters(parameters) => {
                self.synth_parameters.set(parameters);
            }
            Msg::SetNewNote(note) => {
                self.new_note.set(Some((note, Instant::now())));
            }

            Msg::SaveFile => {
                if let Some(editor) = self.editor.as_ref() {
                    match editor.serialize() {
                        Ok(serialized) => {
                            return Change::new()
                                .with_effect(Effect::Save(editor.name().clone(), serialized));
                        }
                        Err(_) => {
                            let text = Cow::Borrowed(
                                "A serialization error occurred while saving! Please try again.",
                            );

                            self.message.set(Some(Message::now(text)));
                        }
                    }
                }
            }
        };

        Change::new()
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
        if let Some(message) = self.message.option_signal_ref() {
            v.view(
                Direction::Horizontal,
                [v.background(
                    Sizing::Fill,
                    Signal::constant(colors::BACKGROUND_LIGHT),
                    [v.middle(v.primitive(
                        Sizing::Fill,
                        Text {
                            content: message.map_ref(&v.bump, |message| message.text.clone()),
                            font_style: Signal::constant(MonoTextStyle::new(
                                &FONT_10X20,
                                colors::TEXT,
                            )),
                        },
                    ))],
                )],
            )
        } else {
            match &*self.page {
                Page::Home(home) => home.view(
                    v,
                    self.editor.signal_ref(),
                    self.synth_parameters.signal(),
                    self.new_note
                        .signal()
                        .map(|new_note| new_note.map(|(note, _)| note)),
                ),
                Page::SelectFile(state) => state.view(v),
                Page::Editor => {
                    let editor = v.interactive(
                        FocusKey::Editor,
                        |event| {
                            editor::Msg::from_event(event)
                                .map(|msg| Msg::Page(PageMsg::Editor(msg)))
                        },
                        |_| {
                            if let Some(editor) = self.editor.option_signal_ref() {
                                v.component(Sizing::Intrinsic, editor::Editor::new(editor), [])
                            } else {
                                v.spacer()
                            }
                        },
                    );

                    v.view(Direction::Vertical, [editor])
                }
            }
        }
    }
}

enum PageMsg {
    Editor(editor::Msg),
    Home(home::Msg),
    SelectFile(select_file::Msg),
}

enum Msg {
    Page(PageMsg),

    SaveFile,

    SaveCompleted(Result<Name, LoadError>),
    DeleteCompleted(Result<(), LoadError>),
    RenameCompleted(Result<(), LoadError>),
    LoadCompleted(Result<editor::source::Source, LoadError>),

    SetSynthParameters(synth::Parameters),
    SetNewNote(Note),

    ChangeFocus(FocusKey),
    ChangePage(Page),
}

impl From<editor::Msg> for Msg {
    fn from(editor_msg: editor::Msg) -> Msg {
        Msg::Page(PageMsg::Editor(editor_msg))
    }
}
impl From<select_file::Msg> for Msg {
    fn from(editor_msg: select_file::Msg) -> Msg {
        Msg::Page(PageMsg::SelectFile(editor_msg))
    }
}
impl From<home::Msg> for Msg {
    fn from(home_msg: home::Msg) -> Msg {
        Msg::Page(PageMsg::Home(home_msg))
    }
}

#[task]
pub async fn app(display: DisplayHardware, storage: StorageHardware) {
    let mut gui = Gui::new();

    // display.driver.display_on().unwrap();
    // display.driver.set_brightness(0xff).unwrap();

    // let Ok(_) = display.driver.clear(Gui::background_color());
    // display.driver.full_flush().await;

    let mut driver = display::Driver::init(display, display::Orientation::Horizontal).await;

    driver.clear(Gui::background_color());
    driver.full_flush().await;

    let mut internal_state = embedded_gui::app::InternalState::new(Gui::initial_focus_key());

    let mut effect_context = effect::Context {
        storage: Storage::new(storage),
    };

    let Ok(_) = embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, true);
    driver.full_flush().await;

    let input_receiver = input_channel::receiver();

    loop {
        let events = select3(
            receive_all(&input_receiver),
            SYNTH_PARAMETERS.wait(),
            NEW_NOTE.wait(),
        )
        .await;

        match events {
            Either3::First(input_events) => {
                embedded_gui::app::dispatch(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    input_events,
                )
                .await;
            }
            Either3::Second(synth_parameters) => {
                // println!("param update!!!");
                embedded_gui::app::dispatch_msg(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    Msg::SetSynthParameters(synth_parameters),
                )
                .await;
            }
            Either3::Third(new_note) => {
                embedded_gui::app::dispatch_msg(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    Msg::SetNewNote(new_note),
                )
                .await
            }
        }
        let page_has_changed = gui.page.has_changed();

        if page_has_changed {
            driver.clear(Gui::background_color());
        }

        let Ok(_) =
            embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, page_has_changed);
        driver.full_flush().await;
    }
}
