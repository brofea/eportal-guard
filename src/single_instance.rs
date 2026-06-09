use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// 用锁文件实现单实例：后启动的进程读取旧实例端口并拉起 Web 控制台。
pub struct SingleInstance {
    _file: File,
    lock_path: PathBuf,
}

impl SingleInstance {
    pub fn acquire(lock_path: &Path, web_port: u16) -> io::Result<Self> {
        // create_new 保证锁文件创建是原子的，避免两个实例同时启动时竞态。
        match open_new_lock(lock_path, web_port) {
            Ok(file) => {
                return Ok(Self {
                    _file: file,
                    lock_path: lock_path.to_path_buf(),
                });
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }

        // 如果锁文件对应的 PID 已不存在，视为上次异常退出留下的陈旧锁。
        let stale = fs::read_to_string(lock_path)
            .ok()
            .and_then(|s| parse_pid(&s))
            .map(|pid| !process_exists(pid))
            .unwrap_or(true);

        if stale {
            let _ = fs::remove_file(lock_path);
            let file = open_new_lock(lock_path, web_port)?;
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

    pub fn read_web_port(lock_path: &Path) -> Option<u16> {
        // 第二个实例依赖这里读取正在运行实例的 Web 端口。
        let text = fs::read_to_string(lock_path).ok()?;
        for line in text.lines() {
            let line = line.trim();
            let Some(port) = line.strip_prefix("web_port=") else {
                continue;
            };
            if let Ok(port) = port.parse::<u16>() {
                if port > 0 {
                    return Some(port);
                }
            }
        }
        None
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

fn open_new_lock(lock_path: &Path, web_port: u16) -> io::Result<File> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    // 写入 PID 和端口；parse_pid 仍兼容旧版本只写 PID 的锁文件。
    let text = format!("pid={}\nweb_port={}\n", std::process::id(), web_port);
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(file)
}

fn parse_pid(text: &str) -> Option<u32> {
    // 兼容两种格式：旧的 “12345” 和新的 “pid=12345”。
    let first_line = text.lines().next()?.trim();
    first_line
        .strip_prefix("pid=")
        .unwrap_or(first_line)
        .parse::<u32>()
        .ok()
}

fn process_exists(pid: u32) -> bool {
    // 不直接发送信号探测，避免权限差异；用系统命令查询更保守。
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
