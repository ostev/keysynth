use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoFont, MonoTextStyle},
    pixelcolor::Rgb565,
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
};

use crate::gui::colors;

pub fn draw_cursor<T: DrawTarget<Color = Rgb565>>(
    font: &MonoFont,
    position: embedded_graphics::geometry::Point,
    target: &mut T,
) -> Result<(), T::Error> {
    RoundedRectangle::new(
        Rectangle::new(
            position,
            embedded_graphics::geometry::Size::new(4, font.character_size.height),
        ),
        CornerRadii::new(embedded_graphics::geometry::Size::new(1, 1)),
    )
    .draw_styled(
        &PrimitiveStyleBuilder::new()
            .fill_color(colors::TEXT)
            .build(),
        target,
    )
}
