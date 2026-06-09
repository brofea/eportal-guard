#[cfg(target_os = "macos")]
pub fn notify(summary: &str, body: &str) {
    // macOS 上使用 osascript，比 notify-rust 的原生通知路径更适合 .app 发布包场景。
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(body),
        escape_applescript(summary)
    );

    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => crate::debuglog::log("通知", &format!("系统通知命令退出状态异常: {}", s)),
        Err(e) => crate::debuglog::log("通知", &format!("发送系统通知失败: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript(input: &str) -> String {
    // 通知文案会拼进 AppleScript 字符串，必须转义引号和反斜杠。
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', " ")
        .replace('\n', " ")
}

#[cfg(not(target_os = "macos"))]
pub fn notify(summary: &str, body: &str) {
    // 非 macOS 保留 notify-rust，并用 catch_unwind 避免通知库 panic 影响主流程。
    let summary = summary.to_string();
    let body = body.to_string();
    let result = std::panic::catch_unwind(move || {
        notify_rust::Notification::new()
            .summary(&summary)
            .body(&body)
            .show()
    });

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => crate::debuglog::log("通知", &format!("发送系统通知失败: {}", e)),
        Err(_) => crate::debuglog::log("通知", "发送系统通知时发生 panic"),
    }
}
