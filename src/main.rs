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

use config::{ensure_files, ensure_parent_dir};
use web::SharedState;

fn main() {
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

    let tray = if should_enable_tray() {
        Some(
            tray::start_tray(
                Arc::clone(&shared_state),
                config_path.clone(),
                curl_path.clone(),
                exe_path,
                on_manual_login,
                on_exit,
            )
            .map_err(|e| format!("托盘初始化失败: {}", e))?,
        )
    } else {
        notifier::notify("ePortal Guard", "当前会话未启用托盘，请使用 Web 后门");
        None
    };

    while running.load(Ordering::SeqCst) {
        if let Some(tray) = &tray {
            tray.process_events();
        }
        thread::sleep(Duration::from_millis(300));
    }

    notifier::notify("ePortal Guard", "程序退出");
    Ok(())
}

fn should_enable_tray() -> bool {
    #[cfg(target_os = "macos")]
    {
        if std::env::var("EPORTAL_TRAY_FORCE").ok().as_deref() == Some("1") {
            return true;
        }
        return std::env::var("TERM_PROGRAM").is_err();
    }

    #[cfg(not(target_os = "macos"))]
    {
        true
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
        while running.load(Ordering::SeqCst) {
            let cfg = config::load_config(&config_path);
            if cfg.ping_interval_secs != previous_cfg.ping_interval_secs
                || cfg.ping_host != previous_cfg.ping_host
                || cfg.web_port != previous_cfg.web_port
            {
                notifier::notify("ePortal Guard", "配置更新");
                previous_cfg = cfg.clone();
            }

            let ping_ok = network::ping_once(&cfg.ping_host);
            if ping_ok {
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
