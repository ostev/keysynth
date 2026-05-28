use std::{
    sync::{Arc, Mutex},
    time,
};

use bumpalo::Bump;
use calc::interpreter::Value;
use codespan_reporting::{diagnostic::Diagnostic, files::SimpleFile, term};
use cpal::{
    Device, FromSample, SampleFormat, SizedSample, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use embedded_graphics::{
    Drawable,
    pixelcolor::BinaryColor,
    prelude::{Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle},
};
use rpds::{List, ht_map, list};
use synth::{
    Synth,
    keyboard::{Key, Keyboard},
    note::{self, Event, Note},
    wavetable::{self, DefaultWavetable, Wavetable},
};

use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
    sdl2::Keycode,
};

fn prompt_for_wavetable<const S: usize>() -> Option<Wavetable<S>> {
    let mut rl = rustyline::DefaultEditor::new().unwrap();

    let readline = rl.readline("> ");
    let ast_arena = Bump::new();

    match readline {
        Ok(input) => match calc::parser::parse(&ast_arena, &input) {
            Ok(expr) => {
                let table = Wavetable::from_fn(|x| {
                    let value = {
                        let outer_scope = list![ht_map!["x" => Value::Number(x)]];

                        let (dynamic_value, _) =
                            calc::interpreter::eval(&expr, outer_scope).unwrap();

                        match dynamic_value {
                            Value::Number(value) => value,
                            _ => panic!("Didn't return a number!"),
                        }
                    };

                    value
                });

                Some(table)
            }
            Err(error) => {
                let diagnostic: Diagnostic<()> = error.into();
                let text = term::emit_into_string(
                    &term::Config::default(),
                    &SimpleFile::new("repl", &input),
                    &diagnostic,
                )
                .unwrap();
                println!("{}", text);
                None
            }
        },
        Err(_) => None,
    }
}

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Please set a default audio output device");

    let output_config = device.default_output_config().unwrap();
    let sample_format = output_config.sample_format();
    let sample_rate = output_config.sample_rate() as f32;
    let config: StreamConfig = output_config.into();

    // The audio callback requires 'static captures, so keep the wavetable alive for process lifetime.
    let table: DefaultWavetable = prompt_for_wavetable().unwrap();

    let mut display = SimulatorDisplay::<BinaryColor>::new(Size::new(128, 64));
    {
        let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

        let Ok(_) = Circle::new(Point::new(72, 8), 48)
            .into_styled(line_style)
            .draw(&mut display);
    }

    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledBlue)
        .build();

    let mut window = Window::new("Hello World", &output_settings);

    let keyboard = Keyboard::new((0, 0), (10, 4)).unwrap();
    let base = Note::C3;

    let synth: Arc<Mutex<Synth<5, { wavetable::DEFAULT_SIZE }>>> =
        Arc::new(Mutex::new(Synth::new(sample_rate, [table.clone(), table])));

    let stream = match sample_format {
        SampleFormat::F32 => {
            make_stream::<f32, 5, { wavetable::DEFAULT_SIZE }>(Arc::clone(&synth), &device, &config)
        }
        SampleFormat::I16 => {
            make_stream::<i16, 5, { wavetable::DEFAULT_SIZE }>(Arc::clone(&synth), &device, &config)
        }
        SampleFormat::U16 => {
            make_stream::<u16, 5, { wavetable::DEFAULT_SIZE }>(Arc::clone(&synth), &device, &config)
        }
        _ => panic!("Unsupported sample format: {sample_format:?}"),
    };

    let start = time::Instant::now();

    stream.play().unwrap();

    'running: loop {
        window.update(&display);
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => {
                    if !repeat {
                        if let Some(key) = keycode_to_key(keycode) {
                            if let Some(note) = keyboard.note_of(key, base) {
                                println!("Note: {}", note.frequency);
                                let human_note = pitchy::Note::try_from(pitchy::Pitch::new(
                                    note.frequency.into(),
                                ))
                                .unwrap();
                                println!("Note name: {}", human_note.name());
                                let _ = synth.lock().unwrap().note_on(Event {
                                    note,
                                    timestamp: start.elapsed().as_micros() as u64,
                                });
                            }
                        }
                    }
                }
                SimulatorEvent::KeyUp {
                    keycode, repeat, ..
                } => {
                    if !repeat {
                        if let Some(key) = keycode_to_key(keycode) {
                            if let Some(note) = keyboard.note_of(key, base) {
                                let _ = synth.lock().unwrap().note_off(note);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn make_stream<T, const N: usize, const S: usize>(
    synth: Arc<Mutex<Synth<N, S>>>,
    device: &Device,
    config: &StreamConfig,
) -> cpal::Stream
where
    T: SizedSample + FromSample<f32>,
{
    let num_channels = config.channels as usize;
    let synth_for_callback = synth.clone();

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create("output.wav", spec).unwrap();

    device
        .build_output_stream(
            config,
            move |output: &mut [T], _info| {
                for frame in output.chunks_mut(num_channels) {
                    let value = synth_for_callback.lock().unwrap().sample();
                    for sample in frame {
                        *sample = T::from_sample(value);
                    }

                    writer.write_sample(value).unwrap();
                }
            },
            |err| eprintln!("Error building output sound stream: {err}"),
            None,
        )
        .unwrap()
}

fn keycode_to_key(keycode: Keycode) -> Option<Key> {
    let (row, column) = match keycode {
        // Top row: numbers 1..0 -> columns 1..10
        Keycode::NUM_1 => (0, 0),
        Keycode::NUM_2 => (0, 1),
        Keycode::NUM_3 => (0, 2),
        Keycode::NUM_4 => (0, 3),
        Keycode::NUM_5 => (0, 4),
        Keycode::NUM_6 => (0, 5),
        Keycode::NUM_7 => (0, 6),
        Keycode::NUM_8 => (0, 7),
        Keycode::NUM_9 => (0, 8),
        Keycode::NUM_0 => (0, 9),

        // QWERTY row -> columns 1..10
        Keycode::Q => (1, 0),
        Keycode::W => (1, 1),
        Keycode::E => (1, 2),
        Keycode::R => (1, 3),
        Keycode::T => (1, 4),
        Keycode::Y => (1, 5),
        Keycode::U => (1, 6),
        Keycode::I => (1, 7),
        Keycode::O => (1, 8),
        Keycode::P => (1, 9),

        // ASDF row -> columns 1..10
        Keycode::A => (2, 0),
        Keycode::S => (2, 1),
        Keycode::D => (2, 2),
        Keycode::F => (2, 3),
        Keycode::G => (2, 4),
        Keycode::H => (2, 5),
        Keycode::J => (2, 6),
        Keycode::K => (2, 7),
        Keycode::L => (2, 8),
        Keycode::Semicolon => (2, 9),

        // ZXCV row -> columns 1..10
        Keycode::Z => (3, 0),
        Keycode::X => (3, 1),
        Keycode::C => (3, 2),
        Keycode::V => (3, 3),
        Keycode::B => (3, 4),
        Keycode::N => (3, 5),
        Keycode::M => (3, 6),
        Keycode::Comma => (3, 7),
        Keycode::Period => (3, 8),
        Keycode::Slash => (3, 9),

        // Ignore any other keys
        _ => return None,
    };

    Some(Key::new(row, column))
}
