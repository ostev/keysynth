use core::convert::Infallible;

use alloc::boxed::Box;
use embassy_time::Timer;
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    framebuffer::buffer_size,
    geometry::{Dimensions, Point, Size},
    pixelcolor::{
        Rgb565,
        raw::{BigEndian, RawU16, ToBytes},
    },
    primitives::Rectangle,
};
use esp_hal::{
    Async, Blocking,
    gpio::Output,
    spi::{self, master::SpiDmaBus},
};

pub type Color = Rgb565;

pub struct DisplayHardware {
    pub spi: SpiDmaBus<'static, Blocking>,
    pub reset_pin: Output<'static>,
    pub dc_pin: Output<'static>,
}

struct DriverHardware {
    pub spi: SpiDmaBus<'static, Async>,
    pub reset_pin: Output<'static>,
    pub dc_pin: Output<'static>,
}

impl DriverHardware {
    async fn write_command(&mut self, command: u8) -> Result<(), spi::Error> {
        self.dc_pin.set_low();
        self.spi.write_async(&[command]).await
    }

    async fn write_data(&mut self, data: &[u8]) -> Result<(), spi::Error> {
        self.dc_pin.set_high();
        self.spi.write_async(data).await
    }

    async fn write_packet(&mut self, command: u8, data: &[u8]) -> Result<(), spi::Error> {
        self.write_command(command).await?;
        self.write_data(data).await
    }

    /// Synchronously reset the display
    fn reset(&mut self) {
        let delay = || esp_hal::rom::ets_delay_us(120_000);

        self.reset_pin.set_high();
        delay();
        self.reset_pin.set_low();
        delay();
        self.reset_pin.set_high();
        delay();
    }

    async fn enable_ram_write(&mut self) -> Result<(), spi::Error> {
        // Send the RAMWR command
        self.write_command(0x2c).await
    }

    async fn init_registers(&mut self) -> Result<(), spi::Error> {
        for _ in 0..3 {
            self.write_command(0xaa).await?;
        }
        self.write_packet(0x36, &[0x00]).await?;

        self.write_packet(0x3a, &[0x05]).await?;

        self.write_packet(0xb2, &[0x0b, 0x0b, 0x00, 0x33, 0x35])
            .await?;

        self.write_packet(0xb7, &[0x2c]).await?;

        self.write_packet(0xc2, &[0x01]).await?;

        self.write_packet(0xc3, &[0x0d]).await?;

        // VDV, 0x20 -> 0V
        self.write_packet(0xc4, &[0x20]).await?;

        // 0x13 -> 60Hz
        self.write_packet(0xc6, &[0x13]).await?;

        self.write_packet(0xd0, &[0xa4, 0xa1]).await?;

        self.write_packet(0xd6, &[0xa1]).await?;

        self.write_packet(
            0xe0,
            &[
                0xf0, 0x06, 0x0b, 0x0a, 0x09, 0x26, 0x29, 0x33, 0x41, 0x18, 0x16, 0x15, 0x29, 0x2d,
            ],
        )
        .await?;

        self.write_packet(
            0x31,
            &[
                0xf0, 0x04, 0x08, 0x08, 0x07, 0x03, 0x28, 0x32, 0x40, 0x3b, 0x19, 0x18, 0x2a, 0x2e,
            ],
        )
        .await?;

        self.write_packet(0xe4, &[0x25, 0x00, 0x00]).await?;

        self.write_command(0x21).await?;
        self.write_command(0x11).await?;

        Timer::after_millis(120).await;
        self.write_command(0x29).await
    }

    async fn set_orientation(&mut self, orientation: Orientation) -> Result<(), spi::Error> {
        let memory_access_register = match orientation {
            Orientation::Horizontal => 0x70,
            Orientation::Vertical => 0x00,
        };

        self.write_packet(0x36, &[memory_access_register]).await
    }

    async fn set_window(
        &mut self,
        window: Window,
        orientation: Orientation,
    ) -> Result<(), spi::Error> {
        // The vertical offset of the display controller
        const ROW_START: u16 = 20;

        let Window {
            x_start,
            y_start,
            x_end,
            y_end,
        } = match orientation {
            Orientation::Vertical => Window {
                x_start: window.x_start,
                x_end: window.x_end,

                y_start: window.y_start + ROW_START,
                y_end: window.y_end + ROW_START,
            },
            Orientation::Horizontal => Window {
                x_start: window.y_start + ROW_START,
                x_end: window.y_end + ROW_START,

                y_start: window.x_start,
                y_end: window.x_end,
            },
        };

        let x_end_inclusive = x_end - 1;
        let y_end_inclusive = y_end - 1;

        // Sets the x coordinates
        self.write_packet(
            0x2a,
            &[
                // Send the x start position as top 8
                // and then bottom 8 bits:
                (x_start >> 8) as u8,
                x_start as u8,
                // Same for x end:
                (x_end_inclusive >> 8) as u8,
                x_end_inclusive as u8,
            ],
        )
        .await?;
        self.write_packet(
            0x2b,
            &[
                (y_start >> 8) as u8,
                y_start as u8,
                (y_end_inclusive >> 8) as u8,
                y_end_inclusive as u8,
            ],
        )
        .await?;

        self.enable_ram_write().await
    }

    pub async fn clear_async(
        &mut self,
        color: Color,
        orientation: Orientation,
    ) -> Result<(), spi::Error> {
        let (width, height) = orientation.dimensions();

        self.set_window(
            Window {
                x_start: 0,
                y_start: 0,
                x_end: width as u16,
                y_end: height as u16,
            },
            orientation,
        )
        .await?;

        // Allocate the stack space for the clear row for the largest dimension
        let clear_row_buffer = [color.to_be_bytes(); if WIDTH > HEIGHT { WIDTH } else { HEIGHT }];
        // Get a slice of just the part we need
        let clear_row = clear_row_buffer[0..width].as_flattened();

        for _ in 0..height {
            self.write_data(clear_row).await?;
        }

        Ok(())
    }
}

pub struct Driver {
    hardware: DriverHardware,

    orientation: Orientation,
    framebuffer: Box<Framebuffer>,
}

type Framebuffer = embedded_graphics::framebuffer::Framebuffer<
    Rgb565,
    RawU16,
    BigEndian,
    WIDTH,
    HEIGHT,
    { buffer_size::<Rgb565>(WIDTH, HEIGHT) },
>;

pub const WIDTH: usize = 280;
pub const HEIGHT: usize = 240;

// TODO: fix orientation bug

impl Driver {
    pub async fn init(
        hardware: DisplayHardware,
        orientation: Orientation,
    ) -> Result<Driver, spi::Error> {
        let driver_hardware = DriverHardware {
            spi: hardware.spi.into_async(),
            reset_pin: hardware.reset_pin,
            dc_pin: hardware.dc_pin,
        };

        let mut driver = Driver {
            hardware: driver_hardware,
            orientation,
            framebuffer: Box::new(Framebuffer::new()),
        };

        driver.hardware.reset();
        driver.hardware.init_registers().await?;
        driver.hardware.set_orientation(orientation).await?;

        Ok(driver)
    }

    pub async fn set_orientation(&mut self, orientation: Orientation) -> Result<(), spi::Error> {
        self.orientation = orientation;
        self.hardware.set_orientation(orientation).await
    }

    pub async fn clear_async(&mut self, color: Color) -> Result<(), spi::Error> {
        self.hardware.clear_async(color, self.orientation).await
    }

    async fn set_window(&mut self, window: Window) -> Result<(), spi::Error> {
        self.hardware.set_window(window, self.orientation).await
    }

    pub async fn full_flush(&mut self) -> Result<(), spi::Error> {
        let (width, height) = self.orientation.dimensions();

        self.set_window(Window {
            x_start: 0,
            y_start: 0,
            x_end: width as u16,
            y_end: height as u16,
        })
        .await?;

        let bytes = self.framebuffer.data();

        self.hardware.write_data(bytes).await?;

        Ok(())
    }
}

impl Dimensions for Driver {
    fn bounding_box(&self) -> Rectangle {
        // let (width, height) = self.orientation.dimensions();

        Rectangle::new(Point::zero(), Size::new(WIDTH as u32, HEIGHT as u32))
    }
}

impl DrawTarget for Driver {
    type Color = Color;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.framebuffer.draw_iter(pixels)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.framebuffer.fill_contiguous(area, colors)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.framebuffer.fill_solid(area, color)
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.framebuffer.clear(color);

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    pub x_start: u16,
    pub y_start: u16,
    pub x_end: u16,
    pub y_end: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn dimensions(self) -> (usize, usize) {
        match self {
            Orientation::Horizontal => (HEIGHT, WIDTH),
            Orientation::Vertical => (WIDTH, HEIGHT),
        }
    }
}
