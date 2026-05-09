use crate::hardware::Hardware;

mod audio;
mod hardware;

pub struct App {
    audio: audio::Engine,
    hardware: Hardware,
}
