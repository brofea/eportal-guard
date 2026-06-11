#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn open_url(url: &str) -> bool {
    // 用各平台的系统默认方式打开 URL，保持 App 不绑定特定浏览器。
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        return hide_window(&mut command)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
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

pub fn shell_run(command: &str) -> bool {
    shell_run_capture(command)
}

pub fn shell_run_capture(command: &str) -> bool {
    // cURL 登录命令本质是用户复制来的 shell 命令，按平台交给系统 shell 执行。
    #[cfg(target_os = "windows")]
    {
        let normalized = normalize_windows_powershell_command(command);
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &normalized,
        ]);
        return command_output(hide_window(&mut command));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let normalized = normalize_command(command);
        return command_output(Command::new("sh").arg("-c").arg(&normalized));
    }
}

fn command_output(command: &mut Command) -> bool {
    command
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn hide_window(command: &mut Command) -> &mut Command {
    // Windows GUI 程序启动 cmd/curl 子进程时默认会闪出控制台窗口，这里显式隐藏。
    command.creation_flags(CREATE_NO_WINDOW)
}

fn normalize_command(input: &str) -> String {
    // 兼容浏览器开发者工具复制出的 BOM、CRLF、多行续行和命令提示符前缀。
    let mut text = input.trim_start_matches('\u{feff}').trim().to_string();
    if text.is_empty() {
        return text;
    }

    text = text.replace("\r\n", "\n").replace('\r', "\n");
    text = text.replace("\\\n", " ");

    let mut lines: Vec<String> = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.strip_prefix("$ ").unwrap_or(line).to_string())
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    let starts_with_curl = lines[0] == "curl" || lines[0].starts_with("curl ");
    if starts_with_curl {
        // 多行 cURL 通常只是为了可读性换行，这里压成一行交给 shell。
        return lines
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    }

    if lines.len() > 1 {
        lines.join("\n")
    } else {
        lines.pop().unwrap_or_default()
    }
}

#[cfg(any(target_os = "windows", test))]
fn normalize_windows_powershell_command(input: &str) -> String {
    let normalized = normalize_command(input);
    if normalized == "curl" {
        return "curl.exe".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("curl ") {
        return format!("curl.exe {}", rest);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{normalize_command, normalize_windows_powershell_command};

    #[test]
    fn normalize_multiline_curl_with_crlf() {
        let raw = "curl 'http://example.com' \\\r\n  -H 'A: B' \\\r\n  --data-raw 'x=1&y=2'\r\n";
        let got = normalize_command(raw);
        assert_eq!(
            got,
            "curl 'http://example.com' -H 'A: B' --data-raw 'x=1&y=2'"
        );
    }

    #[test]
    fn normalize_shell_prompt_prefix() {
        let raw = "$ curl https://example.com -I\n";
        let got = normalize_command(raw);
        assert_eq!(got, "curl https://example.com -I");
    }

    #[test]
    fn normalize_windows_powershell_uses_curl_exe() {
        let raw = "curl 'https://example.com' -H 'A: B'";
        let got = normalize_windows_powershell_command(raw);
        assert_eq!(got, "curl.exe 'https://example.com' -H 'A: B'");
    }
}
