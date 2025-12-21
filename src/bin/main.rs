#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::boxed::Box;
use alloc::format;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::jis_x0201::FONT_10X20;
use embedded_graphics::text::Text;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::{clock::CpuClock, delay::Delay, time::Instant};
use log::info;
use m5stickcplus2::button::Button;
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
    let button_config = InputConfig::default().with_pull(Pull::Up);

    let _power = Output::new(peripherals.GPIO4, Level::High, output_config);

    let mut button_a = Button::new(Input::new(peripherals.GPIO37, button_config));
    let mut button_b = Button::new(Input::new(peripherals.GPIO39, button_config));
    let mut button_c = Button::new(Input::new(peripherals.GPIO35, button_config));

    let mut display = {
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
            .reset_pin(rst)
            .init(&mut delay)
            .unwrap();

        display.clear(Rgb565::BLACK).unwrap();

        let _backlight = Output::new(peripherals.GPIO27, Level::High, output_config);

        display
    };

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let mut count: u64 = 0;
    let mut needs_redraw = true;
    let mut c_press_start = None::<Instant>;

    loop {
        button_a.update();
        button_b.update();
        button_c.update();

        if button_a.just_pressed() {
            count += 1;
            info!("Button A pressed");
        }

        if button_b.just_pressed() {
            count = 0;
            info!("Button B pressed");
        }

        let c_pressed = button_c.is_pressed();

        if button_c.just_pressed() {
            c_press_start = Some(Instant::now());
            info!("Button C pressed");
        }

        if !c_pressed {
            c_press_start = None;
        }

        if button_a.changed() || button_b.changed() || button_c.changed() {
            needs_redraw = true;
        }

        let a_pressed = button_a.is_pressed();
        let b_pressed = button_b.is_pressed();

        if needs_redraw || c_pressed {
            display.clear(Rgb565::BLACK).unwrap();

            Text::new(&format!("Count: {}", count), Point::new(20, 30), text_style)
                .draw(&mut display)
                .unwrap();

            Text::new(
                &format!("A: {}", if a_pressed { "Pressed" } else { "" }),
                Point::new(20, 60),
                text_style,
            )
            .draw(&mut display)
            .unwrap();

            Text::new(
                &format!("B: {}", if b_pressed { "Pressed" } else { "" }),
                Point::new(20, 90),
                text_style,
            )
            .draw(&mut display)
            .unwrap();

            if c_pressed {
                if let Some(start) = c_press_start {
                    let elapsed = start.elapsed();
                    let held_ms = elapsed.as_millis();

                    let color = if held_ms >= 2500 {
                        Rgb565::RED
                    } else if held_ms >= 1500 {
                        Rgb565::YELLOW
                    } else {
                        Rgb565::WHITE
                    };

                    let style = MonoTextStyle::new(&FONT_10X20, color);

                    Text::new(&format!("C: {}ms", held_ms), Point::new(20, 120), style)
                        .draw(&mut display)
                        .unwrap();
                }
            } else {
                Text::new("C:", Point::new(20, 120), text_style)
                    .draw(&mut display)
                    .unwrap();
            }

            needs_redraw = false;
        }

        delay.delay_millis(50);
    }
}
