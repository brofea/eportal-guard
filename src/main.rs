#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod autostart;
mod config;
mod debuglog;
mod network;
mod notifier;
mod paths;
mod platform;
mod single_instance;
mod web;

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use config::{ensure_files, ensure_parent_dir};
use web::SharedState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .any(|arg| arg == "-help" || arg == "--help" || arg == "-h")
    {
        print_help();
        return;
    }

    debuglog::set_console_enabled(true);

    debuglog::log("main", "process start");
    if let Err(e) = run() {
        debuglog::log("main", &format!("fatal error: {}", e));
        notifier::notify("ePortal Guard", &format!("启动失败: {}", e));
    }
    debuglog::log("main", "process end");
}

fn print_help() {
    println!(
        "ePortal Guard 参数列表:\n  -help, --help, -h      显示参数说明\n\n日志行为:\n  默认同时输出到终端和 debug.log，便于调试。"
    );
}

fn run() -> Result<(), String> {
    debuglog::log("core", "initializing core process");
    let config_path = paths::config_path();
    let curl_path = paths::curl_path();
    let lock_path = paths::lock_path();
    ensure_parent_dir(&config_path).map_err(|e| e.to_string())?;
    ensure_files(&config_path, &curl_path).map_err(|e| e.to_string())?;

    let cfg = config::load_config(&config_path);
    let _lock = match single_instance::SingleInstance::acquire(&lock_path, cfg.web_port) {
        Ok(v) => v,
        Err(_) => {
            debuglog::log("core", "single-instance acquire failed");
            let port =
                single_instance::SingleInstance::read_web_port(&lock_path).unwrap_or(cfg.web_port);
            open_web_panel(port, "existing instance");
            return Ok(());
        }
    };

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let running = Arc::new(AtomicBool::new(true));

    debuglog::log(
        "core",
        &format!(
            "web startup on 127.0.0.1:{} | monitor interval={}s",
            cfg.web_port, cfg.ping_interval_secs
        ),
    );
    web::start_web_server(
        Arc::clone(&shared_state),
        Arc::clone(&running),
        config_path.clone(),
        curl_path.clone(),
        exe_path.clone(),
        cfg.web_port,
    );

    if config::read_curl(&curl_path)
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
        open_web_panel(cfg.web_port, "empty curl");
    }

    start_monitor(
        Arc::clone(&running),
        Arc::clone(&shared_state),
        config_path.clone(),
        curl_path.clone(),
    );

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(300));
    }

    Ok(())
}

fn open_web_panel(port: u16, reason: &str) {
    let url = format!("http://127.0.0.1:{}/", port);
    debuglog::log("core", &format!("open web panel: {} | {}", url, reason));
    wait_for_web_panel(port);
    if !platform::open_url(&url) {
        debuglog::log("core", &format!("open web panel failed: {}", url));
    }
}

fn wait_for_web_panel(port: u16) {
    let deadline = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn start_monitor(
    running: Arc<AtomicBool>,
    shared_state: Arc<Mutex<SharedState>>,
    config_path: std::path::PathBuf,
    curl_path: std::path::PathBuf,
) {
    thread::spawn(move || {
        let mut previous_cfg = config::load_config(&config_path);
        let mut login_failure_count: u32 = 0;
        let mut login_failure_notified = false;

        while running.load(Ordering::SeqCst) {
            let cfg = config::load_config(&config_path);
            if cfg.ping_interval_secs != previous_cfg.ping_interval_secs
                || cfg.ping_host != previous_cfg.ping_host
                || cfg.web_port != previous_cfg.web_port
            {
                notifier::notify("ePortal Guard", "配置更新");
                previous_cfg = cfg.clone();
            }

            let curl_cmd = config::read_curl(&curl_path).unwrap_or_default();
            debuglog::log(
                "monitor",
                &format!(
                    "tick | curl_configured={} | interval={}s",
                    !curl_cmd.trim().is_empty(),
                    cfg.ping_interval_secs.max(1)
                ),
            );

            if curl_cmd.trim().is_empty() {
                debuglog::log("monitor", "skip: curl is empty");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "未配置 cURL，等待配置");
                sleep_monitor_interval(&cfg);
                continue;
            }

            let intranet_connected = network::has_private_ip();
            debuglog::log(
                "monitor",
                &format!(
                    "intranet={}",
                    if intranet_connected { "ok" } else { "down" }
                ),
            );
            if !intranet_connected {
                debuglog::log("monitor", "skip: intranet is not connected");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "未连接内网，跳过登录");
                sleep_monitor_interval(&cfg);
                continue;
            }

            let probe = network::internet_probe();
            debuglog::log(
                "monitor",
                &format!(
                    "internet={} | miui={} {}ms | alidns={} {}ms",
                    if probe.ok { "ok" } else { "down" },
                    if probe.miui.ok { "ok" } else { "fail" },
                    probe.miui.elapsed_ms,
                    if probe.alidns.ok { "ok" } else { "fail" },
                    probe.alidns.elapsed_ms,
                ),
            );

            if probe.ok {
                debuglog::log("monitor", "internet available, skip login");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "网络正常");
            } else if !network::curl_exists() {
                register_login_failure(
                    &shared_state,
                    &mut login_failure_count,
                    &mut login_failure_notified,
                    "未检测到系统 curl 命令",
                );
            } else {
                debuglog::log("monitor", "internet unavailable, running login curl");
                let login_ok = platform::shell_run(&curl_cmd);
                debuglog::log(
                    "monitor",
                    &format!("login curl result={}", if login_ok { "ok" } else { "fail" }),
                );

                if login_ok {
                    reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                    set_state(&shared_state, "自动登录成功");
                } else {
                    register_login_failure(
                        &shared_state,
                        &mut login_failure_count,
                        &mut login_failure_notified,
                        "自动登录失败",
                    );
                }
            }

            sleep_monitor_interval(&cfg);
        }
    });
}

fn sleep_monitor_interval(cfg: &config::AppConfig) {
    thread::sleep(Duration::from_secs(cfg.ping_interval_secs.max(1)));
}

fn reset_login_failures(count: &mut u32, notified: &mut bool) {
    *count = 0;
    *notified = false;
}

fn register_login_failure(
    shared_state: &Arc<Mutex<SharedState>>,
    count: &mut u32,
    notified: &mut bool,
    status: &str,
) {
    *count = count.saturating_add(1);
    debuglog::log(
        "monitor",
        &format!("login failure count={} | {}", count, status),
    );

    if *count >= 10 {
        set_state(shared_state, "无法登陆");
        if !*notified {
            notifier::notify("ePortal Guard", "无法登陆");
            *notified = true;
        }
    } else {
        set_state(shared_state, status);
    }
}

fn set_state(shared_state: &Arc<Mutex<SharedState>>, status: &str) {
    if let Ok(mut s) = shared_state.lock() {
        if s.status_text != status {
            debuglog::log("state", &format!("{} -> {}", s.status_text, status));
            s.status_text.clear();
            s.status_text.push_str(status);
        }
    }
}
