//! Contains information for UART communication between the
//! RP2040 and the ESP32-S3.

pub const BAUDRATE: u32 = 115_200;
pub const START_BYTE: u8 = 0xff;
pub const END_BYTE: u8 = 0x00;
