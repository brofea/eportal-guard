#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod autostart;
mod config;
mod debuglog;
mod network;
mod notifier;
mod paths;
mod platform;
mod single_instance;
mod tray;
mod web;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{io::Write, net::TcpStream, process::Command};

use config::{ensure_files, ensure_parent_dir};
use web::SharedState;

fn main() {
    debuglog::log("main", "process start");
    if std::env::args().any(|arg| arg == "--tray-host") {
        debuglog::log("main", "enter tray-host mode");
        if let Err(e) = run_tray_host() {
            debuglog::log("tray-host", &format!("fatal error: {}", e));
            notifier::notify("ePortal Guard Tray", &format!("托盘子进程启动失败: {}", e));
        }
        debuglog::log("main", "tray-host process end");
        return;
    }

    if let Err(e) = run() {
        debuglog::log("main", &format!("fatal error: {}", e));
        notifier::notify("ePortal Guard", &format!("启动失败: {}", e));
    }
    debuglog::log("main", "process end");
}

fn run() -> Result<(), String> {
    debuglog::log("core", "initializing core process");
    let config_path = paths::config_path();
    let curl_path = paths::curl_path();
    let lock_path = paths::lock_path();
    ensure_parent_dir(&config_path).map_err(|e| e.to_string())?;
    ensure_files(&config_path, &curl_path).map_err(|e| e.to_string())?;

    let _lock = match single_instance::SingleInstance::acquire(&lock_path) {
        Ok(v) => v,
        Err(_) => {
            debuglog::log("core", "single-instance acquire failed");
            return Err("检测到程序已在运行（单实例锁失败）".to_string());
        }
    };

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let running = Arc::new(AtomicBool::new(true));

    let cfg = config::load_config(&config_path);
    debuglog::log(
        "core",
        &format!(
            "web startup on 127.0.0.1:{} | ping host={} interval={}s",
            cfg.web_port, cfg.ping_host, cfg.ping_interval_secs
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

    start_monitor(
        Arc::clone(&running),
        Arc::clone(&shared_state),
        config_path.clone(),
        curl_path.clone(),
    );

    #[cfg(not(target_os = "macos"))]
    let on_exit = {
        let running_for_exit = Arc::clone(&running);
        Arc::new(move || {
            running_for_exit.store(false, Ordering::SeqCst);
        })
    };

    #[cfg(target_os = "macos")]
    debuglog::log("core", "starting macOS tray watchdog");
    #[cfg(target_os = "macos")]
    start_macos_tray_watchdog(Arc::clone(&running), Arc::clone(&shared_state), exe_path.clone());

    #[cfg(not(target_os = "macos"))]
    let tray = match tray::start_tray(cfg.web_port, on_exit) {
        Ok(v) => Some(v),
        Err(e) => {
            set_state(&shared_state, "托盘启动失败，已降级为 Web 模式", &e);
            None
        }
    };

    #[cfg(target_os = "macos")]
    let tray: Option<tray::TrayHandle> = None;

    while running.load(Ordering::SeqCst) {
        if let Some(tray) = &tray {
            tray.process_events();
        }
        thread::sleep(Duration::from_millis(300));
    }

    #[cfg(target_os = "macos")]
    debuglog::log("core", "stopping macOS tray sidecar");
    #[cfg(target_os = "macos")]
    stop_macos_tray_sidecar();

    Ok(())
}

fn run_tray_host() -> Result<(), String> {
    debuglog::log("tray-host", "startup begin");
    #[cfg(target_os = "macos")]
    init_macos_appkit();

    let config_path = paths::config_path();
    let cfg = config::load_config(&config_path);
    let port = cfg.web_port;
    debuglog::log("tray-host", &format!("tray host using web port {}", port));

    let running = Arc::new(AtomicBool::new(true));

    let running_for_exit = Arc::clone(&running);
    let on_exit = Arc::new(move || {
        running_for_exit.store(false, Ordering::SeqCst);
        let _ = http_post(port, "/quit", "");
        std::process::exit(0);
    });

    let tray = tray::start_tray(port, on_exit)
    .map_err(|e| format!("托盘初始化失败: {}", e))?;
    debuglog::log("tray-host", "tray created, entering event loop");

    #[cfg(target_os = "macos")]
    {
        tray.install_macos_event_handlers();
        run_macos_app_event_loop();
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        while running.load(Ordering::SeqCst) {
            tray.process_events();
            thread::sleep(Duration::from_millis(200));
        }
        return Ok(());
    }
}

#[cfg(target_os = "macos")]
fn init_macos_appkit() {
    debuglog::log("tray-host", "init macOS AppKit begin");
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        app.finishLaunching();
        let ok = app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);
        debuglog::log("tray-host", &format!("setActivationPolicy(Accessory) => {}", ok));

        app.activate();
        debuglog::log("tray-host", "app.activate() called");

        let policy = app.activationPolicy().0;
        let active = app.isActive();
        let running = app.isRunning();
        debuglog::log(
            "tray-host",
            &format!(
                "app state after init: policy={} active={} running={}",
                policy, active, running
            ),
        );
    } else {
        debuglog::log("tray-host", "MainThreadMarker unavailable");
    }
    debuglog::log("tray-host", "init macOS AppKit end");
}

#[cfg(target_os = "macos")]
fn run_macos_app_event_loop() {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        debuglog::log("tray-host", "enter NSApplication.run() main loop");
        app.run();
        debuglog::log("tray-host", "NSApplication.run() returned");
    } else {
        debuglog::log("tray-host", "MainThreadMarker unavailable before app.run()");
    }
}

#[cfg(target_os = "macos")]
fn start_macos_tray_watchdog(
    running: Arc<AtomicBool>,
    shared_state: Arc<Mutex<SharedState>>,
    exe_path: std::path::PathBuf,
) {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            set_tray_status(&shared_state, "托盘子进程启动中");
            debuglog::log("watchdog", "spawning tray-host subprocess");
            let child = Command::new(&exe_path)
                .arg("--tray-host")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            let Ok(mut child) = child else {
                debuglog::log("watchdog", "spawn tray-host failed");
                set_tray_status(&shared_state, "托盘子进程启动失败，2s 后重试");
                thread::sleep(Duration::from_secs(3));
                continue;
            };

            set_tray_status(&shared_state, "托盘子进程已启动");
            debuglog::log("watchdog", &format!("tray-host pid={} started", child.id()));

            let status = child.wait();
            let detail = status
                .map(|s| format!("托盘子进程退出: {}，重试中", s))
                .unwrap_or_else(|_| "托盘子进程退出（状态未知），重试中".to_string());
            debuglog::log("watchdog", &detail);
            set_tray_status(&shared_state, &detail);
            if running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(2));
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn stop_macos_tray_sidecar() {
    debuglog::log("core", "pkill tray-host subprocess");
    let _ = Command::new("pkill")
        .args(["-f", "eportal_guard --tray-host"])
        .status();
}

fn http_post(port: u16, path: &str, body: &str) -> bool {
    let mut stream = match TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).is_ok()
}

fn start_monitor(
    running: Arc<AtomicBool>,
    shared_state: Arc<Mutex<SharedState>>,
    config_path: std::path::PathBuf,
    curl_path: std::path::PathBuf,
) {
    thread::spawn(move || {
        let mut previous_cfg = config::load_config(&config_path);
        while running.load(Ordering::SeqCst) {
            let cfg = config::load_config(&config_path);
            if cfg.ping_interval_secs != previous_cfg.ping_interval_secs
                || cfg.ping_host != previous_cfg.ping_host
                || cfg.web_port != previous_cfg.web_port
            {
                notifier::notify("ePortal Guard", "配置更新");
                previous_cfg = cfg.clone();
            }

            let probe = network::ping_probe(&cfg.ping_host);
            let ping_text = format!(
                "{} | {}ms | host={}",
                if probe.ok { "PING 成功" } else { "PING 失败" },
                probe.elapsed_ms,
                cfg.ping_host
            );
            set_ping_text(&shared_state, &ping_text);

            if probe.ok {
                set_state(&shared_state, "网络正常", "");
            } else if network::has_private_ip() {
                set_state(&shared_state, "检测掉线，尝试自动登录", "");
                if !network::curl_exists() {
                    let msg = "未检测到系统 curl 命令";
                    set_state(&shared_state, "自动登录失败", msg);
                    notifier::notify("ePortal Guard", msg);
                } else {
                    let cmd = config::read_curl(&curl_path).unwrap_or_default();
                    if cmd.trim().is_empty() {
                        let msg = "curl.txt 为空，无法登录";
                        set_state(&shared_state, "自动登录失败", msg);
                        notifier::notify("ePortal Guard", msg);
                    } else if platform::shell_run(&cmd) {
                        set_state(&shared_state, "自动登录成功", "");
                        notifier::notify("ePortal Guard", "成功登录");
                    } else {
                        let msg = "curl 命令执行失败";
                        set_state(&shared_state, "自动登录失败", msg);
                        notifier::notify("ePortal Guard", msg);
                    }
                }
            } else {
                set_state(&shared_state, "未连接内网，跳过登录", "");
            }

            thread::sleep(Duration::from_secs(cfg.ping_interval_secs.max(1)));
        }
    });
}

fn set_state(shared_state: &Arc<Mutex<SharedState>>, status: &str, err: &str) {
    if let Ok(mut s) = shared_state.lock() {
        s.status_text = status.to_string();
        s.last_error = err.to_string();
    }
}

fn set_ping_text(shared_state: &Arc<Mutex<SharedState>>, ping: &str) {
    if let Ok(mut s) = shared_state.lock() {
        s.last_ping_text = ping.to_string();
    }
}

fn set_tray_status(shared_state: &Arc<Mutex<SharedState>>, tray_status: &str) {
    if let Ok(mut s) = shared_state.lock() {
        s.tray_status_text = tray_status.to_string();
    }
}
