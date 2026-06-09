use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin;
use esp_hal::{
    Async, Blocking, DriverMode,
    spi::{master::SpiDmaBus, slave::Spi},
};
use st7789v2::{ControllerInterface, DisplaySize, ResetInterface, St7789v2};

pub const SIZE: DisplaySize = DisplaySize::new(240, 260);

pub const FRAMEBUFFER_SIZE: usize = st7789v2::framebuffer_size(SIZE, st7789v2::ColorMode::Rgb565);

pub type Color = Rgb565;

pub struct DisplayHardware<ResetPin: OutputPin> {
    pub driver: Driver<ResetPin>,
}

pub struct Driver<ResetPin: OutputPin> {
    pub spi: Spi<'static, Async>,
    pub reset_pin: ResetPin,
}

impl<ResetPin: OutputPin> Driver<ResetPin> {
    // pub fn init(&mut self) {
    //     self.
    // }

    /// Synchronously reset the display
    pub fn reset(&mut self) {
        let delay = || esp_hal::rom::ets_delay_us(10_000_000);

        self.reset_pin.set_high();
        delay();
        self.reset_pin.set_low();
        delay();
        self.reset_pin.set_high();
        delay();
    }

    fn write_command(&mut self, command: ()) {
        self.spi.
    }

    pub fn init(&mut self) {
        self.reset();
        // self.
    }
}
