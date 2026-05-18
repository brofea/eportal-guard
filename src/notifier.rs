#[cfg(target_os = "macos")]
pub fn notify(summary: &str, body: &str) {
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
        Ok(s) => crate::debuglog::log(
            "notifier",
            &format!("notification command exited with status: {}", s),
        ),
        Err(e) => crate::debuglog::log("notifier", &format!("notification failed: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', " ")
        .replace('\n', " ")
}

#[cfg(not(target_os = "macos"))]
pub fn notify(summary: &str, body: &str) {
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
        Ok(Err(e)) => crate::debuglog::log("notifier", &format!("notification failed: {}", e)),
        Err(_) => crate::debuglog::log("notifier", "notification panicked"),
    }
}
