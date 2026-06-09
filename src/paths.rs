use std::env;
use std::path::PathBuf;

// 所有运行时文件统一放在系统约定的用户配置目录下，避免污染安装目录。
pub const APP_DIR_NAME: &str = "eportal-guard";
#[cfg(target_os = "windows")]
pub const APP_RUN_KEY_NAME: &str = "ePortalGuard";

pub fn app_config_dir() -> PathBuf {
    // Windows/macOS/Linux 分别遵守各自的用户配置目录习惯。
    #[cfg(target_os = "windows")]
    {
        if let Ok(base) = env::var("APPDATA") {
            return PathBuf::from(base).join(APP_DIR_NAME);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_DIR_NAME);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join(APP_DIR_NAME);
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(".config").join(APP_DIR_NAME);
        }
    }

    PathBuf::from(".").join(APP_DIR_NAME)
}

pub fn config_path() -> PathBuf {
    app_config_dir().join("config.toml")
}

pub fn curl_path() -> PathBuf {
    app_config_dir().join("curl.txt")
}

pub fn lock_path() -> PathBuf {
    app_config_dir().join("app.lock")
}
