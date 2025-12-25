#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::boxed::Box;
use app::{App, events};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, delay::Delay};
use mipidsi::interface::SpiInterface;
use mipidsi::options::{ColorInversion, Orientation, Rotation};
use mousefood::EmbeddedBackend;
use mousefood::EmbeddedBackendConfig;
use ratatui::Terminal;
use stick::button::Buttons;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn buttons_task(mut button: Buttons) {
    loop {
        button.update().await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    esp_alloc::heap_allocator!(size: 80000);

    // battery
    let mut adc_config = AdcConfig::new();
    let mut battery_pin = adc_config.enable_pin(peripherals.GPIO38, Attenuation::_11dB);
    let mut adc = Adc::new(peripherals.ADC1, adc_config);

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

    let channel = Box::leak(Box::new(events::channel()));

    let sender = channel.publisher().unwrap();

    let buttons = Buttons::new(
        sender,
        Input::new(peripherals.GPIO37, button_config),
        Input::new(peripherals.GPIO39, button_config),
        Input::new(peripherals.GPIO35, button_config),
    );

    let adc_value: u16 = nb::block!(adc.read_oneshot(&mut battery_pin)).unwrap();
    let battery_mv = adc_value * 2;
    let battery_percent = ((battery_mv as i32 - 3300) * 100 / (4150 - 3350)).clamp(0, 100);
    log::info!(
        "Battery: {} mV ({:.2}V) - {}%",
        battery_mv,
        battery_mv as f32 / 1000.0,
        battery_percent
    );

    let mut app = App::new(channel.publisher().unwrap(), channel.subscriber().unwrap());

    let _backlight = Output::new(peripherals.GPIO27, Level::High, output_config);

    spawner.spawn(buttons_task(buttons)).unwrap();

    app.run(&mut terminal).await.unwrap();

    loop {}
}
