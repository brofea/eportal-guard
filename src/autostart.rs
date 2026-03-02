#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::process::Command;

#[cfg(target_os = "windows")]
use crate::paths::APP_RUN_KEY_NAME;

pub fn is_enabled(_exe_path: &std::path::Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                APP_RUN_KEY_NAME,
            ])
            .output();
        return output.map(|o| o.status.success()).unwrap_or(false);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_login_item_exists(_exe_path).unwrap_or(false);
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
        if enabled {
            let value = format!("\"{}\"", exe_path.to_string_lossy());
            let ok = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    APP_RUN_KEY_NAME,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &value,
                    "/f",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Ok(());
            }
            return Err("写入 Windows Run 注册表失败".to_string());
        }

        let ok = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                APP_RUN_KEY_NAME,
                "/f",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
        return Err("删除 Windows Run 注册表失败".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        if enabled {
            macos_remove_login_item(exe_path).ok();
            macos_add_login_item(exe_path).map_err(|e| format!("添加 macOS 登录项失败: {}", e))?;
            return Ok(());
        }

        macos_remove_login_item(exe_path).map_err(|e| format!("移除 macOS 登录项失败: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
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
fn macos_login_item_exists(exe_path: &std::path::Path) -> Result<bool, String> {
    let expected = exe_path.to_string_lossy().to_string();
    let paths = macos_login_item_paths()?;
    Ok(paths.iter().any(|p| p == &expected))
}

#[cfg(target_os = "macos")]
fn macos_add_login_item(exe_path: &std::path::Path) -> Result<(), String> {
    let path = escape_applescript(&exe_path.to_string_lossy());
    let file_name = exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("eportal_guard");
    let name = escape_applescript(file_name);
    let script = format!(
        "tell application \"System Events\" to make login item at end with properties {{name:\"{}\", path:\"{}\", hidden:false}}",
        name, path
    );
    run_osascript(&script)?;
    if macos_login_item_exists(exe_path).unwrap_or(false) {
        Ok(())
    } else {
        Err("登录项添加后状态校验失败".to_string())
    }
}

#[cfg(target_os = "macos")]
fn macos_remove_login_item(exe_path: &std::path::Path) -> Result<(), String> {
    let path = escape_applescript(&exe_path.to_string_lossy());
    let script = format!(
        "tell application \"System Events\" to delete (every login item whose path is \"{}\")",
        path
    );
    run_osascript(&script).map(|_| ())
}

#[cfg(target_os = "macos")]
fn macos_login_item_paths() -> Result<Vec<String>, String> {
    let out = run_osascript("tell application \"System Events\" to get the path of every login item")?;
    let paths = out
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    Ok(paths)
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    Err(String::from_utf8_lossy(&output.stderr).to_string())
}

#[cfg(target_os = "macos")]
fn escape_applescript(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "linux")]
fn desktop_entry_path() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("autostart")
            .join("eportal-guard.desktop");
    }
    PathBuf::from("./eportal-guard.desktop")
}
