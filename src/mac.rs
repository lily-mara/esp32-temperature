use esp_idf_sys::{esp, esp_efuse_mac_get_default, EspError};

pub fn efuse_mac() -> Result<u64, EspError> {
    let mut mac: u64 = 0;

    let mac_p = &mut mac as *mut u64 as *mut u8;

    unsafe {
        esp!(esp_efuse_mac_get_default(mac_p))?;
    }

    Ok(mac)
}
