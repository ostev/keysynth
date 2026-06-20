use core::mem::MaybeUninit;

use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::{Rgb565, RgbColor},
};
use esp_alloc::export::enumset::__internal::EnumSetTypeRepr;
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
    pcnt::Pcnt,
    peripherals::{CPU_CTRL, GPIO4, GPIO7, GPIO8, GPIO19, GPIO20, PSRAM, Peripherals, TIMG0},
    psram::{FlashFreq, Psram, PsramConfig, SpiRamFreq},
    ram,
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
    gui::{
        self, colors,
        display::{self, DisplayHardware},
    },
    input::{encoder, keyboard::KeyboardHardware},
    storage::StorageHardware,
    usb,
};

pub struct Hardware {
    // pub timer_group_0: TimerGroup<'static, TIMG0<'static>>,
    // pub context_switch_interrupt: SoftwareInterruptControl<'static>,
    pub interrupt_1: SoftwareInterrupt<'static, 1>,
    pub interrupt_2: SoftwareInterrupt<'static, 2>,
    pub cpu_control: CPU_CTRL<'static>,

    // pub audio: AudioHardware,
    pub keyboard: KeyboardHardware,
    pub encoder: encoder::Hardware<GPIO8<'static>, GPIO7<'static>>,

    pub display: DisplayHardware,

    // pub hid: UsbHidHardware,
    // pub debug_uart: Uart<'static, Blocking>,
    // pub usb_device: UsbDevice<'static, UsbDriver>,
    pub usb: usb::UsbHardware,
    pub storage: StorageHardware,
}

#[derive(Debug)]
pub enum InitError {
    I2SConfigError(i2s::master::ConfigError),
    UartConfigError(uart::ConfigError),
    SpiConfigError(spi::master::ConfigError),
    DmaBufError(dma::DmaBufError),
    // DisplayInitError(display::DriverError),
}

impl Hardware {
    pub async fn new() -> Result<Hardware, InitError> {
        let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

        esp_alloc::psram_allocator!(
            peripherals.PSRAM,
            esp_hal::psram,
            esp_hal::psram::PsramConfig {
                mode: esp_hal::psram::PsramMode::OctalSpi,
                size: esp_hal::psram::PsramSize::AutoDetect,
                core_clock: None,
                flash_frequency: FlashFreq::FlashFreq80m,
                ram_frequency: SpiRamFreq::Freq80m,
            }
        );

        let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        let timer_group_0 = TimerGroup::new(peripherals.TIMG0);

        esp_rtos::start(timer_group_0.timer0, software_interrupt.software_interrupt0);

        let cpu_control = peripherals.CPU_CTRL;

        let keyboard = {
            let uart = UartRx::new(
                peripherals.UART1,
                uart::Config::default().with_baudrate(BAUDRATE),
            )
            .map_err(InitError::UartConfigError)?
            .with_rx(peripherals.GPIO12);

            KeyboardHardware { uart }
        };

        println!("Keyboard setup complete!");

        let encoder = {
            encoder::Hardware {
                counter: Pcnt::new(peripherals.PCNT),
                a: peripherals.GPIO8,
                b: peripherals.GPIO7,
            }
        };

        let display = {
            let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32_000);

            let dma_rx_buf =
                DmaRxBuf::new(rx_descriptors, rx_buffer).map_err(InitError::DmaBufError)?;
            let dma_tx_buf =
                DmaTxBuf::new(tx_descriptors, tx_buffer).map_err(InitError::DmaBufError)?;

            let spi: esp_hal::spi::master::SpiDmaBus<'static, Blocking> = Spi::new(
                peripherals.SPI2,
                spi::master::Config::default()
                    .with_frequency(Rate::from_mhz(2))
                    .with_mode(spi::Mode::_0),
            )
            .map_err(InitError::SpiConfigError)?
            .with_sck(peripherals.GPIO2)
            .with_cs(peripherals.GPIO3)
            .with_mosi(peripherals.GPIO1)
            .with_dma(peripherals.DMA_CH1)
            .with_buffers(dma_rx_buf, dma_tx_buf);

            let dc_pin = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
            let reset_pin = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());

            display::DisplayHardware {
                spi,
                reset_pin,
                dc_pin,
            }
        };

        println!("Display setup complete!");

        let storage = StorageHardware {
            flash: FlashStorage::new(peripherals.FLASH),
        };

        let usb = usb::Peripherals {
            usb: peripherals.USB0,
            dp: peripherals.GPIO20,
            dm: peripherals.GPIO19,
        }
        .build();

        println!("Storage setup complete!");

        Ok(Hardware {
            // context_switch_interrupt,
            // timer_group_0,
            // debug_uart,
            interrupt_1: software_interrupt.software_interrupt1,
            interrupt_2: software_interrupt.software_interrupt2,
            cpu_control,
            // audio: AudioHardware {
            //     analog: analog_audio,
            //     usb: usb_audio,
            // },
            keyboard,
            encoder,
            usb,
            display,
            storage,
        })
    }
}

pub struct AnalogAudioHardware {
    pub i2s_tx: I2sTx<'static, Blocking>,
    pub tx_buffer: &'static mut [u8],
}
