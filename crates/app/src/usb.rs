use embassy_sync::{blocking_mutex::Mutex, signal::Signal};
use esp_hal::{
    otg_fs::{self, Usb},
    peripherals::USB0,
};
use esp_sync::RawMutex;
use static_cell::StaticCell;
use synth::wavetable::Wavetable;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_audio::{AudioClass, AudioClassBuilder, StreamConfig, TerminalType};
use usbd_hid::{
    descriptor::{KeyboardReport, SerializedDescriptor},
    hid_class::{
        HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass, ProtocolModeConfig,
    },
};

use crate::{
    concurrency::try_receive_all,
    usb::audio::WAVETABLE_SIZE,
    usb::{audio::SAMPLE_RATE, hid::UsbKeyboardStatus},
};

pub mod audio;
pub mod hid;

pub type Bus = esp_hal::otg_fs::UsbBus<esp_hal::otg_fs::Usb<'static>>;

/// Hardware peripherals required to initialize the USB subsystem.
///
/// This owns the USB peripheral and its associated pins until
/// [`build`](Self::build) constructs the USB device.
pub struct Peripherals<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> {
    pub usb: USB0<'static>,
    pub dp: DP,
    pub dm: DM,
}

impl<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> Peripherals<DP, DM> {
    /// Initializes the USB bus and creates all USB classes.
    pub fn build(self) -> UsbHardware {
        static EP_OUT_BUFFER: StaticCell<[u32; 4096]> = StaticCell::new();

        let usb = Usb::new(self.usb, self.dp, self.dm);

        static USB_BUS: StaticCell<UsbBusAllocator<Bus>> = StaticCell::new();

        let bus: &'static mut _ = USB_BUS
            .init_with(|| esp_hal::otg_fs::UsbBus::new(usb, EP_OUT_BUFFER.init_with(|| [0; 4096])));

        let hid = HIDClass::new_with_settings(
            bus,
            KeyboardReport::desc(),
            hid::POLL_MS,
            HidClassSettings {
                subclass: HidSubClass::NoSubClass,
                protocol: HidProtocol::Keyboard,
                config: ProtocolModeConfig::DefaultBehavior,
                locale: HidCountryCode::US,
            },
        );

        let audio = AudioClassBuilder::new()
            .input(
                StreamConfig::new_discrete(
                    usbd_audio::Format::S16le,
                    1,
                    &[SAMPLE_RATE],
                    TerminalType::InMicrophone,
                )
                .unwrap(),
            )
            .build(bus)
            .unwrap();

        let device: UsbDevice<'static, Bus> = UsbDeviceBuilder::new(bus, UsbVidPid(0x1209, 0x0003))
            .max_power(500)
            .unwrap()
            .strings(&[StringDescriptors::default()
                .manufacturer("KeySynth")
                .product("KeySynth")
                .serial_number("000000001")])
            .unwrap()
            .composite_with_iads()
            .build();

        UsbHardware { device, audio, hid }
    }
}

/// Contains the USB device as well as all the device classes being used.
pub struct UsbHardware {
    device: UsbDevice<'static, Bus>,
    audio: AudioClass<'static, Bus>,
    hid: HIDClass<'static, Bus>,
}

mod audio_channel {
    use crate::{channel, usb::audio};

    pub const CAPACITY: usize = 16;

    channel!(audio::Event, CAPACITY);
}
pub use audio_channel::sender as audio_sender;

/// Most recent keyboard state.
static KEYBOARD_STATUS: Mutex<RawMutex, UsbKeyboardStatus> = Mutex::new(UsbKeyboardStatus::empty());

/// Updates the keyboard status, which will then be sent over USB the next time we're polled by the
/// host.
///
/// ## Safety
/// This function **must** not be called from inside another locked mutex.
pub unsafe fn set_keyboard_status(status: UsbKeyboardStatus) {
    unsafe {
        KEYBOARD_STATUS.lock_mut(|locked_status| {
            *locked_status = status;
        })
    }
}

/// Contains the new wavetable to be received by the audio engine.
static WAVETABLE: Signal<RawMutex, Wavetable<{ WAVETABLE_SIZE }>> = Signal::new();

/// Queues a new wavetable for the audio engine.
pub fn set_wavetable(wavetable: Wavetable<WAVETABLE_SIZE>) {
    WAVETABLE.signal(wavetable);
}

/// Stack size allocated for the USB task.
pub const STACK_SIZE: usize = 48_000;

/// Main USB device task.
///
/// This task is responsible for:
/// - transmitting HID keyboard reports,
/// - receiving MIDI messages,
/// - streaming synthesized audio,
/// - applying audio events and wavetable updates.
pub fn device_loop(mut hardware: UsbHardware) -> ! {
    let mut audio = audio::State::new();

    let audio_receiver = audio_channel::receiver();

    let mut sample = audio.sample();

    loop {
        // Apply the next 4 events
        let audio_events = try_receive_all(&audio_receiver).take(4);
        audio.apply_events(audio_events);

        // Update the wavetable if there's a new one
        if let Some(wavetable) = WAVETABLE.try_take() {
            audio.set_wavetable(wavetable);
        }

        if hardware
            .device
            .poll(&mut [&mut hardware.hid, &mut hardware.audio])
        {
            let report: KeyboardReport = KEYBOARD_STATUS.lock(|status| *status).into();

            // We don't care about errors here, since we'll write the next samples soon enough
            // anyway.
            let _ = hardware.hid.push_input(&report);

            match hardware.audio.write(bytemuck::cast_slice(&sample)) {
                Err(usbd_audio::Error::UsbError(usbd_hid::UsbError::WouldBlock)) => {
                    // This poll wasn't meant for audio. We'll write the sample next time.
                }
                _ => {
                    // Write the next sample to the buffer. The previous one either wrote successfully
                    // or a meaningfull error occurred, so we should move on to the next sample.
                    sample = audio.sample();
                }
            }
        }
    }
}
