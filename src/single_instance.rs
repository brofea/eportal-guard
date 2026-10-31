use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// 用锁文件实现单实例：后启动的进程读取旧实例端口并拉起 Web 控制台。
pub struct SingleInstance {
    _file: File,
    lock_path: PathBuf,
}

impl SingleInstance {
    #[cfg(unix)]
    pub fn acquire(lock_path: &Path, web_port: u16) -> io::Result<Self> {
        // Unix 用内核 flock：进程退出（包括崩溃）时锁自动释放。
        // 旧的 PID 探测在重启后容易撞上被复用的 PID，误判“已有实例”导致开机自启起不来。
        use std::os::unix::io::AsRawFd;

        const LOCK_EX: i32 = 2;
        const LOCK_NB: i32 = 4;
        unsafe extern "C" {
            fn flock(fd: i32, operation: i32) -> i32;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "instance already running",
            ));
        }

        // 拿到锁后才覆盖内容，第二个实例仍可从这里读到 Web 端口。
        file.set_len(0)?;
        let text = format!("pid={}\nweb_port={}\n", std::process::id(), web_port);
        file.write_all(text.as_bytes())?;
        file.flush()?;
        Ok(Self {
            _file: file,
            lock_path: lock_path.to_path_buf(),
        })
    }

    #[cfg(not(unix))]
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
        // Unix 下锁随文件描述符释放，保留锁文件，避免删除时与新实例竞争。
        if cfg!(not(unix)) {
            let _ = std::fs::remove_file(&self.lock_path);
        }
    }
}

#[cfg(not(unix))]
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

#[cfg(not(unix))]
fn parse_pid(text: &str) -> Option<u32> {
    // 兼容两种格式：旧的 “12345” 和新的 “pid=12345”。
    let first_line = text.lines().next()?.trim();
    first_line
        .strip_prefix("pid=")
        .unwrap_or(first_line)
        .parse::<u32>()
        .ok()
}

#[cfg(not(unix))]
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

#[cfg(all(test, unix))]
mod tests {
    use super::SingleInstance;

    #[test]
    fn second_acquire_fails_and_lock_frees_on_drop() {
        let path = std::env::temp_dir().join(format!("eg-lock-test-{}", std::process::id()));
        let first = SingleInstance::acquire(&path, 18888).unwrap();
        assert!(SingleInstance::acquire(&path, 18889).is_err());
        assert_eq!(SingleInstance::read_web_port(&path), Some(18888));
        drop(first);
        // 模拟崩溃后残留的锁文件：内容还在，但锁已释放，新实例必须能启动。
        let second = SingleInstance::acquire(&path, 18890).unwrap();
        assert_eq!(SingleInstance::read_web_port(&path), Some(18890));
        drop(second);
        let _ = std::fs::remove_file(&path);
    }
}
