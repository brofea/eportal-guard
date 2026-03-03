use std::process::Command;

pub fn open_url(url: &str) -> bool {
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
