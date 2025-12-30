// This is Claude slop, I have no idea what's happening
// https://github.com/m5stack/M5Unified/blob/master/src/utility/Power_Class.cpp#L1266

use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcPin},
    peripherals::{ADC1, GPIO38},
};

pub fn get_battery_level(
    adc: &mut Adc<'static, ADC1<'static>, Blocking>,
    pin: &mut AdcPin<GPIO38<'static>, ADC1<'static>>,
) -> u8 {
    let adc_value: u16 = nb::block!(adc.read_oneshot(pin)).unwrap();

    let adc_mv = {
        const COEFF_A: u32 = 52814;
        const COEFF_B: u32 = 142;
        ((COEFF_A * adc_value as u32 + 32768) / 65536 + COEFF_B) as u16
    };
    let battery_mv = adc_mv * 2;
    let level = ((battery_mv as i32 - 3300) * 100 / (4150 - 3350)).clamp(0, 100) as u8;

    log::info!(
        "Battery: raw={}, adc_mv={}, battery_mv={}, level={}%",
        adc_value,
        adc_mv,
        battery_mv,
        level
    );

    level
}
