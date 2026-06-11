#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod autostart;
mod config;
mod debuglog;
mod login;
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

const MAX_LOGIN_FAILURES: u32 = 5;

fn main() {
    // 支持 release bundle 通过 --help 自检，也方便 CI 构建脚本验证二进制可运行。
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .any(|arg| arg == "-help" || arg == "--help" || arg == "-h")
    {
        print_help();
        return;
    }

    debuglog::set_console_enabled(true);

    debuglog::log("主进程", "进程启动");
    if let Err(e) = run() {
        debuglog::log("主进程", &format!("致命错误: {}", e));
        notifier::notify("ePortal Guard", &format!("启动失败: {}", e));
    }
    debuglog::log("主进程", "进程结束");
}

fn print_help() {
    println!(
        "ePortal Guard 参数列表:\n  -help, --help, -h      显示参数说明\n\n日志行为:\n  默认同时输出到终端和 debug.log，便于调试。"
    );
}

fn run() -> Result<(), String> {
    debuglog::log("核心", "初始化核心流程");
    let config_path = paths::config_path();
    let curl_path = paths::curl_path();
    let lock_path = paths::lock_path();
    ensure_parent_dir(&config_path).map_err(|e| e.to_string())?;
    ensure_files(&config_path, &curl_path).map_err(|e| e.to_string())?;

    // 先读取配置再申请锁，这样第二个实例也知道应该打开哪个 Web 端口。
    let cfg = config::load_config(&config_path);
    let _lock = match single_instance::SingleInstance::acquire(&lock_path, cfg.web_port) {
        Ok(v) => v,
        Err(_) => {
            debuglog::log("核心", "获取单实例锁失败，已有实例正在运行");
            let port =
                single_instance::SingleInstance::read_web_port(&lock_path).unwrap_or(cfg.web_port);
            notifier::notify("ePortal Guard", "应用已经启动过了，正在打开控制台");
            open_web_panel(port, "已有实例");
            return Ok(());
        }
    };

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let running = Arc::new(AtomicBool::new(true));

    debuglog::log(
        "核心",
        &format!(
            "Web 控制台启动地址 127.0.0.1:{} | 监控间隔 {} 秒",
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

    let curl_is_empty = config::read_curl(&curl_path)
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);
    if curl_is_empty {
        // 首次使用还没有 cURL 时主动打开控制台，引导用户完成配置。
        open_web_panel(cfg.web_port, "cURL 为空");
    } else {
        // 已配置 cURL 时不会自动打开控制台，用系统通知给用户一个启动反馈。
        notifier::notify("ePortal Guard", "应用已启动");
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
    debuglog::log(
        "核心",
        &format!("打开 Web 控制台: {} | 原因: {}", url, reason),
    );
    wait_for_web_panel(port);
    if !platform::open_url(&url) {
        debuglog::log("核心", &format!("打开 Web 控制台失败: {}", url));
    }
}

fn wait_for_web_panel(port: u16) {
    // Web 服务器在线程里启动，打开浏览器前短暂等待端口可连。
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
        // 自动登录的核心状态机：只有 cURL 已配置、内网可达、外网不可达时才执行登录。
        let mut previous_cfg = config::load_config(&config_path);
        let mut login_failure_count: u32 = 0;
        let mut login_failure_notified = false;
        let mut previous_curl_cmd = String::new();

        while running.load(Ordering::SeqCst) {
            let cfg = config::load_config(&config_path);
            if cfg.ping_interval_secs != previous_cfg.ping_interval_secs
                || cfg.ping_host != previous_cfg.ping_host
                || cfg.web_port != previous_cfg.web_port
            {
                // 配置文件可能由 Web UI 或用户手动修改，监控线程每轮重新加载。
                notifier::notify("ePortal Guard", "配置更新");
                previous_cfg = cfg.clone();
            }

            let curl_cmd = config::read_curl(&curl_path).unwrap_or_default();
            if curl_cmd != previous_curl_cmd {
                debuglog::log("监控", "检测到 cURL 内容变化，重置登录失败计数");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                previous_curl_cmd = curl_cmd.clone();
            }

            debuglog::log(
                "监控",
                &format!(
                    "本轮检查 | cURL 已配置: {} | 间隔: {} 秒",
                    if curl_cmd.trim().is_empty() {
                        "否"
                    } else {
                        "是"
                    },
                    cfg.ping_interval_secs.max(1)
                ),
            );

            if curl_cmd.trim().is_empty() {
                // 没有 cURL 时绝不探测和登录，避免空配置状态下刷屏或误报。
                debuglog::log("监控", "跳过检查: cURL 为空");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "未配置 cURL，等待配置");
                sleep_monitor_interval(&cfg);
                continue;
            }

            let intranet_connected = network::has_private_ip();
            debuglog::log(
                "监控",
                &format!(
                    "内网状态: {}",
                    if intranet_connected {
                        "已连接"
                    } else {
                        "未连接"
                    }
                ),
            );
            if !intranet_connected {
                // 未连接校园内网时跳过外网探针和登录，减少无意义请求。
                debuglog::log("监控", "跳过检查: 未连接内网");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "未连接内网，跳过登录");
                sleep_monitor_interval(&cfg);
                continue;
            }

            let probe = network::internet_probe();
            debuglog::log(
                "监控",
                &format!(
                    "互联网状态: {} | 小米探针: {} HTTP {} exit {:?} {}ms{} | 华为探针: {} HTTP {} exit {:?} {}ms{}",
                    if probe.ok {
                        "可访问"
                    } else {
                        "不可访问"
                    },
                    if probe.miui.ok { "成功" } else { "失败" },
                    probe.miui.status_code,
                    probe.miui.exit_code,
                    probe.miui.elapsed_ms,
                    probe_error_suffix(&probe.miui),
                    if probe.huawei.ok { "成功" } else { "失败" },
                    probe.huawei.status_code,
                    probe.huawei.exit_code,
                    probe.huawei.elapsed_ms,
                    probe_error_suffix(&probe.huawei),
                ),
            );

            if probe.ok {
                // 外网可达说明当前 cURL 是否过期无关紧要，不应反复执行登录命令。
                debuglog::log("监控", "互联网可访问，跳过自动登录");
                reset_login_failures(&mut login_failure_count, &mut login_failure_notified);
                set_state(&shared_state, "网络正常");
            } else if login_failure_count >= MAX_LOGIN_FAILURES {
                debuglog::log(
                    "监控",
                    &format!(
                        "登录失败已达到 {} 次，暂停自动执行 cURL，等待 cURL 更新或网络恢复",
                        MAX_LOGIN_FAILURES
                    ),
                );
                set_state(&shared_state, "无法登陆，已暂停重试");
            } else {
                debuglog::log("监控", "互联网不可访问，开始发送登录 HTTP 请求");
                match login::send_curl_request(&curl_cmd) {
                    Ok(result) => debuglog::log(
                        "监控",
                        &format!(
                            "登录 HTTP 请求已发送 | {} {} | HTTP {}",
                            result.method, result.url, result.status
                        ),
                    ),
                    Err(e) => debuglog::log("监控", &format!("登录 HTTP 请求发送失败: {}", e)),
                }

                let after_login_probe = network::internet_probe();
                let login_ok = after_login_probe.ok;
                debuglog::log(
                    "监控",
                    &format!(
                        "登录后复查结果: {} | 小米探针: {} HTTP {} exit {:?} {}ms{} | 华为探针: {} HTTP {} exit {:?} {}ms{}",
                        if after_login_probe.ok {
                            "互联网已恢复"
                        } else {
                            "互联网仍不可访问"
                        },
                        if after_login_probe.miui.ok {
                            "成功"
                        } else {
                            "失败"
                        },
                        after_login_probe.miui.status_code,
                        after_login_probe.miui.exit_code,
                        after_login_probe.miui.elapsed_ms,
                        probe_error_suffix(&after_login_probe.miui),
                        if after_login_probe.huawei.ok {
                            "成功"
                        } else {
                            "失败"
                        },
                        after_login_probe.huawei.status_code,
                        after_login_probe.huawei.exit_code,
                        after_login_probe.huawei.elapsed_ms,
                        probe_error_suffix(&after_login_probe.huawei),
                    ),
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
    // 进入任何“无需登录/登录成功”的状态后，重新开始失败计数。
    *count = 0;
    *notified = false;
}

fn register_login_failure(
    shared_state: &Arc<Mutex<SharedState>>,
    count: &mut u32,
    notified: &mut bool,
    status: &str,
) {
    // 失败连续累计到 5 次时只弹一次通知，并暂停后续自动重试。
    *count = count.saturating_add(1);
    debuglog::log(
        "监控",
        &format!("登录失败计数: {} | 状态: {}", count, status),
    );

    if *count >= MAX_LOGIN_FAILURES {
        set_state(shared_state, "无法登陆，已暂停重试");
        if !*notified {
            notifier::notify("ePortal Guard", "无法登陆");
            *notified = true;
        }
    } else {
        set_state(shared_state, status);
    }
}

fn probe_error_suffix(probe: &network::HeadProbe) -> String {
    if probe.error_message.is_empty() {
        String::new()
    } else {
        format!(" | 错误: {}", probe.error_message)
    }
}

fn set_state(shared_state: &Arc<Mutex<SharedState>>, status: &str) {
    if let Ok(mut s) = shared_state.lock() {
        if s.status_text != status {
            debuglog::log(
                "状态",
                &format!("状态变化: {} -> {}", s.status_text, status),
            );
            s.status_text.clear();
            s.status_text.push_str(status);
        }
    }
}
