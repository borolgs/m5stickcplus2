use core::cell::RefCell;

use alloc::boxed::Box;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    peripherals::{GPIO5, GPIO12, GPIO13, GPIO14, GPIO15, SPI2},
    spi::master::{Config, Spi},
    time::Rate,
};
use mipidsi::{
    Display,
    interface::SpiInterface,
    options::{ColorInversion, Orientation, Rotation},
};

pub static DISPLAY: Mutex<CriticalSectionRawMutex, RefCell<Option<M5Display<'static>>>> =
    Mutex::new(RefCell::new(None));

pub type M5Display<'a> = Display<
    SpiInterface<
        'a,
        embedded_hal_bus::spi::ExclusiveDevice<
            Spi<'a, Blocking>,
            Output<'a>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'a>,
    >,
    mipidsi::models::ST7789,
    Output<'a>,
>;

pub fn stick_display<'a>(
    dc: GPIO14<'static>,
    rst: GPIO12<'static>,
    spi: SPI2<'static>,
    sck: GPIO13<'static>,
    mosi: GPIO15<'static>,
    cs: GPIO5<'static>,
) -> M5Display<'a> {
    let output_config = OutputConfig::default();
    let mut delay = Delay::new();

    let dc = Output::new(dc, Level::Low, output_config);

    let mut rst = Output::new(rst, Level::Low, output_config);
    rst.set_high();

    let spi = Spi::new(spi, Config::default().with_frequency(Rate::from_mhz(40)))
        .unwrap()
        .with_sck(sck)
        .with_mosi(mosi);

    let cs_output = Output::new(cs, Level::High, output_config);
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_output).unwrap();

    let buffer = Box::leak(Box::new([0_u8; 512]));
    let di = SpiInterface::new(spi_device, dc, buffer);

    let display = mipidsi::Builder::new(mipidsi::models::ST7789, di)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(ColorInversion::Inverted)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    display
}
