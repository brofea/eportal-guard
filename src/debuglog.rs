use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

// 调试阶段默认输出到终端，同时也写入配置目录下的 debug.log。
static CONSOLE_ENABLED: AtomicBool = AtomicBool::new(true);
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_console_enabled(enabled: bool) {
    CONSOLE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn log(component: &str, message: &str) {
    // 日志路径延迟初始化，避免模块初始化阶段就依赖 HOME / APPDATA。
    let path = LOG_PATH.get_or_init(|| {
        let path = crate::paths::app_config_dir().join("debug.log");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        path
    });

    let ts = short_local_time();
    let pid = std::process::id();
    let line = format!("[{}][pid:{}][{}] {}\n", ts, pid, component, message);

    if CONSOLE_ENABLED.load(Ordering::Relaxed) {
        eprint!("{}", line);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn short_local_time() -> String {
    let now = local_time_parts().unwrap_or_default();
    format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second)
}

#[derive(Default)]
struct TimeParts {
    hour: u16,
    minute: u16,
    second: u16,
}

#[cfg(unix)]
fn local_time_parts() -> Option<TimeParts> {
    use std::ffi::{c_char, c_int, c_long};
    use std::ptr;

    #[repr(C)]
    struct Tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        tm_gmtoff: c_long,
        tm_zone: *const c_char,
    }

    unsafe extern "C" {
        fn time(tloc: *mut c_long) -> c_long;
        fn localtime_r(timep: *const c_long, result: *mut Tm) -> *mut Tm;
    }

    let mut timestamp = 0 as c_long;
    let timestamp = unsafe { time(&mut timestamp) };
    if timestamp < 0 {
        return None;
    }

    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ptr::null(),
    };
    let ok = unsafe { localtime_r(&timestamp, &mut tm) };
    if ok.is_null() {
        return None;
    }

    Some(TimeParts {
        hour: tm.tm_hour as u16,
        minute: tm.tm_min as u16,
        second: tm.tm_sec as u16,
    })
}

#[cfg(target_os = "windows")]
fn local_time_parts() -> Option<TimeParts> {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millisecond: u16,
    }

    unsafe extern "system" {
        fn GetLocalTime(system_time: *mut SystemTime);
    }

    let mut system_time = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        millisecond: 0,
    };
    unsafe {
        GetLocalTime(&mut system_time);
    }

    Some(TimeParts {
        hour: system_time.hour,
        minute: system_time.minute,
        second: system_time.second,
    })
}
