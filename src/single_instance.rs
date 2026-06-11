use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
    // 用系统 API 探测进程，避免启动外部命令导致窗口闪烁。
    #[cfg(target_os = "windows")]
    {
        type Handle = *mut core::ffi::c_void;
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const STILL_ACTIVE: u32 = 259;

        #[link(name = "Kernel32")]
        unsafe extern "system" {
            fn OpenProcess(
                dw_desired_access: u32,
                b_inherit_handle: i32,
                dw_process_id: u32,
            ) -> Handle;
            fn GetExitCodeProcess(h_process: Handle, lp_exit_code: *mut u32) -> i32;
            fn CloseHandle(h_object: Handle) -> i32;
        }

        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0u32;
        let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) != 0 };
        unsafe {
            CloseHandle(handle);
        }
        return ok && exit_code == STILL_ACTIVE;
    }

    #[cfg(not(target_os = "windows"))]
    {
        unsafe extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }

        if pid > i32::MAX as u32 {
            return false;
        }
        unsafe { kill(pid as i32, 0) == 0 }
    }
}
