use embedded_graphics::pixelcolor::Rgb565;
use esp_hal::{Blocking, gpio::Output, spi::master::SpiDmaBus};
use st7789v2::{ControllerInterface, DisplaySize, ResetInterface, St7789v2};

pub const SIZE: DisplaySize = DisplaySize::new(240, 260);

pub const FRAMEBUFFER_SIZE: usize = st7789v2::framebuffer_size(SIZE, st7789v2::ColorMode::Rgb565);

pub type Color = Rgb565;

pub type Driver = St7789v2<
    DisplaySpiInterface<SpiDmaBus<'static, Blocking>, Output<'static>>,
    DisplayResetInterface<Output<'static>>,
    st7789v2::Buffered,
>;

pub type DriverError = st7789v2::DriverError<
    DisplayInterfaceError<esp_hal::spi::master::SpiDmaBus<'static, Blocking>, Output<'static>>,
    <DisplayResetInterface<Output<'static>> as ResetInterface>::Error,
>;

pub struct DisplayHardware {
    pub driver: Driver,
}

pub struct DisplaySpiInterface<Spi, Dc>
where
    Spi: embedded_hal::spi::SpiBus<u8>,
    Dc: embedded_hal::digital::OutputPin,
{
    spi: Spi,
    dc: Dc,
    // cs: CS,
    ramwr_sent: bool,
}

impl<Spi, Dc> DisplaySpiInterface<Spi, Dc>
where
    Spi: embedded_hal::spi::SpiBus<u8>,
    Dc: embedded_hal::digital::OutputPin,
{
    pub const fn new(spi: Spi, dc: Dc) -> Self {
        Self {
            spi,
            dc,
            ramwr_sent: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DisplayInterfaceError<Spi, Dc>
where
    Spi: embedded_hal::spi::SpiBus<u8>,
    Dc: embedded_hal::digital::OutputPin,
{
    SpiError(Spi::Error),
    DcError(Dc::Error),
}

impl<Spi, Dc> ControllerInterface for DisplaySpiInterface<Spi, Dc>
where
    Spi: embedded_hal::spi::SpiBus<u8>,
    Dc: embedded_hal::digital::OutputPin,
{
    type Error = DisplayInterfaceError<Spi, Dc>;

    fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
        self.ramwr_sent = false;
        self.dc.set_low().map_err(DisplayInterfaceError::DcError)?;
        // self.cs.set_low().ok();
        self.spi
            .write(&[cmd])
            .map_err(DisplayInterfaceError::SpiError)?;
        // self.cs.set_high().ok();
        Ok(())
    }

    fn send_command_with_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), Self::Error> {
        self.ramwr_sent = false;
        self.dc.set_low().map_err(DisplayInterfaceError::DcError)?;
        // self.cs.set_low().ok();
        self.spi
            .write(&[cmd])
            .map_err(DisplayInterfaceError::SpiError)?;
        self.dc.set_high().map_err(DisplayInterfaceError::DcError)?;
        self.spi
            .write(data)
            .map_err(DisplayInterfaceError::SpiError)?;
        // self.cs.set_high().ok();
        Ok(())
    }

    fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), Self::Error> {
        if !self.ramwr_sent {
            self.dc.set_low().map_err(DisplayInterfaceError::DcError)?;
            // self.cs.set_low().ok();
            self.spi
                .write(&[0x2C])
                .map_err(DisplayInterfaceError::SpiError)?; // RAMWR
            self.dc.set_high().map_err(DisplayInterfaceError::DcError)?;
            self.ramwr_sent = true;
        } else {
            self.dc.set_high().map_err(DisplayInterfaceError::DcError)?;
            // self.cs.set_low().ok();
        }
        self.spi
            .write(pixels)
            .map_err(DisplayInterfaceError::SpiError)?;
        // self.cs.set_high().ok();
        Ok(())
    }
}

pub struct DisplayResetInterface<Reset: embedded_hal::digital::OutputPin> {
    reset: Reset,
}

impl<Reset: embedded_hal::digital::OutputPin> DisplayResetInterface<Reset> {
    pub const fn new(reset: Reset) -> Self {
        Self { reset }
    }
}

impl<Reset: embedded_hal::digital::OutputPin> ResetInterface for DisplayResetInterface<Reset> {
    type Error = Reset::Error;

    fn reset(&mut self) -> Result<(), Self::Error> {
        // Pull reset low for >= 10us
        self.reset.set_low()?;
        esp_hal::rom::ets_delay_us(10);
        self.reset.set_high()?;

        // TODO: wait 120ms?

        Ok(())
    }
}
