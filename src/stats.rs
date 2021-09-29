use esp_idf_sys::EspMutex;
use mutex_trait::Mutex;
use std::fmt::Write;
use std::time::Instant;

static mut STATS: EspMutex<Vec<(&'static str, f32)>> = EspMutex::new(Vec::new());

pub fn store(name: &'static str, value: f32) {
    unsafe {
        STATS.lock(|stats| {
            for (k, v) in stats.iter_mut() {
                if *k == name {
                    *v = value;
                    return;
                }
            }

            stats.push((name, value));
        })
    }
}

pub fn render(start: Instant) -> String {
    let mut s = String::new();

    unsafe {
        STATS.lock(|stats| {
            for (k, v) in stats {
                writeln!(&mut s, "{}{{version=\"rust\"}} {}", k, v).unwrap();
            }
        });
    }
    writeln!(&mut s, "uptime_seconds {}", start.elapsed().as_secs_f32()).unwrap();

    s
}
