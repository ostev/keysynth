use embassy_executor::{SendSpawner, Spawner, task};

use esp_hal::{
    interrupt::software::SoftwareInterrupt,
    otg_fs::{self, Usb},
    peripherals::{CPU_CTRL, GPIO19, GPIO20, USB0},
};
use esp_println::println;
use static_cell::StaticCell;
use usb_device::{
    LangID,
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_audio::{AudioClassBuilder, StreamConfig, TerminalType};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use crate::{
    audio::{self, AUDIO_REFRESH_MS},
    usb::{audio::SAMPLE_RATE, hid::UsbHidHardware},
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
        let audio = {
            let audio_device = AudioClassBuilder::new()
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

            audio::Hardware {
                device: audio_device,
            }
        };

        let device: UsbDevice<'static, Bus> = UsbDeviceBuilder::new(bus, UsbVidPid(0x1209, 0x0001))
            .max_power(500)
            .unwrap()
            .strings(&[StringDescriptors::new(LangID::EN).product("KeySynth")])
            .unwrap()
            .build();

        UsbHardware { device, audio }
        // UsbHardware { device, audio }
    }
}

pub struct UsbHardware {
    device: UsbDevice<'static, Bus>,
    audio: audio::Hardware,
    // hid: UsbHidHardware,
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

    loop {
        if hardware.device.poll(&mut [&mut hardware.audio.device]) {
            let sample = audio.sample();
            match hardware.audio.device.write(bytemuck::cast_slice(&sample)) {
                Ok(_) => {}
                Err(err) => {
                    println!("audio write error {:?}", err);
                }
            }
        }
    }
}
