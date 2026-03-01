use cpal::{
    SizedSample,
    traits::{DeviceTrait, HostTrait},
};
use synth::wavetable::{DefaultWavetable, Oscillator, Wavetable};

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Please set a default audio output device");

    let config = device.default_output_config().unwrap();

    let table = DefaultWavetable::from_fn(f32::sin);
    let oscillator = Oscillator::new(&table, config.sample_rate() as f32);
}
