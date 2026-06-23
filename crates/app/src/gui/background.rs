use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    primitives::{CornerRadii, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StyledDrawable},
};

use crate::gui::colors;

/// Draw a background rounded reactangle
pub fn draw_background<T: DrawTarget<Color = Rgb565>>(
    rectangle: Rectangle,
    target: &mut T,
) -> Result<(), T::Error> {
    let rounded = RoundedRectangle::new(
        rectangle,
        CornerRadii::new(embedded_graphics::geometry::Size::new(4, 4)),
    );
    rounded.draw_styled(
        &PrimitiveStyleBuilder::new()
            .fill_color(colors::BACKGROUND_INTERACTIVE)
            .build(),
        target,
    )
}
