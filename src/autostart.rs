#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use crate::paths::APP_RUN_KEY_NAME;

pub fn is_enabled(_exe_path: &std::path::Path) -> bool {
    // 三个平台的自启机制完全不同，因此在同一个入口里按平台分发。
    #[cfg(target_os = "windows")]
    {
        return windows_read_run_value(APP_RUN_KEY_NAME).is_some();
    }

    #[cfg(target_os = "macos")]
    {
        return macos_launch_agent_path().exists();
    }

    #[cfg(target_os = "linux")]
    {
        return desktop_entry_path().exists();
    }

    #[allow(unreachable_code)]
    {
        let _ = _exe_path;
        false
    }
}

pub fn set_enabled(exe_path: &std::path::Path, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Windows 使用当前用户 Run 注册表项，不需要管理员权限。
        if enabled {
            let value = format!("\"{}\"", exe_path.to_string_lossy());
            return windows_write_run_value(APP_RUN_KEY_NAME, &value);
        }

        return windows_delete_run_value(APP_RUN_KEY_NAME);
    }

    #[cfg(target_os = "macos")]
    {
        // macOS 使用 LaunchAgent plist，避免触发额外进程或权限弹窗。
        let plist = macos_launch_agent_path();
        if enabled {
            if let Some(parent) = plist.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let content = macos_launch_agent_plist(exe_path);
            std::fs::write(plist, content).map_err(|e| e.to_string())?;
            return Ok(());
        }

        if plist.exists() {
            std::fs::remove_file(plist).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // Linux 桌面环境使用 XDG autostart .desktop 文件。
        let desktop = desktop_entry_path();
        if enabled {
            if let Some(parent) = desktop.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=ePortal Guard\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                exe_path.to_string_lossy()
            );
            fs::write(desktop, content).map_err(|e| e.to_string())?;
            return Ok(());
        }

        if desktop.exists() {
            fs::remove_file(desktop).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        let _ = (exe_path, enabled);
        Err("当前系统暂不支持开机自启".to_string())
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join("Library")
            .join("LaunchAgents")
            .join("com.brofea.eportal-guard.plist");
    }
    std::path::PathBuf::from("./com.brofea.eportal-guard.plist")
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_plist(exe_path: &std::path::Path) -> String {
    let exe = xml_escape(&exe_path.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.brofea.eportal-guard</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        exe
    )
}

#[cfg(target_os = "macos")]
fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(target_os = "linux")]
fn desktop_entry_path() -> PathBuf {
    // 优先写入用户 HOME 下的 autostart，缺失 HOME 时退回当前目录便于测试。
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("autostart")
            .join("eportal-guard.desktop");
    }
    PathBuf::from("./eportal-guard.desktop")
}

#[cfg(target_os = "windows")]
fn windows_read_run_value(name: &str) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    type Hkey = *mut core::ffi::c_void;
    const HKEY_CURRENT_USER: Hkey = 0x80000001usize as Hkey;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_MORE_DATA: i32 = 234;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const REG_SZ: u32 = 1;

    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            h_key: Hkey,
            lp_sub_key: *const u16,
            ul_options: u32,
            sam_desired: u32,
            phk_result: *mut Hkey,
        ) -> i32;
        fn RegQueryValueExW(
            h_key: Hkey,
            lp_value_name: *const u16,
            lp_reserved: *mut u32,
            lp_type: *mut u32,
            lp_data: *mut u8,
            lpcb_data: *mut u32,
        ) -> i32;
        fn RegCloseKey(h_key: Hkey) -> i32;
    }

    fn wide(input: &str) -> Vec<u16> {
        OsStr::new(input).encode_wide().chain(Some(0)).collect()
    }

    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let mut key: Hkey = null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    let name = wide(name);
    let mut value_type = 0u32;
    let mut bytes = 0u32;
    let mut status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            null_mut(),
            &mut value_type,
            null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_MORE_DATA {
        status = ERROR_SUCCESS;
    }
    if status != ERROR_SUCCESS || value_type != REG_SZ || bytes == 0 {
        unsafe {
            RegCloseKey(key);
        }
        return None;
    }

    let mut buf = vec![0u16; (bytes as usize).div_ceil(std::mem::size_of::<u16>())];
    let status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            null_mut(),
            &mut value_type,
            buf.as_mut_ptr().cast::<u8>(),
            &mut bytes,
        )
    };
    unsafe {
        RegCloseKey(key);
    }
    if status != ERROR_SUCCESS || value_type != REG_SZ {
        return None;
    }

    while buf.last() == Some(&0) {
        buf.pop();
    }
    Some(String::from_utf16_lossy(&buf))
}

#[cfg(target_os = "windows")]
fn windows_write_run_value(name: &str, value: &str) -> Result<(), String> {
    windows_set_run_value(name, Some(value))
}

#[cfg(target_os = "windows")]
fn windows_delete_run_value(name: &str) -> Result<(), String> {
    windows_set_run_value(name, None)
}

#[cfg(target_os = "windows")]
fn windows_set_run_value(name: &str, value: Option<&str>) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};

    type Hkey = *mut core::ffi::c_void;
    const HKEY_CURRENT_USER: Hkey = 0x80000001usize as Hkey;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const ERROR_SUCCESS: i32 = 0;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_OPTION_NON_VOLATILE: u32 = 0;
    const REG_SZ: u32 = 1;

    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            h_key: Hkey,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *mut u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const core::ffi::c_void,
            phk_result: *mut Hkey,
            lpdw_disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            h_key: Hkey,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
        fn RegDeleteValueW(h_key: Hkey, lp_value_name: *const u16) -> i32;
        fn RegCloseKey(h_key: Hkey) -> i32;
    }

    fn wide(input: &str) -> Vec<u16> {
        OsStr::new(input).encode_wide().chain(Some(0)).collect()
    }

    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let mut key: Hkey = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("打开 Windows Run 注册表失败: {}", status));
    }

    let name = wide(name);
    let result = if let Some(value) = value {
        let value = wide(value);
        let bytes = value.len() * std::mem::size_of::<u16>();
        let status = unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast::<u8>(),
                bytes as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("写入 Windows Run 注册表失败: {}", status))
        }
    } else {
        let status = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!("删除 Windows Run 注册表失败: {}", status))
        }
    };

    unsafe {
        RegCloseKey(key);
    }
    result
}
