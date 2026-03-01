use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct SingleInstance {
    _file: File,
    lock_path: PathBuf,
}

impl SingleInstance {
    pub fn acquire(lock_path: &Path) -> io::Result<Self> {
        match open_new_lock(lock_path) {
            Ok(file) => {
                return Ok(Self {
                    _file: file,
                    lock_path: lock_path.to_path_buf(),
                });
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }

        let stale = fs::read_to_string(lock_path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .map(|pid| !process_exists(pid))
            .unwrap_or(true);

        if stale {
            let _ = fs::remove_file(lock_path);
            let file = open_new_lock(lock_path)?;
            return Ok(Self {
                _file: file,
                lock_path: lock_path.to_path_buf(),
            });
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "instance already running",
        ))
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

fn open_new_lock(lock_path: &Path) -> io::Result<File> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    let pid = std::process::id().to_string();
    file.write_all(pid.as_bytes())?;
    file.flush()?;
    Ok(file)
}

fn process_exists(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        let filter = format!("PID eq {}", pid);
        let output = Command::new("tasklist")
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output();
        let Ok(output) = output else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
        return !text.contains("no tasks") && text.contains(&format!(",\"{}\"", pid));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps").args(["-p", &pid.to_string()]).output();
        let Ok(output) = output else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        return text.lines().count() > 1;
    }
}
