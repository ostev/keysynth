use core::f32;

use embedded_graphics::{
    geometry::{Angle, Point},
    primitives::{Circle, PrimitiveStyle, Sector, StyledDrawable},
};
use embedded_gui::{
    layout::IntrinsicSize,
    primitive::Primitive,
    signal::{Reactive, Signal},
    size::Size,
};

use crate::gui::display;

#[derive(Reactive)]
pub struct Dial {
    pub progress: Signal<f32>,
    pub color: Signal<display::Color>,
}

impl Dial {
    pub fn increase(value: f32) -> f32 {
        (value + (1.0 / STEPS)).clamp(0.0, 1.0)
    }

    pub fn decrease(value: f32) -> f32 {
        (value - (1.0 / STEPS)).clamp(0.0, 1.0)
    }
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
