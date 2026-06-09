use std::process::Command;

pub fn open_url(url: &str) -> bool {
    // 用各平台的系统默认方式打开 URL，保持 App 不绑定特定浏览器。
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", "start", "", url])
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
    let normalized = normalize_command(command);

    // cURL 登录命令本质是用户复制来的 shell 命令，按平台交给系统 shell 执行。
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", &normalized])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Command::new("sh")
            .arg("-c")
            .arg(&normalized)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
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

#[cfg(test)]
mod tests {
    use super::normalize_command;

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
}
