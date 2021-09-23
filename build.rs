use std::path::PathBuf;

use embuild::{
    self, bingen,
    build::{CfgArgs, LinkArgs},
    cargo, symgen,
};

// Necessary because of this issue: https://github.com/rust-lang/cargo/issues/9641
fn main() -> anyhow::Result<()> {
    dbg!();
        panic!("20");
    LinkArgs::output_propagated("ESP_IDF")?;
        panic!("20");
    dbg!();

        panic!("20");
    let cfg = CfgArgs::try_from_env("ESP_IDF")?;
        panic!("20");
    dbg!();

        panic!("20");
    if cfg.get("esp32s2").is_some() {
        let ulp_elf = PathBuf::from("ulp").join("esp32-rust");
    dbg!();
        symgen::run(dbg!(&ulp_elf), 0x5000_0000)?; // This is where the RTC Slow Mem is mapped within the ESP32-S2 memory space
    dbg!();
        bingen::run(&ulp_elf)?;
    dbg!();

    dbg!();
        cargo::track_file(ulp_elf);
    dbg!();
    }
    dbg!();

    cfg.output();
    dbg!();

    Ok(())
}
