use esp_idf_sys::{
    esp, mdns_free, mdns_hostname_set, mdns_init, mdns_service_add, mdns_txt_item_t, EspError,
    EspMutex, ESP_ERR_INVALID_STATE,
};
use mutex_trait::Mutex;
use std::ffi::CString;

pub struct EspMdns {}

pub struct EspMdnsServer {}

#[derive(Default)]
pub struct MdnsService {
    pub instance_name: Option<String>,
    pub service_type: String,
    pub protocol: String,
    pub port: u16,
    pub txt: Vec<(String, String)>,
}

static mut TAKEN: EspMutex<bool> = EspMutex::new(false);

impl EspMdns {
    pub fn new() -> Result<Self, EspError> {
        unsafe {
            TAKEN.lock(|taken| {
                if *taken {
                    Err(EspError::from(ESP_ERR_INVALID_STATE as i32).unwrap())
                } else {
                    let mdns = Self::init()?;

                    *taken = true;
                    Ok(mdns)
                }
            })
        }
    }

    fn init() -> Result<Self, EspError> {
        unsafe {
            esp!(mdns_init())?;
        }

        Ok(Self {})
    }

    pub fn server(&mut self, hostname: impl AsRef<str>) -> Result<EspMdnsServer, EspError> {
        let c_str = CString::new(hostname.as_ref()).unwrap();

        unsafe {
            esp!(mdns_hostname_set(c_str.as_ptr()))?;
        }
        Ok(EspMdnsServer {})
    }
}

impl Drop for EspMdns {
    fn drop(&mut self) {
        unsafe {
            TAKEN.lock(|taken| {
                mdns_free();

                *taken = false;
            });
        }
    }
}

impl EspMdnsServer {
    pub fn add_service(&mut self, service: MdnsService) -> Result<(), EspError> {
        let instance_name = service.instance_name.map(|s| CString::new(s).unwrap());
        let instance_name_ptr = match &instance_name {
            Some(c_str) => c_str.as_ptr(),
            None => std::ptr::null(),
        };
        let service_type = CString::new(service.service_type).unwrap();
        let proto = CString::new(service.protocol).unwrap();

        let txt_cstr_items: Vec<_> = service
            .txt
            .into_iter()
            .map(|(k, v)| (CString::new(k).unwrap(), CString::new(v).unwrap()))
            .collect();

        let mut txt_raw_items: Vec<_> = txt_cstr_items
            .iter()
            .map(|(k, v)| mdns_txt_item_t {
                key: k.as_ptr(),
                value: v.as_ptr(),
            })
            .collect();

        unsafe {
            esp!(mdns_service_add(
                instance_name_ptr,
                service_type.as_ptr(),
                proto.as_ptr(),
                service.port,
                txt_raw_items.as_mut_ptr(),
                txt_raw_items.len() as _,
            ))?;
        }

        Ok(())
    }
}
