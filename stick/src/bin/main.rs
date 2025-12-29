#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::boxed::Box;
use app::{App, EVENTS, Event, Sender, Stats};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc::HEAP;
use esp_backtrace as _;
use esp_hal::Blocking;
use esp_hal::analog::adc::{Adc, AdcConfig, AdcPin, Attenuation};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::peripherals::{ADC1, GPIO38};
use esp_hal::rmt::{Rmt, RxChannelCreator, TxChannelCreator};
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
use stick::ir;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn buttons_task(mut button: Buttons) {
    loop {
        button.update().await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn battery_task(
    mut adc: Adc<'static, ADC1<'static>, Blocking>,
    mut pin: AdcPin<GPIO38<'static>, ADC1<'static>>,
    sender: Sender,
) {
    loop {
        let adc_value: u16 = nb::block!(adc.read_oneshot(&mut pin)).unwrap();
        let battery_mv = (adc_value as u32 * 2600 * 4 / 4096) as u16;
        let level = ((battery_mv as i32 - 3300) * 100 / (4150 - 3300)).clamp(0, 100) as u8;
        log::info!("Battery: {} mV - {}%", battery_mv, level);
        sender
            .publish(Event::StatsUpdated(Stats {
                battery_level: level,
                heap_used: HEAP.used(),
                heap_free: HEAP.free(),
            }))
            .await;
        Timer::after(Duration::from_secs(30)).await;
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

    let mut adc_config = AdcConfig::new();
    let battery_pin = adc_config.enable_pin(peripherals.GPIO38, Attenuation::_11dB);
    let adc = Adc::new(peripherals.ADC1, adc_config);
    spawner
        .spawn(battery_task(adc, battery_pin, EVENTS.publisher().unwrap()))
        .unwrap();

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
            .orientation(Orientation::new().rotate(Rotation::Deg270))
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
        EVENTS.publisher().unwrap(),
        Input::new(peripherals.GPIO37, button_config),
        Input::new(peripherals.GPIO39, button_config),
        Input::new(peripherals.GPIO35, button_config),
    );

    let mut app = App::new();

    spawner.spawn(buttons_task(buttons)).unwrap();

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();

    let ir_tx_channel = rmt
        .channel0
        .configure_tx(peripherals.GPIO19, ir::tx_config())
        .unwrap();

    let ir_rx_channel = rmt
        .channel1
        .configure_rx(peripherals.GPIO33, ir::rx_config())
        .unwrap();

    spawner
        .spawn(ir::tx_task(EVENTS.subscriber().unwrap(), ir_tx_channel))
        .unwrap();
    spawner.spawn(ir::rx_task(ir_rx_channel)).unwrap();

    let _backlight = Output::new(peripherals.GPIO27, Level::High, output_config);

    app.run(&mut terminal).await.unwrap();

    loop {}
}
