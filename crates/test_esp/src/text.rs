use alloc::vec::Vec;

pub type ByteChar = u8;
pub type ByteString = Vec<ByteChar>;
pub type ByteStr<'a> = &'a [u8];
