#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod autostart;
mod config;
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
use std::{io::Read, io::Write, net::TcpStream, process::Command};

use config::{ensure_files, ensure_parent_dir};
use web::SharedState;

fn main() {
    if std::env::args().any(|arg| arg == "--tray-host") {
        if let Err(e) = run_tray_host() {
            notifier::notify("ePortal Guard Tray", &format!("托盘子进程启动失败: {}", e));
        }
        return;
    }

    if let Err(e) = run() {
        notifier::notify("ePortal Guard", &format!("启动失败: {}", e));
    }
}

fn run() -> Result<(), String> {
    let config_path = paths::config_path();
    let curl_path = paths::curl_path();
    let lock_path = paths::lock_path();
    ensure_parent_dir(&config_path).map_err(|e| e.to_string())?;
    ensure_files(&config_path, &curl_path).map_err(|e| e.to_string())?;

    let _lock = match single_instance::SingleInstance::acquire(&lock_path) {
        Ok(v) => v,
        Err(_) => {
            return Err("检测到程序已在运行（单实例锁失败）".to_string());
        }
    };

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let running = Arc::new(AtomicBool::new(true));

    let cfg = config::load_config(&config_path);
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

    let running_for_exit = Arc::clone(&running);
    let on_exit = Arc::new(move || {
        running_for_exit.store(false, Ordering::SeqCst);
    });

    let curl_path_for_manual = curl_path.clone();
    let on_manual_login = Arc::new(move || {
        tray::run_manual_login(&curl_path_for_manual);
    });

    #[cfg(target_os = "macos")]
    start_macos_tray_watchdog(Arc::clone(&running), Arc::clone(&shared_state), exe_path.clone());

    #[cfg(not(target_os = "macos"))]
    let tray = match tray::start_tray(
        Arc::clone(&shared_state),
        config_path.clone(),
        curl_path.clone(),
        exe_path,
        on_manual_login,
        on_exit,
    ) {
        Ok(v) => Some(v),
        Err(e) => {
            set_state(&shared_state, "托盘启动失败，已降级为 Web 模式", &e);
            None
        }
    };

    #[cfg(target_os = "macos")]
    let tray: Option<tray::TrayHandle> = None;

    let _keep_closures = (on_manual_login, on_exit);

    while running.load(Ordering::SeqCst) {
        if let Some(tray) = &tray {
            tray.process_events();
        }
        thread::sleep(Duration::from_millis(300));
    }

    notifier::notify("ePortal Guard", "程序退出");
    Ok(())
}

fn run_tray_host() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    init_macos_appkit();

    let config_path = paths::config_path();
    let curl_path = paths::curl_path();
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let cfg = config::load_config(&config_path);
    let port = cfg.web_port;

    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let running = Arc::new(AtomicBool::new(true));

    {
        let shared_state = Arc::clone(&shared_state);
        let running = Arc::clone(&running);
        thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                if let Some((status, err, ping, tray)) = fetch_status(port) {
                    set_full_state(&shared_state, &status, &err, &ping, &tray);
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    let on_manual_login = Arc::new(move || {
        let _ = http_post(port, "/manual-login", "");
    });

    let running_for_exit = Arc::clone(&running);
    let on_exit = Arc::new(move || {
        running_for_exit.store(false, Ordering::SeqCst);
        let _ = http_post(port, "/quit", "");
    });

    let tray = tray::start_tray(
        Arc::clone(&shared_state),
        config_path,
        curl_path,
        exe_path,
        on_manual_login,
        on_exit,
    )
    .map_err(|e| format!("托盘初始化失败: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        let dispatcher = tray.dispatcher();
        thread::spawn(move || dispatcher.run_blocking());
        run_macos_app_loop();
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
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let _ = objc2_app_kit::NSApplication::sharedApplication(mtm);
    }
}

#[cfg(target_os = "macos")]
fn run_macos_app_loop() -> ! {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        app.run();
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
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
            let child = Command::new(&exe_path)
                .arg("--tray-host")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            let Ok(mut child) = child else {
                set_tray_status(&shared_state, "托盘子进程启动失败，2s 后重试");
                thread::sleep(Duration::from_secs(3));
                continue;
            };

            set_tray_status(&shared_state, "托盘子进程已启动");

            let status = child.wait();
            let detail = status
                .map(|s| format!("托盘子进程退出: {}，重试中", s))
                .unwrap_or_else(|_| "托盘子进程退出（状态未知），重试中".to_string());
            set_tray_status(&shared_state, &detail);
            if running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(2));
            }
        }
    });
}

fn fetch_status(port: u16) -> Option<(String, String, String, String)> {
    let resp = http_get(port, "/status")?;
    let mut parts = resp.lines();
    let status = parts.next().unwrap_or_default().to_string();
    let err = parts.next().unwrap_or_default().to_string();
    let ping = parts.next().unwrap_or_default().to_string();
    let tray = parts.next().unwrap_or_default().to_string();
    Some((status, err, ping, tray))
}

fn http_get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        path
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return None;
    }

    let mut raw = String::new();
    if stream.read_to_string(&mut raw).is_err() {
        return None;
    }
    raw.split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .or(Some(raw))
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

fn set_full_state(
    shared_state: &Arc<Mutex<SharedState>>,
    status: &str,
    err: &str,
    ping: &str,
    tray: &str,
) {
    if let Ok(mut s) = shared_state.lock() {
        s.status_text = status.to_string();
        s.last_error = err.to_string();
        s.last_ping_text = ping.to_string();
        s.tray_status_text = tray.to_string();
    }
}
