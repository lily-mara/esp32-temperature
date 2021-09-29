# ESP-32 Temperature Sensor

This is a WiFi enabled, prometheus-compatible, mDNS broadcasting temperature
sensor setup for the ESP-32. It is designed so that you can compile once, then
drop the ESPs anywhere within WiFi range and data collection should begin
quickly.

## Requirements

- ESP fork of Rust (https://github.com/esp-rs/rust)
- ESP-ready Clang (https://kerkour.com/blog/compile-rust-for-esp32-xtensa-on-raspberry-pi-aarch64/)
- https://github.com/esp-rs/espflash
- https://github.com/esp-rs/espmonitor (optional, for debug monitoring)
- Prometheus (https://prometheus.io)
- Prometheus mDNS-SD (https://github.com/msiebuhr/prometheus-mdns-sd)

## Running

```
$ cargo espflash --release --monitor SERIAL-DEVICE
```

Where SERIAL-DEVICE is the path to the serial device where your ESP-32 is
prepared to be programmed.

You must also provide WiFi network information at compile time via environment
variables. You can either provide these via your shell, or write them into a
`.env` file at the root of this repository.

```
WIFI_SSID=MyHomeNet
WIFI_PASS=MySecretPassword
```
