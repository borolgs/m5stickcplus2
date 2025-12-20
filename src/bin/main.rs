#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::format;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::jis_x0201::FONT_10X20;
use embedded_graphics::text::Text;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::{clock::CpuClock, delay::Delay};
use log::info;
use mipidsi::interface::SpiInterface;
use mipidsi::options::ColorInversion;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let mut delay = Delay::new();

    let output_config = OutputConfig::default();

    Output::new(peripherals.GPIO4, Level::High, output_config); // power

    let dc = Output::new(peripherals.GPIO14, Level::Low, output_config);

    let mut rst = Output::new(peripherals.GPIO12, Level::Low, output_config);
    rst.set_high();

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_mhz(40)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO15);

    let cs_output = Output::new(peripherals.GPIO5, Level::High, output_config);
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_output).unwrap();

    let mut buffer = [0_u8; 512];
    let di = SpiInterface::new(spi_device, dc, &mut buffer);

    let mut display = mipidsi::Builder::new(mipidsi::models::ST7789, di)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    display.clear(Rgb565::BLACK).unwrap();

    Output::new(peripherals.GPIO27, Level::High, output_config); // backlight

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let mut count: u64 = 0;
    loop {
        info!("Hello {}", count);
        count += 1;

        display.clear(Rgb565::BLACK).unwrap();
        Text::new(&format!("Hello {}", count), Point::new(20, 40), text_style)
            .draw(&mut display)
            .unwrap();

        delay.delay_millis(1000);
    }
}
