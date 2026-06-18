use embassy_executor::{SendSpawner, Spawner, task};
use embassy_usb::{
    Handler, UsbDevice, UsbVersion,
    class::{
        hid::{HidBootProtocol, HidSubclass, HidWriter},
        uac1::{self, source::AudioSourceControlHandler},
    },
};
use esp_hal::{
    otg_fs::{self, Usb},
    peripherals::{GPIO19, GPIO20, USB0},
};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use crate::usb::{
    audio::{AUDIO_REFRESH_MS, SAMPLE_RATE},
    hid::UsbHidHardware,
};

pub mod audio;
pub mod hid;

pub type Driver = esp_hal::otg_fs::asynch::Driver<'static>;

pub struct Peripherals<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> {
    pub usb: USB0<'static>,
    pub dp: DP,
    pub dm: DM,
}

impl<DP: otg_fs::UsbDp + 'static, DM: otg_fs::UsbDm + 'static> Peripherals<DP, DM> {
    pub fn build(self) -> UsbHardware {
        static EP_OUT_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();

        let usb = Usb::new(self.usb, self.dp, self.dm);
        let driver = esp_hal::otg_fs::asynch::Driver::new(
            usb,
            EP_OUT_BUFFER.init_with(|| [0; 1024]),
            esp_hal::otg_fs::asynch::Config::default(),
        );

        let config = {
            // TODO: replace PID and VID
            let mut config = embassy_usb::Config::new(0x1209, 0x0001);
            config.product = Some("KeySynth");
            config.max_power = 500;

            // Standard USB 2.0 full-speed settings
            config.max_packet_size_0 = 64;
            config.supports_remote_wakeup = false;
            config.bcd_usb = UsbVersion::Two;

            config
        };

        let mut builder = {
            static CONFIG_DESCRIPTOR: StaticCell<[u8; 512]> = StaticCell::new();
            static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
            static CONTROL_BUF: StaticCell<[u8; 1024]> = StaticCell::new();

            embassy_usb::Builder::new(
                driver,
                config,
                CONFIG_DESCRIPTOR.init_with(|| [0; 512]),
                BOS_DESCRIPTOR.init_with(|| [0; 256]),
                // No MSOS-specific descriptors
                &mut [],
                CONTROL_BUF.init_with(|| [0; 1024]),
            )
        };

        let audio = {
            let (audio_endpoint, feedback_endpoint, handler) = uac1::source::AudioSource::new(
                &mut builder,
                &[SAMPLE_RATE],
                uac1::SampleWidth::Width2Byte,
                AUDIO_REFRESH_MS,
                Some(uac1::terminal_type::TerminalType::Synthesizer),
            );

            static AUDIO_CONTROL_HANDLER: StaticCell<AudioSourceControlHandler> = StaticCell::new();
            let audio_control_handler = AUDIO_CONTROL_HANDLER.init(handler);

            builder.handler(audio_control_handler);

            audio::Hardware {
                audio_endpoint,
                feedback_endpoint,
            }
        };

        let hid = {
            static STATE: StaticCell<embassy_usb::class::hid::State> = StaticCell::new();
            const REPORT_POLLING_MS: u8 = 4;

            let config = embassy_usb::class::hid::Config {
                report_descriptor: KeyboardReport::desc(),
                request_handler: None,
                poll_ms: REPORT_POLLING_MS,
                max_packet_size: 64,
                hid_subclass: HidSubclass::No,
                hid_boot_protocol: HidBootProtocol::Keyboard,
            };

            let writer = HidWriter::new(
                &mut builder,
                STATE.init_with(|| embassy_usb::class::hid::State::new()),
                config,
            );

            UsbHidHardware { writer }
        };

        let device = builder.build();

        UsbHardware { device, audio, hid }
        // UsbHardware { device, audio }
    }
}

pub struct UsbHardware {
    device: UsbDevice<'static, Driver>,
    audio: audio::Hardware,
    hid: UsbHidHardware,
}

#[task]
pub async fn usb_device(
    spawner: SendSpawner,
    peripherals: Peripherals<GPIO20<'static>, GPIO19<'static>>,
) -> ! {
    let mut hardware = peripherals.build();

    spawner.spawn(hid::hid(hardware.hid).unwrap());
    spawner.spawn(audio::usb_audio(spawner, hardware.audio).unwrap());

    hardware.device.run().await
}
