use core::f32::{self, MIN};

use embedded_graphics::{
    geometry::{Angle, Point},
    mono_font::{MonoFont, MonoTextStyle, ascii},
    primitives::{Circle, PrimitiveStyle, Sector, StyledDrawable},
};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::{Primitive, spacer::Spacer, text::Text},
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};

use crate::{
    gui::{self, colors, display, event, home},
    input::event::Event,
};

#[derive(Reactive)]
pub struct Dial {
    pub progress: Signal<f32>,
    pub color: Signal<display::Color>,
}

const RADIUS: u16 = 20;
const OUTLINE_THICKNESS: u32 = 3;

impl IntrinsicSize for Dial {
    fn intrinsic_size(&self) -> Size {
        Size::new(RADIUS * 2 + 4, RADIUS * 2 + 4)
    }
}

impl Primitive<display::Driver> for Dial {
    fn draw(
        &self,
        target: &mut embedded_gui::draw::LocalTarget<display::Driver>,
    ) -> Result<(), <display::Driver as embedded_graphics::prelude::DrawTarget>::Error> {
        let outline_style = PrimitiveStyle::with_stroke(*self.color, OUTLINE_THICKNESS);
        let fill_style = PrimitiveStyle::with_fill(*self.color);

        let outline = Circle::new(Point::new(0, 0), RADIUS as u32 * 2);

        const MIN_PROGRESS: f32 = 0.03;

        let angle = self.progress.clamp(MIN_PROGRESS, 1.0) * 2.0 * f32::consts::PI;

        // Draw the fill sector
        Sector::from_circle(
            outline,
            Angle::from_radians(-(f32::consts::PI / 2.0)),
            Angle::from_radians(angle),
        )
        .draw_styled(&fill_style, target)?;

        // Draw the outline ring
        outline.draw_styled(&outline_style, target)?;

        Ok(())
    }
}

// pub fn increase(value: f32, steps: usize) -> f32 {
//     (value + (1.0 / STEPS) * steps as f32).clamp(0.0, 1.0)
// }

// pub fn decrease(value: f32, steps: usize) -> f32 {
//     (value - (1.0 / STEPS) * steps as f32).clamp(0.0, 1.0)
// }

#[derive(Reactive, Clone)]
pub struct ControlInfo {
    pub progress: Signal<f32>,
    pub label: SignalRef<'static, &'static str>,
}

#[derive(Reactive)]
pub struct Control {
    pub color: Signal<display::Color>,

    pub info: ControlInfo,
}

const FONT: MonoFont = ascii::FONT_10X20;

impl IntrinsicSize for Control {
    fn intrinsic_size(&self) -> Size {
        // Size::new(
        //     RADIUS * 2,
        //     RADIUS * 2 + FONT.character_size.height as u16 + 40,
        // )
        panic!("Fill only")
    }
}

impl<'a>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for Control
{
    fn view(
        &self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
        _: embedded_gui::view::Children<
            'a,
            display::Driver,
            Event,
            gui::Msg,
            gui::FocusKey,
            gui::AnyComponent<'a>,
            gui::AnyPrimitive<'a>,
        >,
    ) -> embedded_gui::view::View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        v.view(
            Direction::Vertical,
            [
                v.primitive(
                    Sizing::Intrinsic,
                    Dial {
                        progress: self.info.progress,
                        color: self.color.clone(),
                    },
                ),
                // Label centered below the dial
                v.centered(
                    Direction::Horizontal,
                    v.primitive(
                        Sizing::Intrinsic,
                        Text {
                            content: self.info.label.clone(),
                            font_style: Signal::constant(MonoTextStyle::new(&FONT, colors::TEXT)),
                        },
                    ),
                ),
            ],
        )
    }
}

#[derive(Reactive)]
pub struct Panel {
    pub info_1: ControlInfo,
    pub info_2: ControlInfo,

    pub color: Signal<display::Color>,
}

const MARGIN: u16 = 20;

impl IntrinsicSize for Panel {
    fn intrinsic_size(&self) -> Size {
        // Size::new(
        //     RADIUS * 2 * 2 + MARGIN,
        //     RADIUS * 2 + FONT.character_size.height as u16 + 20,
        // )
        panic!("Fill only")
    }
}

impl<'a>
    Component<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > for Panel
{
    fn view(
        &self,
        v: &'a embedded_gui::view::Factory<Event, gui::Msg, gui::FocusKey>,
        _: embedded_gui::view::Children<
            'a,
            display::Driver,
            Event,
            gui::Msg,
            gui::FocusKey,
            gui::AnyComponent<'a>,
            gui::AnyPrimitive<'a>,
        >,
    ) -> embedded_gui::view::View<
        'a,
        display::Driver,
        Event,
        gui::Msg,
        gui::FocusKey,
        gui::AnyComponent<'a>,
        gui::AnyPrimitive<'a>,
    > {
        v.view(
            Direction::Horizontal,
            [
                v.component(
                    Sizing::Fill,
                    Control {
                        info: self.info_1.clone(),
                        color: self.color.clone(),
                    },
                    [],
                ),
                v.primitive(
                    Sizing::Intrinsic,
                    Spacer {
                        size: Signal::constant(Size::new(MARGIN, 0)),
                    },
                ),
                v.component(
                    Sizing::Fill,
                    Control {
                        info: self.info_2.clone(),
                        color: self.color.clone(),
                    },
                    [],
                ),
            ],
        )
    }
}
