use alloc::borrow::Cow;
use embassy_time::Instant;
use embedded_graphics::{
    Drawable, geometry::Point, mono_font::MonoTextStyle, primitives::Rectangle,
};
use embedded_gui::{
    layout::IntrinsicSize,
    primitive::Primitive,
    signal::{Reactive, Signal, SignalRef},
    size::Size,
};
use embedded_text::{
    TextBox,
    alignment::HorizontalAlignment,
    style::{HeightMode, TextBoxStyleBuilder},
};

use crate::gui::{colors, display};

/// A full-screen flash message.
#[derive(Clone)]
pub struct Message {
    /// The message text.
    pub text: Cow<'static, str>,

    /// The time at which the message was created. Can be used for hiding it
    /// if it's been up for a while
    pub timestamp: Instant,
}

impl Message {
    /// Create a new message with the current timestamp.
    pub fn now(text: Cow<'static, str>) -> Message {
        Message {
            text,
            timestamp: Instant::now(),
        }
    }
}

/// Display a message as a full-screen splash.
#[derive(Reactive)]
pub struct View<'a> {
    /// The message to display.
    pub message: SignalRef<'a, Message>,
    /// The vertical scroll offset, measured in text lines.
    pub scroll: Signal<u16>,
}

impl<'a> IntrinsicSize for View<'a> {
    /// Return the size of the display.
    fn intrinsic_size(&self) -> Size {
        Size::new(display::WIDTH as u16, display::HEIGHT as u16)
    }
}

impl<'a> Primitive<display::Driver> for View<'a> {
    fn draw(
        &self,
        target: &mut embedded_gui::draw::LocalTarget<display::Driver>,
    ) -> Result<(), <display::Driver as embedded_graphics::prelude::DrawTarget>::Error> {
        const PADDING_X: u32 = 40;
        const PADDING_Y: u32 = 20;
        const SCROLL_INTERVAL: i32 = 20;

        let text = &self.message.text;
        let character_style =
            MonoTextStyle::new(&embedded_graphics_unicodefonts::MONO_10X20, colors::TEXT);
        let textbox_style = TextBoxStyleBuilder::new()
            .height_mode(HeightMode::Exact(
                embedded_text::style::VerticalOverdraw::Visible,
            ))
            .alignment(HorizontalAlignment::Left)
            .paragraph_spacing(4)
            .build();

        let bounds = Rectangle::new(
            Point::new(
                (PADDING_X / 2) as i32,
                (PADDING_Y / 2) as i32 - ((*self.scroll as i32) * SCROLL_INTERVAL),
            ),
            embedded_graphics::geometry::Size::new(
                display::WIDTH as u32 - PADDING_X,
                display::HEIGHT as u32 - PADDING_Y,
            ),
        );

        let text_box = TextBox::with_textbox_style(text, bounds, character_style, textbox_style);

        text_box.draw(target)?;

        Ok(())
    }
}
