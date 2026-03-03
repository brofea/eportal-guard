use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static CONSOLE_ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_console_enabled(enabled: bool) {
    CONSOLE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn log(component: &str, message: &str) {
    let path = LOG_PATH.get_or_init(|| {
        let path = crate::paths::app_config_dir().join("debug.log");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        path
    });

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ts = format!("{}.{:03}", now.as_secs(), now.subsec_millis());
    let pid = std::process::id();
    let line = format!("[{}][pid:{}][{}] {}\n", ts, pid, component, message);

    if CONSOLE_ENABLED.load(Ordering::Relaxed) {
        eprint!("{}", line);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
