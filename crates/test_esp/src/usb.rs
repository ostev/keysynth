use embassy_executor::{SendSpawner, Spawner, task};

use embassy_sync::blocking_mutex::Mutex;
use esp_hal::{
    interrupt::software::SoftwareInterrupt,
    otg_fs::{self, Usb},
    peripherals::{CPU_CTRL, GPIO19, GPIO20, USB0},
};
use esp_println::println;
use esp_sync::RawMutex;
use static_cell::StaticCell;
use usb_device::{
    LangID,
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_audio::{AudioClass, AudioClassBuilder, StreamConfig, TerminalType};
use usbd_hid::{
    descriptor::{KeyboardReport, SerializedDescriptor},
    hid_class::{
        self, HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass,
        ProtocolModeConfig,
    },
};

use crate::{
    audio::{self, AUDIO_REFRESH_MS},
    concurrency::try_receive_all,
    usb::{audio::SAMPLE_RATE, hid::UsbKeyboardStatus},
};

pub mod hid;

pub type Bus = esp_hal::otg_fs::UsbBus<esp_hal::otg_fs::Usb<'static>>;

pub struct Peripherals<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> {
    pub usb: USB0<'static>,
    pub dp: DP,
    pub dm: DM,
}

impl<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> Peripherals<DP, DM> {
    pub fn build(self) -> UsbHardware {
        static EP_OUT_BUFFER: StaticCell<[u32; 4096]> = StaticCell::new();

        let usb = Usb::new(self.usb, self.dp, self.dm);

        static USB_BUS: StaticCell<UsbBusAllocator<Bus>> = StaticCell::new();

        let bus: &'static mut _ = USB_BUS.init_with(|| {
            esp_hal::otg_fs::UsbBus::new(
                usb,
                EP_OUT_BUFFER.init_with(|| [0; 4096]),
                // esp_hal::otg_fs::asynch::Config::default(),
            )
        });

        // let config = {
        //     // TODO: replace PID and VID
        //     let mut config = embassy_usb::Config::new(0x1209, 0x0001);
        //     config.product = Some("KeySynth");
        //     config.max_power = 500;

        //     // Standard USB 2.0 full-speed settings
        //     config.max_packet_size_0 = 64;
        //     config.supports_remote_wakeup = false;
        //     config.bcd_usb = UsbVersion::Two;

        //     config
        // };

        // let mut builder = {
        //     static CONFIG_DESCRIPTOR: StaticCell<[u8; 512]> = StaticCell::new();
        //     static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        //     static CONTROL_BUF: StaticCell<[u8; 1024]> = StaticCell::new();

        //     embassy_usb::Builder::new(
        //         driver,
        //         config,
        //         CONFIG_DESCRIPTOR.init_with(|| [0; 512]),
        //         BOS_DESCRIPTOR.init_with(|| [0; 256]),
        //         // No MSOS-specific descriptors
        //         &mut [],
        //         CONTROL_BUF.init_with(|| [0; 1024]),
        //     )
        // };

        // let audio = {
        // let (audio_endpoint, feedback_endpoint, handler) = uac1::source::AudioSource::new(
        //     &mut builder,
        //     &[SAMPLE_RATE],
        //     uac1::SampleWidth::Width2Byte,
        //     AUDIO_REFRESH_MS,
        //     Some(uac1::terminal_type::TerminalType::Synthesizer),
        // );

        // static AUDIO_CONTROL_HANDLER: StaticCell<AudioSourceControlHandler> = StaticCell::new();
        // let audio_control_handler = AUDIO_CONTROL_HANDLER.init(handler);

        // builder.handler(audio_control_handler);

        // audio::Hardware {
        //     audio_endpoint,
        //     feedback_endpoint,
        // }
        // };

        // let hid = {
        //     static STATE: StaticCell<embassy_usb::class::hid::State> = StaticCell::new();
        //     const REPORT_POLLING_MS: u8 = 4;

        //     let config = embassy_usb::class::hid::Config {
        //         report_descriptor: KeyboardReport::desc(),
        //         request_handler: None,
        //         poll_ms: REPORT_POLLING_MS,
        //         max_packet_size: 64,
        //         hid_subclass: HidSubclass::No,
        //         hid_boot_protocol: HidBootProtocol::Keyboard,
        //     };

        //     let writer = HidWriter::new(
        //         &mut builder,
        //         STATE.init_with(|| embassy_usb::class::hid::State::new()),
        //         config,
        //     );

        //     UsbHidHardware { writer }
        // };

        // let device = builder.build();
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

        let device: UsbDevice<'static, Bus> = UsbDeviceBuilder::new(bus, UsbVidPid(0x1209, 0x0001))
            .max_power(500)
            .unwrap()
            .strings(&[StringDescriptors::new(LangID::EN).product("KeySynth")])
            .unwrap()
            .composite_with_iads()
            .build();

        UsbHardware { device, audio, hid }
        // UsbHardware { device, audio }
    }
}

pub struct UsbHardware {
    device: UsbDevice<'static, Bus>,
    audio: AudioClass<'static, Bus>,
    hid: HIDClass<'static, Bus>,
}

// #[task]
// pub async fn usb_device(
//     spawner: SendSpawner,
//     cpu_control: CPU_CTRL<'static>,
//     interrupt: SoftwareInterrupt<'static, 1>,
//     peripherals: Peripherals<GPIO20<'static>, GPIO19<'static>>,
// ) {
//     let hardware = peripherals.build();

//     // spawner.spawn(hid::hid(hardware.hid).unwrap());
//     audio::start(cpu_control, interrupt, hardware.audio);

//     // hardware.device.run().await
// }

mod audio_channel {
    use crate::{audio, channel};

    pub const CAPACITY: usize = 16;

    channel!(audio::Event, CAPACITY);
}
pub use audio_channel::sender as audio_sender;

// mod hid_channel {
//     use crate::{channel, usb::hid::UsbKeyboardStatus};

//     channel! { UsbKeyboardStatus }
// }

// pub use hid_channel::sender as hid_sender;
static KEYBOARD_STATUS: Mutex<RawMutex, UsbKeyboardStatus> = Mutex::new(UsbKeyboardStatus::empty());

/// Updates the keyboard status, which will then be sent over USB.
/// ## Safety
/// this function **must** not be called from inside another locked mutex.
pub unsafe fn set_keyboard_status(status: UsbKeyboardStatus) {
    unsafe {
        KEYBOARD_STATUS.lock_mut(|locked_status| {
            *locked_status = status;
        })
    }
}

pub const STACK_SIZE: usize = 48_000;

pub fn device_loop(mut hardware: UsbHardware) -> ! {
    // let sinetab = [
    //     0i16, 4276, 8480, 12539, 16383, 19947, 23169, 25995, 28377, 30272, 31650, 32486, 32767,
    //     32486, 31650, 30272, 28377, 25995, 23169, 19947, 16383, 12539, 8480, 4276, 0, -4276, -8480,
    //     -12539, -16383, -19947, -23169, -25995, -28377, -30272, -31650, -32486, -32767, -32486,
    //     -31650, -30272, -28377, -25995, -23169, -19947, -16383, -12539, -8480, -4276,
    // ];
    // let sinetab_le = unsafe { &*(&sinetab as *const _ as *const [u8; 96]) };

    let mut audio = audio::State::new();
    println!("Init!");

    let audio_receiver = audio_channel::receiver();

    let mut sample = audio.sample();

    loop {
        // Apply the next 4 events
        let audio_events = try_receive_all(&audio_receiver).take(4);
        audio.apply_events(audio_events);

        if hardware
            .device
            .poll(&mut [&mut hardware.hid, &mut hardware.audio])
        {
            let report: KeyboardReport = KEYBOARD_STATUS.lock(|status| *status).into();
            // Likewise, we don't care about HID errors since we can't really do anything about them.

            // if let Ok(_) = hardware.audio.write(&[]) {

            // We don't care about errors here, since we'll write the next samples soon enough
            // anyway.
            let _ = hardware.hid.push_input(&report);
            match hardware.audio.write(bytemuck::cast_slice(&sample)) {
                Err(usbd_audio::Error::UsbError(usbd_hid::UsbError::WouldBlock)) => {
                    // This poll wasn't meant for audio.
                }
                _ => {
                    // Write the next sample to the buffer
                    sample = audio.sample();
                }
            }

            // Err(_) => {}
            // }
        }
    }
}
