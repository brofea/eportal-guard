use std::env;
use std::fs;
use std::path::PathBuf;
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
        return launch_agent_path().exists();
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
        let plist = launch_agent_path();
        if enabled {
            if let Some(parent) = plist.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let content = format!(
                r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\"><dict>
<key>Label</key><string>com.eportal.guard</string>
<key>ProgramArguments</key><array><string>{}</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
"#,
                exe_path.to_string_lossy()
            );
            fs::write(&plist, content).map_err(|e| e.to_string())?;
            let _ = Command::new("launchctl").args(["load", "-w"]).arg(&plist).status();
            return Ok(());
        }

        if plist.exists() {
            let _ = Command::new("launchctl").args(["unload", "-w"]).arg(&plist).status();
            fs::remove_file(plist).map_err(|e| e.to_string())?;
        }
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
fn launch_agent_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.eportal.guard.plist")
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
