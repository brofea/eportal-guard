use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn log(component: &str, message: &str) {
    let path = crate::paths::app_config_dir().join("debug.log");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ts = format!("{}.{:03}", now.as_secs(), now.subsec_millis());
    let pid = std::process::id();
    let line = format!("[{}][pid:{}][{}] {}\n", ts, pid, component, message);

    eprint!("{}", line);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
