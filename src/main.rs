use std::fmt::Display;
use std::time::Instant;
use std::{fmt::Write, sync::Arc, thread, time::Duration};

use crate::mdns::{EspMdns, MdnsService};
use anyhow::{bail, Result};
use log::{error, info};
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
use esp_idf_sys::{esp, EspMutex};

use esp_idf_sys::{
    adc1_channel_t_ADC1_CHANNEL_4, adc1_config_channel_atten, adc1_config_width, adc1_get_raw,
    adc_atten_t_ADC_ATTEN_DB_11, adc_bits_width_t_ADC_WIDTH_BIT_12,
};

use dotenv_codegen::dotenv;

mod mac;
mod mdns;

const SSID: &str = dotenv!("WIFI_SSID");
const PASS: &str = dotenv!("WIFI_PASS");

static mut TEMPERATURE: EspMutex<f32> = EspMutex::new(0.0);

fn battery() -> Result<f32> {
    // let reading: u16 = adc1.read(pin).map_err(|e| anyhow::format_err!("{:?}", e))?;

    // Ok((reading as f32 / 4095.0) * 2.0 * 3.3 * 1.1)
    Ok(0.0)
}

fn temperature() -> Result<()> {
    let raw = unsafe { adc1_get_raw(adc1_channel_t_ADC1_CHANNEL_4) };

    if raw == -1 {
        bail!("ADC read error");
    }

    let voltage = (raw as f32 * 5.0) / 4096.0;

    let temperature_c = ((voltage - 0.5) * 100.0) / 2.0;

    unsafe {
        TEMPERATURE.lock(|l| *l = temperature_c);
    }

    Ok(())
}

fn main() -> Result<()> {
    let start = Instant::now();

    unsafe {
        adc1_config_channel_atten(adc1_channel_t_ADC1_CHANNEL_4, adc_atten_t_ADC_ATTEN_DB_11);
        esp!(adc1_config_width(adc_bits_width_t_ADC_WIDTH_BIT_12))?;
    }

    // let mut battery_pin = gpio.gpio34.into_analog();

    let netif = Arc::new(EspNetifStack::new()?);
    let sys_loop = Arc::new(EspSysLoopStack::new()?);
    let nvs = Arc::new(EspDefaultNvs::new()?);

    let _wifi = wifi(netif.clone(), sys_loop.clone(), nvs.clone())?;
    let _http = httpd(start)?;
    let _mdns = mdns()?;

    loop {
        if let Err(e) = temperature() {
            error!("Failed to read temperature: {}", e)
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

fn render_stats(start: Instant) -> String {
    let mut s = String::new();

    stat(&mut s, "uptime_seconds", start.elapsed().as_secs());

    unsafe {
        TEMPERATURE.lock(|t| {
            stat(&mut s, "temperature_celcius", *t);
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

fn httpd(start: Instant) -> Result<Server> {
    let server = ServerRegistry::new()
        .at("/")
        .get(move |_| Ok("this server serves prometheus-compatible metrics at /metrics".into()))?
        .at("/metrics")
        .get(move |_| Ok(render_stats(start).into()))?;

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
