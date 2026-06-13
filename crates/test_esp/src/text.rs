use core::str::Utf8Error;

use alloc::vec::Vec;
use embedded_graphics::mono_font::MonoFont;
use heapless::format;

pub type ByteChar = u8;
pub type ByteString = Vec<ByteChar>;
pub type FixedByteString<const N: usize> = heapless::Vec<ByteChar, N>;
pub type ByteStr = [u8];

pub type Name = FixedByteString<NAME_SIZE>;
pub const NAME_SIZE: usize = 14;

pub fn fixed_str<const N: usize>(text: &'static str) -> FixedByteString<N> {
    FixedByteString::from_slice(text.as_bytes()).unwrap()
}

pub fn fixed_str_to_str<const N: usize>(text: &FixedByteString<N>) -> Result<&str, Utf8Error> {
    str::from_utf8(text.as_slice())
}

pub fn name_to_string(name: Name) -> heapless::String<NAME_SIZE> {
    heapless::String::from_utf8(name).unwrap_or(format!("<corrupted>").unwrap())
}

pub const fn width(font: &MonoFont, characters: u32) -> u32 {
    font.character_size.width * characters + font.character_spacing * (characters.saturating_sub(1))
}

pub const fn width_with_space_after(font: &MonoFont, characters: u32) -> u32 {
    (font.character_size.width + font.character_spacing) * characters
}

pub const fn characters_in(font: &MonoFont, width: u32) -> u32 {
    width / (font.character_size.width + font.character_spacing)
}
