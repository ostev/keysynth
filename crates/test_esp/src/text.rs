use alloc::vec::Vec;
use embedded_graphics::mono_font::MonoFont;

pub type ByteChar = u8;
pub type ByteString = Vec<ByteChar>;
pub type ByteStr = [u8];

pub const fn width(font: &MonoFont, characters: u32) -> u32 {
    font.character_size.width * characters + font.character_spacing * (characters.saturating_sub(1))
}

pub const fn width_with_space_after(font: &MonoFont, characters: u32) -> u32 {
    (font.character_size.width + font.character_spacing) * characters
}

pub const fn characters_in(font: &MonoFont, width: u32) -> u32 {
    width / (font.character_size.width + font.character_spacing)
}
