//! > The morning light is turning blue, the feeling is bizarre (Bizarre)
//! > The night is almost over, I still don't know where you are
//! > The shadows, yeah, they keep me pretty like a movie star
//! > Daylight makes me feel like Dracula (Dracula)...
//! > Run from the sun like Dracula!
//! --- "Dracula" by Tame Impala

use embedded_graphics::pixelcolor::Rgb565;

pub const ERROR: Rgb565 = to_rgb565(255, 85, 85);
pub const TEXT: Rgb565 = to_rgb565(248, 248, 242);
pub const LITERAL: Rgb565 = to_rgb565(255, 184, 108);
pub const SELECTION: Rgb565 = to_rgb565(68, 71, 90);

pub const BACKGROUND_INTERACTIVE: Rgb565 = to_rgb565(52, 55, 70);
pub const BACKGROUND_LIGHT: Rgb565 = to_rgb565(52, 55, 70);
pub const BACKGROUND_DARK: Rgb565 = to_rgb565(33, 34, 44);

pub const PURPLE: Rgb565 = to_rgb565(100, 74, 201);
pub const PINK: Rgb565 = to_rgb565(255, 121, 198);

pub const fn to_rgb565(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::new(
        ((r as u32 * 249 + 1014) >> 11) as u8,
        ((g as u32 * 253 + 505) >> 10) as u8,
        ((b as u32 * 249 + 1014) >> 11) as u8,
    )
}
