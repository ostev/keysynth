use alloc::borrow::ToOwned;
use embedded_graphics::mono_font::{
    MonoTextStyle, MonoTextStyleBuilder,
    ascii::{self, FONT_8X13_ITALIC, FONT_9X18_BOLD},
};
use embedded_gui::{
    app::Change,
    component::{Component, button::Button, group::Group},
    interactive::FocusState,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::{owned_text::OwnedText, spacer::Spacer, text::Text},
    signal::{Reactive, Signal, SignalRef, Source},
    size::Size,
    view::{View, Widget},
};
use esp_println::println;
use keyboard_protocol::{Key, StandardKey};
use synth::note::Note;

pub mod dial;

use crate::{
    audio::MAX_POLYPHONY,
    gui::{
        self, colors, display,
        editor::{
            self,
            line::{self, LineEditor},
        },
        effect::Effect,
        event,
        select_file::file_list::{FileList, ScrollDirection},
        text_bar::{OwnedTextBar, TextBar},
    },
    input::{encoder, event::Event},
    storage::{self, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name, fixed_str},
};

pub const PREVIEW_LINE_LENGTH: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusKey {
    Panel(encoder::Panel),
}

impl Default for FocusKey {
    fn default() -> Self {
        FocusKey::Panel(encoder::Panel::CutoffResonance)
    }
}

pub enum Msg {
    SetPanel(encoder::Panel),
}

#[derive(Reactive, embedded_gui::app::State)]
pub struct Home {
    a: Source<f32>,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            a: Source::new(0.0),
        }
    }
}

impl Home {
    pub fn update(&mut self, msg: Msg) -> Change<gui::Msg, gui::FocusKey, Effect> {
        match msg {
            Msg::SetPanel(panel) => Change::new()
                .with_focus_key(FocusKey::Panel(panel))
                .with_effect(Effect::SetPanel(panel)),
        }
    }
}

impl Home {
    pub fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
        editor: SignalRef<'a, Option<editor::State>>,
        synth_parameters: Signal<synth::Parameters>,
        new_note: Signal<Option<Note>>,
    ) -> View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        const BAR_HEIGHT: u16 = 10;

        let style = Signal::constant(MonoTextStyle::new(&FONT_9X18_BOLD, colors::TEXT));

        v.view(
            Direction::Vertical,
            [v.group_fill(
                Direction::Horizontal,
                [
                    v.primitive(
                        Sizing::Intrinsic,
                        Spacer {
                            size: Signal::constant(Size::new(40, 0)),
                        },
                    ),
                    v.group(
                        Direction::Vertical,
                        Sizing::Constrained(120),
                        [
                            // Cutoff & resonance
                            v.interactive(
                                FocusKey::Panel(encoder::Panel::CutoffResonance),
                                event::on_keydown(
                                    |key| match key {
                                        Key::Standard(StandardKey::Down) => {
                                            Some(Msg::SetPanel(encoder::Panel::AttackDecay))
                                        }
                                        _ => None,
                                    },
                                    |_| None,
                                ),
                                |focus_state| {
                                    panel_with_background(
                                        v,
                                        focus_state,
                                        synth_parameters.map(|params| params.cutoff),
                                        synth_parameters.map(|params| params.resonance),
                                        SignalRef::constant(&"Cut"),
                                        SignalRef::constant(&"Res"),
                                    )
                                },
                            ),
                            // // Attack & decay
                            v.interactive(
                                FocusKey::Panel(encoder::Panel::AttackDecay),
                                event::on_keydown(
                                    |key| match key {
                                        Key::Standard(StandardKey::Down) => {
                                            Some(Msg::SetPanel(encoder::Panel::SustainRelease))
                                        }
                                        Key::Standard(StandardKey::Up) => {
                                            Some(Msg::SetPanel(encoder::Panel::CutoffResonance))
                                        }

                                        _ => None,
                                    },
                                    |_| None,
                                ),
                                |focus_state| {
                                    panel_with_background(
                                        v,
                                        focus_state,
                                        synth_parameters.map(|params| params.envelope.attack),
                                        synth_parameters.map(|params| params.envelope.decay),
                                        SignalRef::constant(&"Att"),
                                        SignalRef::constant(&"Dec"),
                                    )
                                },
                            ),
                            // Sustain & release
                            v.interactive(
                                FocusKey::Panel(encoder::Panel::SustainRelease),
                                event::on_keydown(
                                    |key| match key {
                                        Key::Standard(StandardKey::Up) => {
                                            Some(Msg::SetPanel(encoder::Panel::AttackDecay))
                                        }
                                        _ => None,
                                    },
                                    |_| None,
                                ),
                                |focus_state| {
                                    panel_with_background(
                                        v,
                                        focus_state,
                                        synth_parameters.map(|params| params.envelope.sustain),
                                        synth_parameters.map(|params| {
                                            params.envelope.release / synth::MAX_RELEASE
                                        }),
                                        SignalRef::constant(&"Sus"),
                                        SignalRef::constant(&"Rel"),
                                    )
                                },
                            ),
                        ],
                    ),
                    v.spacer(),
                    v.group(
                        Direction::Vertical,
                        Sizing::Constrained(90),
                        [
                            v.centered(
                                Direction::Horizontal,
                                v.primitive(
                                    Sizing::Intrinsic,
                                    OwnedText {
                                        content: new_note.map(|note| {
                                            note.map(|note| note.to_name())
                                                .flatten()
                                                .unwrap_or(heapless::format!("<  >").unwrap())
                                        }),
                                        font_style: style,
                                    },
                                ),
                            ),
                            v.centered(
                                Direction::Horizontal,
                                v.primitive(
                                    Sizing::Intrinsic,
                                    Text {
                                        content: SignalRef::constant(&"Program"),
                                        font_style: style,
                                    },
                                ),
                            ),
                            {
                                let preview_text = editor.map(|option_editor| {
                                    option_editor
                                        .as_ref()
                                        .map(|editor| editor.get_preview::<PREVIEW_LINE_LENGTH>())
                                        .unwrap_or(heapless::format!("no program loaded").unwrap())
                                });
                                println!("Preview text: {:?}", preview_text);

                                v.centered(
                                    Direction::Horizontal,
                                    v.primitive(
                                        Sizing::Intrinsic,
                                        OwnedText {
                                            content: preview_text,
                                            font_style: style,
                                        },
                                    ),
                                )
                            },
                            // Gain
                            v.component(
                                Sizing::Constrained(80),
                                dial::Control {
                                    color: Signal::constant(colors::TEXT),
                                    info: dial::ControlInfo {
                                        progress: synth_parameters.map(|params| params.voice_gain),
                                        label: SignalRef::constant(&"Gain"),
                                    },
                                },
                                [],
                            ),
                            v.spacer(),
                        ],
                    ),
                    // v.spacer(),
                ],
            )],
        )
    }
}

fn panel_with_background<'a>(
    v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
    focus_state: Signal<FocusState>,
    param_1: Signal<f32>,
    param_2: Signal<f32>,
    param_1_label: SignalRef<'static, &'static str>,
    param_2_label: SignalRef<'static, &'static str>,
) -> Widget<
    'a,
    display::Driver,
    Event,
    gui::Msg,
    gui::FocusKey,
    gui::AnyComponent<'a>,
    gui::AnyPrimitive<'a>,
> {
    let dial_color = focus_state.map(|focus_state| match focus_state {
        FocusState::Focused => colors::PURPLE,
        FocusState::Unfocused => colors::TEXT,
    });

    v.component(
        Sizing::Constrained(80),
        dial::Panel {
            info_1: dial::ControlInfo {
                progress: param_1,
                label: param_1_label,
            },
            info_2: dial::ControlInfo {
                progress: param_2,
                label: param_2_label,
            },
            color: dial_color,
        },
        [],
    )
}
