use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii};
use embedded_gui::{
    app::Change,
    component::{Component, button::Button, group::Group},
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::text::Text,
    signal::{Reactive, Signal, SignalRef, Source},
    view::View,
};

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
        select_file::file_list::{FileList, ScrollDirection},
        text_bar::TextBar,
    },
    input::event::Event,
    storage::{self, LoadError, MAX_FILES},
    text::{NAME_SIZE, Name, fixed_str},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FocusKey {
    #[default]
    NewFile,

    FileList(usize),
}

pub enum Msg {}

#[derive(Reactive, embedded_gui::app::State)]
pub struct Home {
    cutoff: Source<f32>,
    resonance: Source<f32>,
    voice_gain: Source<f32>,

    attack: Source<f32>,
    decay: Source<f32>,
    sustain: Source<f32>,
    release: Source<f32>,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            cutoff: Source::new(0.8),
            resonance: Source::new(0.1),
            voice_gain: Source::new(1.0 / (MAX_POLYPHONY as f32)),
            attack: Source::new(0.3),
            decay: Source::new(0.3),
            sustain: Source::new(1.0),
            release: Source::new(0.9),
        }
    }
}

impl Home {
    pub fn update(&mut self, msg: Msg) -> Change<gui::Msg, gui::FocusKey, Effect> {
        match msg {}

        Change::new()
    }
}

impl Home {
    pub fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
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

        v.view(Direction::Vertical, [])
    }
}
