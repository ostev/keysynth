use esp_hal::{
    Blocking,
    clock::CpuClock,
    dma::{self, DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    i2s::{self, master::I2sTx},
    interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl},
    pcnt::Pcnt,
    peripherals::{CPU_CTRL, GPIO7, GPIO8},
    psram::{FlashFreq, SpiRamFreq},
    spi::{self, master::Spi},
    time::Rate,
    timer::timg::TimerGroup,
    uart::{self, UartRx},
};
use esp_println::println;
use esp_storage::FlashStorage;
use keyboard_protocol::uart::BAUDRATE;

use crate::{
    gui::display::{self, DisplayHardware},
    input::{encoder, keyboard::KeyboardHardware},
    storage::StorageHardware,
    usb,
};

/// Contains all the hardware that the synthesiser talks to.
pub struct Hardware {
    pub interrupt_1: SoftwareInterrupt<'static, 1>,
    pub interrupt_2: SoftwareInterrupt<'static, 2>,
    pub cpu_control: CPU_CTRL<'static>,

    pub keyboard: KeyboardHardware,
    pub encoder: encoder::Hardware<GPIO7<'static>, GPIO8<'static>>,

    pub display: DisplayHardware,

    pub usb: usb::UsbHardware,
    pub storage: StorageHardware,
}

/// An error that occurred during hardware initialisation.
#[derive(Debug)]
pub enum InitError {
    I2SConfigError(i2s::master::ConfigError),
    UartConfigError(uart::ConfigError),
    SpiConfigError(spi::master::ConfigError),
    DmaBufError(dma::DmaBufError),
}

impl Hardware {
    pub async fn new() -> Result<Hardware, InitError> {
        let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

        // Initialise heap memory
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

        // Start the executor
        esp_rtos::start(timer_group_0.timer0, software_interrupt.software_interrupt0);

        // This is used later to start the second core
        let cpu_control = peripherals.CPU_CTRL;

        // UART receiver from RP2040
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

        // The primary encoder hardware
        let encoder = {
            encoder::Hardware {
                counter: Pcnt::new(peripherals.PCNT),
                a: peripherals.GPIO7,
                b: peripherals.GPIO8,
            }
        };

        // The SPI display driver
        let display = {
            let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32_000);

            let dma_rx_buf =
                DmaRxBuf::new(rx_descriptors, rx_buffer).map_err(InitError::DmaBufError)?;
            let dma_tx_buf =
                DmaTxBuf::new(tx_descriptors, tx_buffer).map_err(InitError::DmaBufError)?;

            let spi: esp_hal::spi::master::SpiDmaBus<'static, Blocking> = Spi::new(
                peripherals.SPI2,
                spi::master::Config::default()
                    .with_frequency(Rate::from_mhz(5))
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

        println!("Storage setup complete!");

        let usb = usb::Peripherals {
            usb: peripherals.USB0,
            dp: peripherals.GPIO20,
            dm: peripherals.GPIO19,
        }
        .build();

        println!("USB setup complete!");

        Ok(Hardware {
            interrupt_1: software_interrupt.software_interrupt1,
            interrupt_2: software_interrupt.software_interrupt2,
            cpu_control,

            // },
            keyboard,
            encoder,
            usb,
            display,
            storage,
        })
    }
}
