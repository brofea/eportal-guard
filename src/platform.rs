use std::process::Command;

pub fn open_url(url: &str) -> bool {
    // 用各平台的系统默认方式打开 URL，保持 App 不绑定特定浏览器。
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        return command.status().map(|s| s.success()).unwrap_or(false);
    }

    #[cfg(target_os = "macos")]
    {
        return Command::new("open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    #[cfg(target_os = "linux")]
    {
        return Command::new("xdg-open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    #[allow(unreachable_code)]
    false
}
