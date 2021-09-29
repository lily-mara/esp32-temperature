use std::{thread, time::Duration};

use anyhow::Result;

// use esp32_hal::analog::adc::ADC;
// use esp32_hal::analog::config::Adc1Config;
// use esp32_hal::analog::config::Attenuation;
// use esp32_hal::analog::SensExt;
// use esp32_hal::gpio::GpioExt;
// use esp32_hal::gpio::*;
use esp_idf_sys::{
    adc1_channel_t_ADC1_CHANNEL_4, adc1_config_channel_atten, adc1_config_width, adc1_get_raw,
    adc_atten_t_ADC_ATTEN_DB_11, adc_bits_width_t_ADC_WIDTH_BIT_12, esp,
};

fn main() -> Result<()> {
    // Reference to the esp_idf_svc crate to setup app_main for us
    let _ = esp_idf_svc::sysloop::EspSysLoopStack::new();

    // let pins = esp_idf_hal::gpio::Pins::new();
    // let gpio = periphs.GPIO.split();
    // let sensors = periphs.SENS.split();

    unsafe {
        adc1_config_channel_atten(adc1_channel_t_ADC1_CHANNEL_4, adc_atten_t_ADC_ATTEN_DB_11);
        esp!(adc1_config_width(adc_bits_width_t_ADC_WIDTH_BIT_12))?;

        loop {
            let result = adc1_get_raw(adc1_channel_t_ADC1_CHANNEL_4);
            println!("{}", result);

            thread::sleep(Duration::from_millis(50));
        }
    }
}
