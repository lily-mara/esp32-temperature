use std::fmt::Display;
use std::time::Instant;
use std::{fmt::Write, sync::Arc, thread, time::Duration};

use crate::mdns::{EspMdns, MdnsService};
use anyhow::{bail, Result};
use log::info;
use mutex_trait::Mutex;

use embedded_svc::httpd::registry::Registry;
use embedded_svc::wifi::ClientConfiguration;
use embedded_svc::wifi::Configuration;
use embedded_svc::wifi::Status;
use embedded_svc::wifi::{ClientConnectionStatus, ClientIpStatus, ClientStatus, Wifi};

use esp_idf_svc::httpd::Server;
use esp_idf_svc::httpd::ServerRegistry;
use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::wifi::EspWifi;
use esp_idf_sys::EspMutex;

use esp32_hal::analog::adc::ADC;
use esp32_hal::analog::config::Adc1Config;
use esp32_hal::analog::config::Attenuation;
use esp32_hal::analog::SensExt;
use esp32_hal::analog::ADC1;
use esp32_hal::gpio::*;
use esp32_hal::gpio::{Analog, GpioExt};

mod mac;
mod mdns;

const SSID: &str = env!("WIFI_SSID");
const PASS: &str = env!("WIFI_PASS");

struct Stats {
    start: Instant,
    battery_volts: Option<f32>,
    temperature: Option<f32>,
    raw_adc: Option<u16>,
}

static mut STATS: EspMutex<Option<Stats>> = EspMutex::new(None);

fn battery(adc1: &mut ADC<ADC1>, pin: &mut Gpio34<Analog>) -> Result<f32> {
    let reading: u16 = adc1.read(pin).map_err(|e| anyhow::format_err!("{:?}", e))?;

    Ok((reading as f32 / 4095.0) * 2.0 * 3.3 * 1.1)
}

fn temperature(adc1: &mut ADC<ADC1>, pin: &mut Gpio33<Analog>) -> Result<(u16, f32)> {
    let raw: u16 = dbg!(adc1.read(pin).map_err(|e| anyhow::format_err!("{:?}", e))?);
    let voltage = (raw as f32 * 5.0) / 4096.0;

    let temperature_c = ((voltage - 0.5) * 100.0) / 2.0;

    Ok((raw, temperature_c))
}

fn main() -> Result<()> {
    let start = Instant::now();
    let periphs = esp32::Peripherals::take().unwrap();
    let gpio = periphs.GPIO.split();
    let sensors = periphs.SENS.split();

    let mut battery_pin = gpio.gpio34.into_analog();
    let mut temperature_pin = gpio.gpio33.into_analog();

    let mut adc1_config = Adc1Config::new();
    adc1_config.enable_pin(&mut battery_pin, Attenuation::Attenuation11dB);
    adc1_config.enable_pin(&mut temperature_pin, Attenuation::Attenuation11dB);

    let mut adc1 = ADC::adc1(sensors.adc1, adc1_config).unwrap();

    unsafe {
        STATS.lock(|s| {
            *s = Some(Stats {
                start,
                battery_volts: None,
                temperature: None,
                raw_adc: None,
            });
        });
    }

    let netif = Arc::new(EspNetifStack::new()?);
    let sys_loop = Arc::new(EspSysLoopStack::new()?);
    let nvs = Arc::new(EspDefaultNvs::new()?);

    let _wifi = wifi(netif.clone(), sys_loop.clone(), nvs.clone())?;
    let _http = httpd()?;
    let _mdns = mdns()?;

    loop {
        unsafe {
            STATS.lock(|s| {
                let s = s.as_mut().unwrap();

                if let Ok(v) = battery(&mut adc1, &mut battery_pin) {
                    s.battery_volts = Some(v);
                }

                if let Ok((raw, t)) = temperature(&mut adc1, &mut temperature_pin) {
                    s.temperature = Some(t);
                    s.raw_adc = Some(raw);
                }
            });
        }

        thread::sleep(Duration::from_secs(5));
    }
}

fn mdns() -> Result<EspMdns> {
    let mut mdns = EspMdns::new()?;

    let mac = mac::efuse_mac()?;

    let mut server = mdns.server(format!("ESP_{:X}", mac))?;

    server.add_service(MdnsService {
        service_type: "_http".into(),
        protocol: "_tcp".into(),
        port: 80,
        ..Default::default()
    })?;
    server.add_service(MdnsService {
        service_type: "_prometheus-http".into(),
        protocol: "_tcp".into(),
        port: 80,
        ..Default::default()
    })?;

    Ok(mdns)
}

fn render_stats() -> String {
    let mut s = String::new();

    unsafe {
        STATS.lock(|stats| {
            let stats = stats.as_ref().unwrap();

            stat(&mut s, "uptime_seconds", stats.start.elapsed().as_secs());
            stat_opt(&mut s, "voltage_volts", stats.battery_volts);
            stat_opt(&mut s, "temperature_celcius", stats.temperature);
            stat_opt(&mut s, "raw_adc", stats.raw_adc);
        });
    }

    s
}

fn stat<T>(s: &mut String, name: &str, val: T)
where
    T: Display,
{
    writeln!(s, "{} {}", name, val).unwrap();
}

fn stat_opt<T>(s: &mut String, name: &str, val: Option<T>)
where
    T: Display,
{
    if let Some(x) = val {
        stat(s, name, x);
    }
}

fn httpd() -> Result<Server> {
    let server = ServerRegistry::new()
        .at("/")
        .get(move |_| Ok("this server serves prometheus-compatible metrics at /metrics".into()))?
        .at("/metrics")
        .get(move |_| Ok(render_stats().into()))?;

    server.start(&Default::default())
}

fn wifi(
    netif: Arc<EspNetifStack>,
    sys_loop: Arc<EspSysLoopStack>,
    nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif, sys_loop, nvs)?);

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASS.into(),
        ..Default::default()
    }))?;

    info!("Wifi configuration set, about to get status");

    match wifi.get_status() {
        Status(
            ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(_))),
            _,
        ) => {
            info!("Wifi connected");
        }
        status => bail!("Unexpected Wifi status: {:?}", status),
    }

    Ok(wifi)
}
