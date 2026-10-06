# KeySynth

KeySynth is an equation synthesiser built around a traditional mechanical keyboard!

An equation synthesiser is a wavetable synthesiser that allows the user to input arbitrary mathematical expressions which can then be used to create sounds! It also features a basic low-pass filter, with more effects (hopefully) to come.

In addition, it can act as a regular computer keyboard.

This repository contains the open-source PCBs (missing non-open source components from UltraLibrarian and SnapMagic) as well as project's source code. The PCB contains an 80% keyboard, three knobs, a pinout for a display, an ESP32-S3 (used for audio, display and USB) and an RP2040 (used for interfacing with the keyboard matrix due to limited pin availability).

The source code is divided into the following crates:

- [`calc`](crates/calc/) for the tiny programming language used for synthesis,
- [`repl`](crates/repl/) for a simple REPL for `calc` which is used for debugging and implementing new langauge features,
- [`synth`](crates/synth/) for the audio engine,
- [`keyboard_protocol`](crates/keyboard_protocol/) for interfacing between the RP2040 and the ESP32-S3 about keyboards and two of the three knobs over UART serial,
- [`keyboard_rp2040`](crates/keyboard_rp2040/) for running the keyboard matrix on the RP2040, and
- [`app`](crates/app/) for running the main application on the ESP32-S3.

## :construction: :construction: :construction: Roadmap

This project isn't finished! Equation synthesis and keyboard input all work, but I want to implement some more features before considering it complete:

- [ ] USB MIDI input from host
- [ ] Improve PCB layout (as-is there are a couple of design flaws that don't inhabit the current set of functionality but limit the utility of 3.5mm audio output. I'm not sure when I'll be able to fix this: it'll be when I've made enough changes that I can justify the cost of ordering another set of PCBs.)
- [ ] Add more audio effects, like reverb and chorus.
- [ ] Add more audio synthesis engines, like subtractive or FM synthesis.
