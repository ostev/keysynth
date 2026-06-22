use core::sync::atomic::{self, AtomicBool};

use alloc::{borrow::Cow, format, string::String};
use bumpalo::Bump;
use embassy_executor::task;
use embassy_futures::select::{Either3, select3};
use embassy_time::{Duration, Instant};
use embedded_graphics::{draw_target::DrawTarget, pixelcolor::Rgb565};
use embedded_gui::{
    app::{App, Change, State},
    component::{any_component, background::Background, button::Button, group::Group},
    layout::{Direction, Sizing},
    primitive::{any_primitive, owned_text::OwnedText, text::Text},
    signal::{Reactive, Source},
};
use esp_println::println;
use esp_sync::RawMutex;
use keyboard_protocol::{Key, StandardKey};

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
        editor::{MAX_SIZE, line::LineEditor},
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

/// Receives input events.
mod input_channel {
    use crate::{channel, input};

    channel!(input::event::Event);
}

pub use input_channel::sender as input_sender;

/// Receives updated synth parameters.
static SYNTH_PARAMETERS: embassy_sync::signal::Signal<RawMutex, synth::Parameters> =
    embassy_sync::signal::Signal::new();

/// Set the new synth parameters to be shown in the GUI.
pub fn set_synth_parameters(parameters: synth::Parameters) {
    SYNTH_PARAMETERS.signal(parameters);
}

/// Receives a new note when it's first pressed.
static NEW_NOTE: embassy_sync::signal::Signal<RawMutex, Note> = embassy_sync::signal::Signal::new();

/// Set the note that was just pressed to be displayed in the GUI.
pub fn set_new_note(note: Note) {
    NEW_NOTE.signal(note);
}

/// Represents whether the GUI is currently capturing all keyboard input. This is updated when the page changes.
static IS_CAPTURING_ALL_KEYBOARD_INPUT: AtomicBool = AtomicBool::new(false);

/// Represents whether the GUI is currently capturing all keyboard input.
fn is_capturing_all_keyboard_input() -> bool {
    IS_CAPTURING_ALL_KEYBOARD_INPUT.load(atomic::Ordering::Relaxed)
}

/// Will the GUI capture this event?
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

/// Represents what is actively focused in the GUI.
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

/// Represents all the components in the app.
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

/// Represents all the primitives in the app.
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
    Message(message::View<'a>),
    LineEditor(LineEditor<'a, { NAME_SIZE }>),
    Dial(Dial),
}

/// The currently active page of the GUI.
enum Page {
    SelectFile(select_file::SelectFile),
    Editor,
    Home(home::Home),
}

impl Page {
    /// Change the current page, running any relevant effects.
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

    /// The flash message text to display (if there is any)
    message: Source<Option<Message>>,
    /// The scroll position of the message text
    message_scroll: Source<u16>,
}

impl Gui {
    /// Update the flash message and reset its scroll position.
    fn set_message(&mut self, message: Option<Message>) {
        self.message_scroll.set(0);
        self.message.set(message);
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
            page: Source::new(Page::Home(Home::default())),
            message: Source::new(None),
            message_scroll: Source::new(0),

            synth_parameters: Source::new(synth::Parameters::default()),
            new_note: Source::new(None),
            editor: Source::new(None),
        }
    }

    fn initial_focus_key() -> Self::FocusKey {
        FocusKey::Home(home::FocusKey::default())
    }

    fn background_color() -> Rgb565 {
        colors::BACKGROUND_DARK
    }

    fn global_event_handler(&self, event: Event) -> Option<Msg> {
        event::on_keydown(
            |key| match key {
                // If a flash message is displayed, scroll it on up/down arrow keys
                // and clear it on any other key.
                Key::Standard(StandardKey::Down) => {
                    if self.message.is_some() {
                        Some(Msg::ScrollMessage(1))
                    } else {
                        None
                    }
                }
                Key::Standard(StandardKey::Up) => {
                    if self.message.is_some() {
                        Some(Msg::ScrollMessage(-1))
                    } else {
                        None
                    }
                }
                _ => {
                    if self.message.is_some() {
                        Some(Msg::ClearMessage)
                    } else {
                        None
                    }
                }
            },
            // Global keyboard shortcuts:
            |key| match key {
                Key::Standard(StandardKey::E) => match *self.page {
                    Page::Editor => Some(Msg::ChangePage(Page::Home(home::Home::default()))),
                    _ => self.editor.as_ref().map(|_| Msg::ChangePage(Page::Editor)),
                },
                Key::Standard(StandardKey::H) => {
                    Some(Msg::ChangePage(Page::Home(home::Home::default())))
                }

                Key::Standard(StandardKey::O) => {
                    Some(Msg::ChangePage(Page::SelectFile(SelectFile::default())))
                }
                Key::Standard(StandardKey::S) => Some(Msg::SaveFile),
                Key::Standard(StandardKey::B) => Some(Msg::BuildWavetable),

                _ => None,
            },
        )(event)
    }

    fn update(&mut self, msg: Self::Msg) -> Change<Msg, FocusKey, Effect> {
        // Clear the newly-pressed note if it's been displayed for more than
        // a certain amount of time.
        const NEW_NOTE_DISPLAY_DURATION: Duration = Duration::from_secs(2);

        if let Some((_, start)) = *self.new_note {
            if start.elapsed() > NEW_NOTE_DISPLAY_DURATION {
                self.new_note.set(None);
            }
        }

        match msg {
            // Msg directed toward a subpage
            Msg::Page(page_msg) => {
                return self.page.update(|page| match (page, page_msg) {
                    (Page::Home(state), PageMsg::Home(home_msg)) => state.update(home_msg),
                    (Page::Editor, PageMsg::Editor(editor_msg)) => self.editor.update(|editor| {
                        editor
                            .as_mut()
                            .map(|editor| editor.update(editor_msg))
                            .unwrap_or(Change::new())
                    }),
                    (Page::SelectFile(select_file), PageMsg::SelectFile(select_file_msg)) => {
                        select_file.update(select_file_msg)
                    }

                    _ => Change::new(),
                });
            }

            Msg::ScrollMessage(delta) => {
                self.message_scroll
                    .set_with(|scroll| (*scroll as i16 + delta as i16).try_into().unwrap_or(0));
            }
            Msg::ClearMessage => {
                self.set_message(None);
            }

            // A file has loaded
            Msg::LoadCompleted(result) => match result {
                Ok(source) => {
                    self.editor
                        .update(|editor| *editor = Some(editor::State::new(source)));

                    return self.page.update(|page| page.change(Page::Editor));
                }
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while loading from flash! Please try again. The error is: {:?}",
                        error
                    ));
                    self.set_message(Some(Message::now(text)));
                }
            },
            // File save finished
            Msg::SaveCompleted(result) => match result {
                Ok(name) => {
                    let name = str::from_utf8(&name);

                    match name {
                            Ok(name) => {
                                let message = Cow::Owned(format!(
                                    "Saved to {}!",
                                    name
                                ));
                                self.set_message(Some(Message::now(message)));
                            }
                            Err(_) => {
                                self.set_message(Some(Message::now(Cow::Borrowed(
                                    "The name of the file that was just saved to flash is invalid! Corruption has occured.",
                                ))))
                            }
                        }
                }
                Err(error) => {
                    println!("error saving to flash: {:?}", error);
                    let text = Cow::Owned(format!(
                        "An error occurred while saving to flash! Please try again. The error is: {:?}",
                        error
                    ));
                    self.set_message(Some(Message::now(text)));
                }
            },
            // File delete finished
            Msg::DeleteCompleted(result) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while deleting! Please try again. The error is: {:?}",
                        error
                    ));
                    self.set_message(Some(Message::now(text)));
                }
            },
            // File rename finished
            Msg::RenameCompleted(result) => match result {
                Ok(_) => return Change::new().with_effect(Effect::FetchFiles),
                Err(error) => {
                    let text = Cow::Owned(format!(
                        "An error occurred while renaming! Please try again. The error is: {:?}",
                        error
                    ));
                    self.set_message(Some(Message::now(text)));
                }
            },

            // Update the current focus
            Msg::ChangeFocus(focus) => {
                return Change::new().with_focus_key(focus);
            }
            // Update the current page
            Msg::ChangePage(new_page) => {
                return self.page.update(|page| page.change(new_page));
            }

            Msg::SetSynthParameters(parameters) => {
                self.synth_parameters.set(parameters);
            }
            Msg::SetNewNote(note) => {
                self.new_note.set(Some((note, Instant::now())));
            }
            Msg::WavetableBuilt(result) => match result {
                Ok(_) => {
                    self.message
                        .set(Some(Message::now(Cow::Borrowed(&"Wavetable built!"))));
                }
                Err(error_text) => {
                    self.set_message(Some(Message::now(Cow::Owned(error_text))));
                }
            },

            // Save the currently active file, if there is one.
            Msg::SaveFile => {
                if let Some(editor) = self.editor.as_ref() {
                    let mut buffer = [0; MAX_SIZE];

                    match editor.serialize(&mut buffer) {
                        Ok(_) => {
                            return Change::new()
                                .with_effect(Effect::Save(editor.name().clone(), buffer));
                        }
                        Err(_) => {
                            let text = Cow::Borrowed(
                                "A serialization error occurred while saving! Please try again.",
                            );

                            self.set_message(Some(Message::now(text)));
                        }
                    }
                }
            }

            // Build the currently active wavetable, if there is one.
            Msg::BuildWavetable => {
                if let Some(editor) = self.editor.as_ref() {
                    return Change::new().with_effect(Effect::BuildWavetable(
                        editor.name().clone(),
                        unsafe {
                            // SAFETY: the editor can only ever contain ASCII, so this is safe.
                            editor.to_string_unchecked()
                        },
                    ));
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
            // Show the flash message. This takes priority over everything else.
            v.view(
                Direction::Horizontal,
                [v.primitive(
                    Sizing::Intrinsic,
                    message::View {
                        message,
                        scroll: self.message_scroll.signal(),
                    },
                )],
            )
        } else {
            match &*self.page {
                // Home page
                Page::Home(home) => home.view(
                    v,
                    self.editor.signal_ref(),
                    self.synth_parameters.signal(),
                    self.new_note
                        .signal()
                        .map(|new_note| new_note.map(|(note, _)| note)),
                ),
                // Select file page
                Page::SelectFile(state) => state.view(v),
                // Text editor page
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

/// A msg directed toward a subpage.
enum PageMsg {
    Editor(editor::Msg),
    Home(home::Msg),
    SelectFile(select_file::Msg),
}

enum Msg {
    /// A message directed toward a sub-page
    Page(PageMsg),

    ClearMessage,
    ScrollMessage(i16),

    /// Save the current editor to flash
    SaveFile,

    /// Build the wavetable in the current editor
    BuildWavetable,
    /// The wavetable has been built
    WavetableBuilt(Result<(), String>),

    /// The editor has been saved to flash
    SaveCompleted(Result<Name, LoadError>),
    /// A file has been loaded from flash
    DeleteCompleted(Result<(), LoadError>),
    /// A file on flash has been renamed
    RenameCompleted(Result<(), LoadError>),
    /// A file has been loaded from flash
    LoadCompleted(Result<editor::source::Source, LoadError>),

    /// The synth parameters have changed, so we need to update the UI.
    SetSynthParameters(synth::Parameters),
    /// The newest note has changed, so we need to update the UI.
    SetNewNote(Note),

    /// Change the current focus to another element
    ChangeFocus(FocusKey),
    /// Change the current page
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

/// The GUI app task.
///
/// This should be run in a thread-mode executor, as otherwise rendering will block interrupts for a while.
#[task]
pub async fn app(display: DisplayHardware, storage: StorageHardware) {
    let mut gui = Gui::new();

    let mut driver = display::Driver::init(display, display::Orientation::Horizontal)
        .await
        .expect("Display driver failed to initialise.");

    driver.clear(Gui::background_color());
    driver
        .full_flush()
        .await
        .expect("Initial display clear failed.");

    let mut internal_state = embedded_gui::app::InternalState::new(Gui::initial_focus_key());

    let mut effect_context = effect::Context {
        storage: Storage::new(storage),
        ast_arena: Bump::new(),
    };

    let Ok(_) = embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, true);
    driver
        .full_flush()
        .await
        .expect("Initial display render failed.");

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
                // Dispatch these input events
                embedded_gui::app::dispatch(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    input_events,
                )
                .await;
            }
            Either3::Second(synth_parameters) => {
                // Update synth params
                embedded_gui::app::dispatch_msg(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    Msg::SetSynthParameters(synth_parameters),
                )
                .await;
            }
            Either3::Third(new_note) => {
                // Update new note
                embedded_gui::app::dispatch_msg(
                    &mut gui,
                    &mut effect_context,
                    &mut internal_state,
                    Msg::SetNewNote(new_note),
                )
                .await
            }
        }
        let page_has_changed = gui.page.has_changed() || gui.message.has_changed();

        if page_has_changed {
            driver.clear(Gui::background_color());
        }

        let Ok(_) =
            embedded_gui::app::render(&mut gui, &mut internal_state, &mut driver, page_has_changed);
        driver.full_flush().await.expect("Display flush failed.");
    }
}
