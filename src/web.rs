use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tiny_http::{Method, Request, Response, Server};

use crate::autostart;
use crate::config;
use crate::notifier;
use crate::platform;

pub const TUTORIAL_URL: &str = "https://github.com/curl/curl";

#[derive(Clone, Debug)]
pub struct SharedState {
    pub status_text: String,
    pub last_error: String,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            status_text: "初始化中".to_string(),
            last_error: String::new(),
        }
    }
}

pub fn start_web_server(
    state: Arc<Mutex<SharedState>>,
    running: Arc<AtomicBool>,
    config_path: std::path::PathBuf,
    curl_path: std::path::PathBuf,
    exe_path: std::path::PathBuf,
    port: u16,
) {
    let addr = format!("127.0.0.1:{}", port);
    std::thread::spawn(move || {
        let server = match Server::http(&addr) {
            Ok(s) => s,
            Err(_) => {
                notifier::notify("ePortal Guard", "Web 后门端口启动失败");
                return;
            }
        };

        for req in server.incoming_requests() {
            handle_request(req, &state, &running, &config_path, &curl_path, &exe_path);
        }
    });
}

fn handle_request(
    mut req: Request,
    state: &Arc<Mutex<SharedState>>,
    running: &Arc<AtomicBool>,
    config_path: &std::path::Path,
    curl_path: &std::path::Path,
    exe_path: &std::path::Path,
) {
    let url = req.url().to_string();
    match (req.method(), url.as_str()) {
        (&Method::Get, "/") => {
            let body = render_home(state, config_path, curl_path, exe_path);
            let _ = req.respond(Response::from_string(body).with_status_code(200));
        }
        (&Method::Post, "/save") => {
            let form = read_form(&mut req);
            let mut cfg = config::load_config(config_path);
            if let Some(v) = form.get("ping_interval_secs").and_then(|s| s.parse::<u64>().ok()) {
                cfg.ping_interval_secs = v.max(1);
            }
            if let Some(v) = form.get("ping_host") {
                if !v.trim().is_empty() {
                    cfg.ping_host = v.trim().to_string();
                }
            }
            if let Some(v) = form.get("web_port").and_then(|s| s.parse::<u16>().ok()) {
                cfg.web_port = v;
            }
            match config::save_config(config_path, &cfg) {
                Ok(_) => {
                    notifier::notify("ePortal Guard", "配置更新成功");
                    let _ = req.respond(Response::from_string("saved").with_status_code(200));
                }
                Err(e) => {
                    let _ = req.respond(Response::from_string(format!("save failed: {}", e)).with_status_code(500));
                }
            }
        }
        (&Method::Post, "/save-curl") => {
            let form = read_form(&mut req);
            let content = form.get("curl").cloned().unwrap_or_default();
            match config::write_curl(curl_path, &content) {
                Ok(_) => {
                    notifier::notify("ePortal Guard", "cURL 已更新");
                    let _ = req.respond(Response::from_string("curl saved").with_status_code(200));
                }
                Err(e) => {
                    let _ = req.respond(Response::from_string(format!("curl save failed: {}", e)).with_status_code(500));
                }
            }
        }
        (&Method::Post, "/open-config") => {
            let ok = platform::open_path(config_path);
            let _ = req.respond(Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200));
        }
        (&Method::Post, "/open-curl") => {
            let ok = platform::open_path(curl_path);
            let _ = req.respond(Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200));
        }
        (&Method::Post, "/manual-login") => {
            let cmd = config::read_curl(curl_path).unwrap_or_default();
            let ok = crate::platform::shell_run(&cmd);
            if ok {
                notifier::notify("ePortal Guard", "手动登录执行完成");
            }
            let _ = req.respond(Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200));
        }
        (&Method::Post, "/tutorial") => {
            let ok = platform::open_url(TUTORIAL_URL);
            let _ = req.respond(Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200));
        }
        (&Method::Post, "/toggle-autostart") => {
            let cur = autostart::is_enabled(exe_path);
            let res = autostart::set_enabled(exe_path, !cur);
            match res {
                Ok(_) => {
                    let _ = req.respond(Response::from_string("ok").with_status_code(200));
                }
                Err(e) => {
                    let _ = req.respond(Response::from_string(e).with_status_code(500));
                }
            }
        }
        (&Method::Post, "/quit") => {
            running.store(false, Ordering::SeqCst);
            notifier::notify("ePortal Guard", "程序退出");
            let _ = req.respond(Response::from_string("bye").with_status_code(200));
        }
        _ => {
            let _ = req.respond(Response::from_string("not found").with_status_code(404));
        }
    }
}

fn render_home(
    state: &Arc<Mutex<SharedState>>,
    config_path: &std::path::Path,
    curl_path: &std::path::Path,
    exe_path: &std::path::Path,
) -> String {
    let cfg = config::load_config(config_path);
    let curl = config::read_curl(curl_path).unwrap_or_default();
    let s = state.lock().map(|v| v.clone()).unwrap_or_default();
    let autostart = autostart::is_enabled(exe_path);

    format!(
        "<!doctype html><html><head><meta charset='utf-8'><title>ePortal Guard</title></head><body>\
        <h2>ePortal Guard</h2>\
        <p>状态: {}</p><p>错误: {}</p><p>自启: {}</p>\
        <form method='post' action='/save'>\
        ping间隔(s): <input name='ping_interval_secs' value='{}'/><br/>\
        ping服务器: <input name='ping_host' value='{}'/><br/>\
        Web端口: <input name='web_port' value='{}'/><br/>\
        <button type='submit'>保存配置</button></form><hr/>\
        <form method='post' action='/save-curl'>\
        <textarea name='curl' rows='6' cols='80'>{}</textarea><br/>\
        <button type='submit'>保存cURL</button></form><hr/>\
        <form method='post' action='/manual-login'><button>手动登录</button></form>\
        <form method='post' action='/open-config'><button>打开config</button></form>\
        <form method='post' action='/open-curl'><button>粘贴cURL</button></form>\
        <form method='post' action='/tutorial'><button>获取cURL教程</button></form>\
        <form method='post' action='/toggle-autostart'><button>切换开机自启</button></form>\
        <form method='post' action='/quit'><button>退出程序</button></form>\
        </body></html>",
        html_escape(&s.status_text),
        html_escape(&s.last_error),
        if autostart { "已开启" } else { "已关闭" },
        cfg.ping_interval_secs,
        html_escape(&cfg.ping_host),
        cfg.web_port,
        html_escape(&curl),
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn read_form(req: &mut Request) -> HashMap<String, String> {
    let mut body = String::new();
    if req.as_reader().read_to_string(&mut body).is_err() {
        return HashMap::new();
    }

    let mut map = HashMap::new();
    for pair in body.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut kv = pair.splitn(2, '=');
        let key = percent_decode(kv.next().unwrap_or_default());
        let val = percent_decode(kv.next().unwrap_or_default());
        map.insert(key, val);
    }
    map
}

fn percent_decode(v: &str) -> String {
    let bytes = v.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
