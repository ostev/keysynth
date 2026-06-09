use embassy_usb::{
    UsbDevice,
    class::{
        hid::{self, HidWriter},
        uac1,
    },
    driver::host::pipe::Out,
};
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::{Rgb565, RgbColor},
};
use esp_hal::{
    Async, Blocking,
    clock::{ClockConfig, CpuClock, ll::UartFunctionClockConfig},
    delay::Delay,
    dma::{self, DmaInterrupt, DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    i2s::{
        self,
        master::{Channels, I2s, I2sTx},
    },
    interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl},
    otg_fs::{Usb, UsbBus},
    peripherals::{GPIO4, PSRAM, Peripherals, TIMG0},
    spi::{self, master::Spi},
    system::CpuControl,
    time::Rate,
    timer::timg::TimerGroup,
    uart::{self, Uart, UartRx},
};
use esp_println::println;
use esp_storage::FlashStorage;
use keyboard_protocol::uart::BAUDRATE;
use st7789v2::{ResetInterface, St7789v2};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use crate::{
    audio::SAMPLE_RATE,
    gui::{
        self,
        display::{self, DisplayHardware, DisplayResetInterface, DisplaySpiInterface},
    },
    hid::UsbHidHardware,
    keyboard::KeyboardHardware,
    storage::StorageHardware,
};

pub struct Hardware {
    pub timer_group_0: TimerGroup<'static, TIMG0<'static>>,
    pub context_switch_interrupt: SoftwareInterruptControl<'static>,
    pub cpu_control: CpuControl<'static>,

    pub audio: AudioHardware,
    pub keyboard: KeyboardHardware,

    pub display: DisplayHardware,

    pub hid: UsbHidHardware,
    // pub debug_uart: Uart<'static, Blocking>,
    pub storage: StorageHardware,
}

#[derive(Debug)]
pub enum InitError {
    I2SConfigError(i2s::master::ConfigError),
    UartConfigError(uart::ConfigError),
    SpiConfigError(spi::master::ConfigError),
    DmaBufError(dma::DmaBufError),
    DisplayInitError(display::DriverError),
}

impl Hardware {
    pub fn new() -> Result<Hardware, InitError> {
        let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
        esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

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

        println!("Audio setup complete!");

        let keyboard = {
            let uart = UartRx::new(
                peripherals.UART1,
                uart::Config::default().with_baudrate(BAUDRATE),
            )
            .map_err(InitError::UartConfigError)?
            .with_rx(peripherals.GPIO12)
            .into_async();

            KeyboardHardware { uart }
        };

        println!("Keyboard setup complete!");

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

        println!("USB setup complete!");

        let display = {
            let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32_000);

            let dma_rx_buf =
                DmaRxBuf::new(rx_descriptors, rx_buffer).map_err(InitError::DmaBufError)?;
            let dma_tx_buf =
                DmaTxBuf::new(tx_descriptors, tx_buffer).map_err(InitError::DmaBufError)?;

            let spi: esp_hal::spi::master::SpiDmaBus<'static, Blocking> = Spi::new(
                peripherals.SPI2,
                spi::master::Config::default()
                    .with_frequency(Rate::from_mhz(1))
                    .with_mode(spi::Mode::_0),
            )
            .map_err(InitError::SpiConfigError)?
            .with_sck(peripherals.GPIO2)
            .with_cs(peripherals.GPIO3)
            .with_mosi(peripherals.GPIO1)
            .with_dma(peripherals.DMA_CH1)
            .with_buffers(dma_rx_buf, dma_tx_buf);

            let display_interface = DisplaySpiInterface::new(
                spi,
                Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()),
            );
            let reset_interface = DisplayResetInterface::new(Output::new(
                peripherals.GPIO5,
                Level::Low,
                OutputConfig::default(),
            ));

            let mut delay = Delay::new();

            let mut driver = St7789v2::builder(display_interface, reset_interface, display::SIZE)
                .buffered::<display::Color>(st7789v2::Framebuffer::heap::<
                    { display::FRAMEBUFFER_SIZE },
                >())
                .build(st7789v2::ColorMode::Rgb565, &mut delay)
                .map_err(InitError::DisplayInitError)?;

            driver.hard_reset().unwrap();
            driver.clear(gui::colors::BACKGROUND_LIGHT);
            driver.full_flush().unwrap();

            DisplayHardware { driver }
        };

        println!("Display setup complete!");

        let storage = StorageHardware {
            flash: FlashStorage::new(peripherals.FLASH),
        };

        println!("Storage setup complete!");

        // let debug_uart = Uart::new(peripherals.UART0, uart::Config::default())
        //     .map_err(InitError::UartConfigError)?
        //     .with_tx(peripherals.GPIO43)
        //     .with_rx(peripherals.GPIO44);

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
            display,
            storage,
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

pub struct AudioHardware {
    pub analog: AnalogAudioHardware,
    pub usb: UsbAudioHardware,
}
