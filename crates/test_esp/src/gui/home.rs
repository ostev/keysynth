use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii};
use embedded_gui::{
    app::Change,
    component::{Component, button::Button, group::Group},
    interactive::FocusState,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::text::Text,
    signal::{Reactive, Signal, SignalRef, Source},
    view::{View, Widget},
};
use keyboard_protocol::{Key, StandardKey};

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
        text_bar::TextBar,
    },
    input::{encoder, event::Event},
    storage::{self, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name, fixed_str},
};

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
        synth_parameters: Signal<synth::Parameters>,
    ) -> View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        const BAR_HEIGHT: u16 = 40;

        v.view(
            Direction::Vertical,
            [v.group(
                Direction::Horizontal,
                [
                    // Cutoff & resonance
                    v.interactive(
                        FocusKey::Panel(encoder::Panel::CutoffResonance),
                        event::on_keydown(
                            |key| match key {
                                Key::Standard(StandardKey::Right) => {
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
                                SignalRef::constant(&"Cutoff"),
                                SignalRef::constant(&"Resonance"),
                            )
                        },
                    ), // Attack & decay
                       // v.interactive(
                       //     FocusKey::Panel(encoder::Panel::AttackDecay),
                       //     event::on_keydown(
                       //         |key| match key {
                       //             Key::Standard(StandardKey::Right) => {
                       //                 Some(Msg::SetPanel(encoder::Panel::AttackDecay))
                       //             }
                       //             _ => None,
                       //         },
                       //         |_| None,
                       //     ),
                       //     |focus_state| {
                       //         panel_with_background(
                       //             v,
                       //             focus_state,
                       //             synth_parameters.map(|params| params.envelope.attack),
                       //             synth_parameters.map(|params| params.envelope.decay),
                       //             SignalRef::constant(&"Attack"),
                       //             SignalRef::constant(&"Decay"),
                       //         )
                       //     },
                       // ),
                       // Sustain & release
                       // v.interactive(
                       //     FocusKey::Panel(encoder::Panel::SustainRelease),
                       //     event::on_keydown(
                       //         |key| match key {
                       //             Key::Standard(StandardKey::Right) => {
                       //                 Some(Msg::SetPanel(encoder::Panel::SustainRelease))
                       //             }
                       //             _ => None,
                       //         },
                       //         |_| None,
                       //     ),
                       //     |focus_state| {
                       //         panel_with_background(
                       //             v,
                       //             focus_state,
                       //             synth_parameters.map(|params| params.envelope.sustain),
                       //             synth_parameters.map(|params| params.envelope.release),
                       //             SignalRef::constant(&"Sustain"),
                       //             SignalRef::constant(&"Release"),
                       //         )
                       //     },
                       // ),
                ],
            )],
        )
    }
}

fn panel_with_background<'a>(
    v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
    focus_state: Option<FocusState>,
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
    let dial_color = Signal::constant(colors::PURPLE);

    let background = if focus_state.is_some() {
        colors::BACKGROUND_LIGHT
    } else {
        colors::BACKGROUND_DARK
    };

    v.background(
        Sizing::Fill,
        Signal::constant(background),
        [v.component(
            Sizing::Fill,
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
        )],
    )
}
