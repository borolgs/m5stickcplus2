#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::boxed::Box;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::{clock::CpuClock, delay::Delay};
use m5stickcplus2::app::App;
use m5stickcplus2::button::Buttons;
use mipidsi::interface::SpiInterface;
use mipidsi::options::{ColorInversion, Orientation, Rotation};
use mousefood::EmbeddedBackend;
use mousefood::EmbeddedBackendConfig;
use ratatui::Terminal;

extern crate alloc;

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

    esp_alloc::heap_allocator!(size: 180000);

    let output_config = OutputConfig::default();
    let button_config = InputConfig::default().with_pull(Pull::Up);

    let _power = Output::new(peripherals.GPIO4, Level::High, output_config);

    let mut display = {
        let mut delay = Delay::new();

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

        let buffer = Box::leak(Box::new([0_u8; 512]));
        let di = SpiInterface::new(spi_device, dc, buffer);

        let mut display = mipidsi::Builder::new(mipidsi::models::ST7789, di)
            .display_size(135, 240)
            .display_offset(52, 40)
            .invert_colors(ColorInversion::Inverted)
            .orientation(Orientation::new().rotate(Rotation::Deg90))
            .reset_pin(rst)
            .init(&mut delay)
            .unwrap();

        display.clear(Rgb565::BLACK).unwrap();

        display
    };

    let backend = EmbeddedBackend::new(
        &mut display,
        EmbeddedBackendConfig {
            ..Default::default()
        },
    );

    let mut terminal = Terminal::new(backend).unwrap();

    let buttons = Buttons::new(
        Input::new(peripherals.GPIO37, button_config),
        Input::new(peripherals.GPIO39, button_config),
        Input::new(peripherals.GPIO35, button_config),
    );
    let mut app = App::new(buttons);

    let _backlight = Output::new(peripherals.GPIO27, Level::High, output_config);

    app.run(&mut terminal).unwrap();

    loop {}
}
