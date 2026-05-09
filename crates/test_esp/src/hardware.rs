use embassy_usb::{
    UsbDevice,
    class::{
        hid::{self, HidWriter},
        uac1,
    },
};
use esp_hal::{
    Async, Blocking,
    clock::{ClockConfig, CpuClock},
    dma::{self, DmaInterrupt, DmaRxBuf},
    dma_buffers,
    i2s::{
        self,
        master::{Channels, I2s, I2sTx},
    },
    interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl},
    otg_fs::{Usb, UsbBus},
    peripherals::{Peripherals, TIMG0},
    system::CpuControl,
    time::Rate,
    timer::timg::TimerGroup,
    uart::{self, Config, Uart, UartRx},
};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use crate::audio::SAMPLE_RATE;

pub struct Hardware {
    pub timer_group_0: TimerGroup<'static, TIMG0<'static>>,
    pub context_switch_interrupt: SoftwareInterruptControl<'static>,
    pub cpu_control: CpuControl<'static>,

    pub audio: AudioHardware,
    pub keyboard: KeyboardHardware,

    pub hid: UsbHidHardware,
}

#[derive(Clone, Copy, Debug)]
pub enum InitError {
    I2SConfigError(i2s::master::ConfigError),
    UartConfigError(uart::ConfigError),
}

impl Hardware {
    pub fn new() -> Result<Hardware, InitError> {
        let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

        let context_switch_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        let timer_group_0 = TimerGroup::new(peripherals.TIMG0);

        let cpu_control = CpuControl::new(peripherals.CPU_CTRL);

        let analog_audio = {
            let i2s = I2s::new(
                peripherals.I2S0,
                peripherals.DMA_CH0,
                i2s::master::Config::new_tdm_philips()
                    .with_sample_rate(Rate::from_hz(SAMPLE_RATE))
                    .with_data_format(i2s::master::DataFormat::Data16Channel16)
                    .with_channels(Channels::STEREO),
            )
            .map_err(InitError::I2SConfigError)?
            .with_mclk(peripherals.GPIO34);

            let (tx_buffer, tx_descriptors, _, _) = dma_buffers!(4 * 4092, 0);

            let i2s_tx = i2s
                .i2s_tx
                .with_bclk(peripherals.GPIO33)
                .with_dout(peripherals.GPIO47)
                .with_ws(peripherals.GPIO48)
                .build(tx_descriptors);

            AnalogAudioHardware { i2s_tx, tx_buffer }
        };

        let keyboard = {
            let uart = UartRx::new(peripherals.UART0, Config::default())
                .map_err(InitError::UartConfigError)?
                .with_rx(peripherals.GPIO12);

            KeyboardHardware { uart }
        };

        let (hid, usb_audio) = {
            static EP_OUT_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();

            let usb = Usb::new(peripherals.USB0, peripherals.GPIO20, peripherals.GPIO19);
            let driver = esp_hal::otg_fs::asynch::Driver::new(
                usb,
                EP_OUT_BUFFER.init_with(|| [0; 1024]),
                esp_hal::otg_fs::asynch::Config::default(),
            );

            let config = {
                // TODO: replace PID and VID
                let mut config = embassy_usb::Config::new(0xffff, 0xffff);
                config.product = Some("KeySynth");
                config.max_power = 500;
                config
            };

            let mut builder = {
                static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
                static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
                static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

                embassy_usb::Builder::new(
                    driver,
                    config,
                    CONFIG_DESCRIPTOR.init_with(|| [0; 256]),
                    BOS_DESCRIPTOR.init_with(|| [0; 256]),
                    // No MSOS-specific descriptors
                    &mut [],
                    CONTROL_BUF.init_with(|| [0; 64]),
                )
            };

            let hid = {
                static STATE: StaticCell<hid::State> = StaticCell::new();
                const REPORT_POLLING_MS: u8 = 4;

                let config = hid::Config {
                    report_descriptor: KeyboardReport::desc(),
                    request_handler: None,
                    poll_ms: REPORT_POLLING_MS,
                    max_packet_size: 64,
                    hid_subclass: hid::HidSubclass::No,
                    hid_boot_protocol: hid::HidBootProtocol::Keyboard,
                };

                let writer =
                    HidWriter::new(&mut builder, STATE.init_with(|| hid::State::new()), config);

                UsbHidHardware { writer }
            };

            let audio = {
                const AUDIO_REFRESH_MS: u8 = 2;

                let (endpoint, _, _) = uac1::source::AudioSource::new(
                    &mut builder,
                    &[SAMPLE_RATE],
                    uac1::SampleWidth::Width4Byte,
                    AUDIO_REFRESH_MS,
                    Some(uac1::terminal_type::TerminalType::Synthesizer),
                );

                UsbAudioHardware { endpoint }
            };

            (hid, audio)
        };

        Ok(Hardware {
            context_switch_interrupt,
            timer_group_0,
            cpu_control,
            audio: AudioHardware {
                analog: analog_audio,
                usb: usb_audio,
            },
            keyboard,
            hid,
        })
    }
}

pub struct AnalogAudioHardware {
    pub i2s_tx: I2sTx<'static, Blocking>,
    pub tx_buffer: &'static mut [u8],
}

pub type UsbDriver = esp_hal::otg_fs::asynch::Driver<'static>;

pub struct UsbAudioHardware {
    pub endpoint: uac1::source::AudioSourceEpIn<'static, UsbDriver>,
}

pub struct UsbHidHardware {
    pub writer: HidWriter<'static, UsbDriver, { UsbHidHardware::REPORT_SIZE }>,
}

impl UsbHidHardware {
    const REPORT_SIZE: usize = 8;
}

pub struct KeyboardHardware {
    pub uart: UartRx<'static, Blocking>,
}

pub struct AudioHardware {
    pub analog: AnalogAudioHardware,
    pub usb: UsbAudioHardware,
}
