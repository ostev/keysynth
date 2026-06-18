use core::f32;

use embedded_graphics::{
    geometry::{Angle, Point},
    mono_font::{MonoTextStyle, ascii},
    primitives::{Circle, PrimitiveStyle, Sector, StyledDrawable},
};
use embedded_gui::{
    component::Component,
    layout::{Direction, IntrinsicSize, Sizing},
    primitive::{Primitive, text::Text},
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
const OUTLINE_THICKNESS: u32 = 2;
const STEPS: f32 = 32.0;

impl IntrinsicSize for Dial {
    fn intrinsic_size(&self) -> Size {
        Size::new(RADIUS * 2, RADIUS * 2)
    }
}

impl Primitive<display::Driver> for Dial {
    fn draw(
        &self,
        target: &mut embedded_gui::draw::LocalTarget<display::Driver>,
    ) -> Result<(), <display::Driver as embedded_graphics::prelude::DrawTarget>::Error> {
        let outline_style = PrimitiveStyle::with_stroke(*self.color, OUTLINE_THICKNESS);
        let fill_style = PrimitiveStyle::with_fill(*self.color);

        let outline = Circle::new(Point::zero(), RADIUS as u32 * 2);

        // Draw the outline ring
        outline.draw_styled(&outline_style, target)?;

        // Draw the fill sector
        Sector::from_circle(
            outline,
            Angle::zero(),
            Angle::from_radians(*self.progress * 2.0 * f32::consts::PI),
        )
        .draw_styled(&fill_style, target)?;

        Ok(())
    }
}

pub fn increase(value: f32, steps: usize) -> f32 {
    (value + (1.0 / STEPS) * steps as f32).clamp(0.0, 1.0)
}

pub fn decrease(value: f32, steps: usize) -> f32 {
    (value - (1.0 / STEPS) * steps as f32).clamp(0.0, 1.0)
}

#[derive(Reactive)]
pub struct Control<'a> {
    pub progress: Signal<f32>,
    pub color: Signal<display::Color>,
    pub label: SignalRef<'a, &'static str>,

    pub on_increment: Signal<home::Msg>,
    pub on_decrement: Signal<home::Msg>,
    pub focus_key: Signal<home::FocusKey>,
}

impl<'a> IntrinsicSize for Control<'a> {
    fn intrinsic_size(&self) -> Size {
        todo!()
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
    > for Control<'a>
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
            [v.interactive(
                *self.focus_key,
                event::on_keydown::<home::Msg>(|key| None, |_| None),
                |_| {
                    v.group(
                        Direction::Vertical,
                        [
                            v.primitive(
                                Sizing::Intrinsic,
                                Dial {
                                    progress: self.progress.clone(),
                                    color: self.color.clone(),
                                },
                            ),
                            // Label centered below the dial
                            v.centered(
                                Direction::Horizontal,
                                v.primitive(
                                    Sizing::Intrinsic,
                                    Text {
                                        content: self.label.clone(),
                                        font_style: Signal::constant(MonoTextStyle::new(
                                            &ascii::FONT_10X20,
                                            colors::TEXT,
                                        )),
                                    },
                                ),
                            ),
                        ],
                    )
                },
            )],
        )
    }
}
