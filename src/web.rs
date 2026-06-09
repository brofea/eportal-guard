use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tiny_http::{Header, Method, Request, Response, Server};

use crate::autostart;
use crate::config;
use crate::debuglog;
use crate::notifier;
use crate::platform;

pub const TUTORIAL_URL: &str = "https://github.com/brofea/eportal-guard";
const APP_ICON_PNG: &[u8] = include_bytes!("Assets.xcassets/AppIcon.appiconset/256-mac.png");
const WEB_JS: &str = r#"<script>
const statusToneMap = [
    ['正常', 'good'],
    ['成功', 'good'],
    ['掉线', 'warn'],
    ['尝试', 'warn'],
    ['失败', 'bad'],
    ['未连接', 'idle'],
    ['初始化', 'idle'],
];

function setStatusText(status) {
    const statusEl = document.getElementById('status_text');
    const statusCard = document.getElementById('status_card');
    const nextStatus = status || '等待状态';
    if (statusEl) statusEl.textContent = nextStatus;
    if (statusCard) {
        const tone = statusToneMap.find(([keyword]) => nextStatus.includes(keyword));
        statusCard.dataset.tone = tone ? tone[1] : 'idle';
    }
}

function setResult(text, tone) {
    const result = document.getElementById('result');
    if (!result) return;
    result.textContent = text;
    result.dataset.tone = tone || 'idle';
}

async function refreshStatus() {
    try {
        const resp = await fetch('/status');
        const text = await resp.text();
        setStatusText(text || '');
    } catch (_) {}
}

async function postAction(action, formId) {
    let body = new URLSearchParams();
    if (formId) {
        const form = document.getElementById(formId);
        if (form) {
            body = new URLSearchParams(new FormData(form));
        }
    }
    setResult('正在提交...', 'busy');
    try {
        const resp = await fetch(action, {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8' },
            body,
        });
        const text = await resp.text();
        const content = (resp.ok ? '完成: ' : '失败: ') + text;
        setResult(content, resp.ok ? 'good' : 'bad');
        if (resp.ok) {
            refreshStatus();
            setTimeout(() => {
                const result = document.getElementById('result');
                if (result && result.textContent === content) {
                    setResult('等待操作', 'idle');
                }
            }, 2000);
        }
    } catch (e) {
        setResult('请求失败: ' + (e && e.message ? e.message : e), 'bad');
    }
}

setInterval(refreshStatus, 2000);
refreshStatus();
</script>"#;

const WEB_CSS: &str = r#"<style>
:root {
    color-scheme: light;
    --bg: #f6f8fb;
    --panel: #ffffff;
    --ink: #172033;
    --muted: #647084;
    --line: #dce3ef;
    --blue: #2563eb;
    --blue-ink: #1d4ed8;
    --green: #0f9f6e;
    --amber: #b7791f;
    --red: #dc2626;
    --shadow: 0 22px 70px rgba(29, 41, 57, 0.12);
}

* { box-sizing: border-box; }

body {
    margin: 0;
    min-height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: var(--ink);
    background:
        radial-gradient(circle at 12% 10%, rgba(37, 99, 235, 0.12), transparent 30%),
        linear-gradient(135deg, #f7fbff 0%, var(--bg) 42%, #f9faf5 100%);
}

.shell {
    width: min(1160px, calc(100% - 32px));
    margin: 0 auto;
    padding: 32px 0;
}

.topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 22px;
}

.brand {
    display: flex;
    align-items: center;
    gap: 14px;
}

.brand-mark {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: #ffffff;
    box-shadow: 0 14px 34px rgba(37, 99, 235, 0.24);
    overflow: hidden;
}

.brand-mark img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
}

h1, h2, p { margin: 0; }
h1 { font-size: 26px; line-height: 1.1; letter-spacing: 0; }
.subtitle { margin-top: 6px; color: var(--muted); font-size: 14px; }

.status-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 7px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.78);
    color: var(--muted);
    font-size: 13px;
    white-space: nowrap;
}

.status-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--green);
    box-shadow: 0 0 0 5px rgba(15, 159, 110, 0.12);
}

.grid {
    display: grid;
    grid-template-columns: minmax(0, 1.08fr) minmax(320px, 0.92fr);
    gap: 18px;
    align-items: start;
}

.panel {
    background: rgba(255, 255, 255, 0.92);
    border: 1px solid rgba(220, 227, 239, 0.95);
    border-radius: 8px;
    box-shadow: var(--shadow);
}

.panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 20px 22px 0;
}

.panel-title { font-size: 17px; font-weight: 750; }
.panel-note { margin-top: 4px; color: var(--muted); font-size: 13px; }
.panel-body { padding: 18px 22px 22px; }

.status-card {
    display: grid;
    gap: 18px;
    min-height: 196px;
    padding: 22px;
    color: #ffffff;
    background: linear-gradient(135deg, #1e3a8a, #0f766e);
}

.status-card[data-tone="good"] { background: linear-gradient(135deg, #166534, #0f766e); }
.status-card[data-tone="warn"] { background: linear-gradient(135deg, #92400e, #b45309); }
.status-card[data-tone="bad"] { background: linear-gradient(135deg, #991b1b, #b91c1c); }
.status-card[data-tone="idle"] { background: linear-gradient(135deg, #334155, #1d4ed8); }

.status-label { color: rgba(255, 255, 255, 0.72); font-size: 13px; }
.status-text { margin-top: 8px; font-size: 34px; line-height: 1.12; font-weight: 800; }
.status-meta { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.metric {
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.1);
}
.metric-label { color: rgba(255, 255, 255, 0.72); font-size: 12px; }
.metric-value { margin-top: 4px; font-weight: 750; }

#result {
    min-height: 42px;
    margin: 14px 0 0;
    padding: 11px 13px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: #f8fafc;
    color: var(--muted);
    font-size: 14px;
}

#result[data-tone="good"] { color: #047857; background: #ecfdf5; border-color: #a7f3d0; }
#result[data-tone="bad"] { color: var(--red); background: #fef2f2; border-color: #fecaca; }
#result[data-tone="busy"] { color: var(--blue-ink); background: #eff6ff; border-color: #bfdbfe; }

.field-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
}

label {
    display: block;
    margin-bottom: 7px;
    color: #475569;
    font-size: 13px;
    font-weight: 650;
}

input, textarea {
    width: 100%;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: #ffffff;
    color: var(--ink);
    outline: none;
    transition: border-color .16s ease, box-shadow .16s ease;
}

input {
    height: 42px;
    padding: 0 12px;
    font-size: 14px;
}

textarea {
    min-height: 246px;
    padding: 13px;
    resize: vertical;
    line-height: 1.5;
    font-size: 13px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

input:focus, textarea:focus {
    border-color: rgba(37, 99, 235, 0.72);
    box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.12);
}

.actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 14px;
}

button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 40px;
    padding: 0 14px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: #ffffff;
    color: var(--ink);
    cursor: pointer;
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    transition: transform .16s ease, border-color .16s ease, background .16s ease, color .16s ease;
}

button:hover {
    transform: translateY(-1px);
    border-color: #b8c4d6;
    background: #f8fafc;
}

.primary {
    border-color: var(--blue);
    background: var(--blue);
    color: #ffffff;
}

.primary:hover { background: var(--blue-ink); border-color: var(--blue-ink); }
.danger { color: var(--red); }

@media (max-width: 860px) {
    .shell { width: min(100% - 22px, 680px); padding: 18px 0; }
    .topbar { align-items: flex-start; flex-direction: column; }
    .grid, .field-grid { grid-template-columns: 1fr; }
    .panel-header, .panel-body, .status-card { padding-left: 16px; padding-right: 16px; }
    .status-text { font-size: 28px; }
    .status-meta { grid-template-columns: 1fr; }
    button { width: 100%; }
}
</style>"#;

#[derive(Clone, Debug)]
pub struct SharedState {
    pub status_text: String,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            status_text: "初始化中".to_string(),
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
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                handle_request(req, &state, &running, &config_path, &curl_path, &exe_path);
            }));

            if let Err(payload) = result {
                debuglog::log(
                    "web",
                    &format!(
                        "request handler panicked: {}",
                        panic_message(payload.as_ref())
                    ),
                );
            }
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
    debuglog::log("web", &format!("{} {}", req.method().as_str(), url));
    match (req.method(), url.as_str()) {
        (&Method::Get, "/") => {
            let body = render_home(state, config_path, curl_path, exe_path);
            let html_header = Header::from_bytes(
                b"Content-Type".as_slice(),
                b"text/html; charset=utf-8".as_slice(),
            )
            .ok();
            let mut resp = Response::from_string(body).with_status_code(200);
            if let Some(header) = html_header {
                resp = resp.with_header(header);
            }
            let _ = req.respond(resp);
        }
        (&Method::Get, "/app-icon.png") | (&Method::Get, "/favicon.ico") => {
            respond_app_icon(req, true);
        }
        (&Method::Head, "/app-icon.png") | (&Method::Head, "/favicon.ico") => {
            respond_app_icon(req, false);
        }
        (&Method::Get, "/status") => {
            let s = state.lock().map(|v| v.clone()).unwrap_or_default();
            let body = s.status_text;
            let header = Header::from_bytes(
                b"Content-Type".as_slice(),
                b"text/plain; charset=utf-8".as_slice(),
            )
            .ok();
            let mut resp = Response::from_string(body).with_status_code(200);
            if let Some(header) = header {
                resp = resp.with_header(header);
            }
            let _ = req.respond(resp);
        }
        (&Method::Post, "/save") => {
            let form = read_form(&mut req);
            let mut cfg = config::load_config(config_path);
            if let Some(v) = form
                .get("ping_interval_secs")
                .and_then(|s| s.parse::<u64>().ok())
            {
                cfg.ping_interval_secs = v.clamp(1, 3600);
            }
            if let Some(v) = form.get("web_port").and_then(|s| s.parse::<u16>().ok()) {
                cfg.web_port = v;
            }
            match config::save_config(config_path, &cfg) {
                Ok(_) => {
                    let _ = req.respond(Response::from_string("saved").with_status_code(200));
                    notifier::notify("ePortal Guard", "配置更新成功");
                }
                Err(e) => {
                    let _ = req.respond(
                        Response::from_string(format!("save failed: {}", e)).with_status_code(500),
                    );
                }
            }
        }
        (&Method::Post, "/save-curl") => {
            let form = read_form(&mut req);
            let content = form.get("curl").cloned().unwrap_or_default();
            match config::write_curl(curl_path, &content) {
                Ok(_) => {
                    let _ = req.respond(Response::from_string("curl saved").with_status_code(200));
                    notifier::notify("ePortal Guard", "cURL 已更新");
                }
                Err(e) => {
                    let _ = req.respond(
                        Response::from_string(format!("curl save failed: {}", e))
                            .with_status_code(500),
                    );
                }
            }
        }
        (&Method::Post, "/manual-login") => {
            let cmd = config::read_curl(curl_path).unwrap_or_default();
            let ok = crate::platform::shell_run(&cmd);
            let _ = req.respond(
                Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200),
            );
            if ok {
                notifier::notify("ePortal Guard", "手动登录执行完成");
            }
        }
        (&Method::Post, "/tutorial") => {
            let ok = platform::open_url(TUTORIAL_URL);
            let _ = req.respond(
                Response::from_string(if ok { "ok" } else { "failed" }).with_status_code(200),
            );
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
            let _ = req.respond(Response::from_string("bye").with_status_code(200));
        }
        _ => {
            let _ = req.respond(Response::from_string("not found").with_status_code(404));
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.as_str()
    } else {
        "unknown panic payload"
    }
}

fn respond_app_icon(req: Request, include_body: bool) {
    let header = Header::from_bytes(b"Content-Type".as_slice(), b"image/png".as_slice()).ok();
    let cache_header = Header::from_bytes(
        b"Cache-Control".as_slice(),
        b"public, max-age=86400".as_slice(),
    )
    .ok();
    let body = if include_body {
        APP_ICON_PNG.to_vec()
    } else {
        Vec::new()
    };
    let mut resp = Response::from_data(body).with_status_code(200);
    if let Some(header) = header {
        resp = resp.with_header(header);
    }
    if let Some(header) = cache_header {
        resp = resp.with_header(header);
    }
    let _ = req.respond(resp);
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
        "<!doctype html><html lang='zh-CN'><head><meta charset='utf-8'>\
        <meta name='viewport' content='width=device-width, initial-scale=1'>\
        <link rel='icon' type='image/png' href='/app-icon.png'>\
        <link rel='apple-touch-icon' href='/app-icon.png'>\
        <title>ePortal Guard 锐捷校园网自动登录</title>{}</head><body>\
        <main class='shell'>\
            <header class='topbar'>\
                <div class='brand'>\
                    <div class='brand-mark'><img src='/app-icon.png' alt='ePortal Guard logo'></div>\
                    <div><h1>ePortal Guard</h1><p class='subtitle'>校园网自动登录控制台</p></div>\
                </div>\
                <div class='status-pill'><span class='status-dot'></span><span>本机 Web UI</span></div>\
            </header>\
            <section class='grid'>\
                <div>\
                    <section id='status_card' class='panel status-card' data-tone='idle'>\
                        <div>\
                            <div class='status-label'>当前连接状态</div>\
                            <div id='status_text' class='status-text'>{}</div>\
                        </div>\
                        <div class='status-meta'>\
                            <div class='metric'><div class='metric-label'>开机自启</div><div class='metric-value'>{}</div></div>\
                            <div class='metric'><div class='metric-label'>Web 端口</div><div class='metric-value'>{}</div></div>\
                        </div>\
                    </section>\
                    <div id='result' data-tone='idle' aria-live='polite'>等待操作</div>\
                    <section class='panel' style='margin-top:18px'>\
                        <div class='panel-header'>\
                            <div><h2 class='panel-title'>连接设置</h2><p class='panel-note'>调整网络探针频率和本机控制台端口。</p></div>\
                        </div>\
                        <div class='panel-body'>\
                            <form id='cfgForm'>\
                                <div class='field-grid'>\
                                    <div><label for='ping_interval_secs'>探针间隔（秒）</label><input id='ping_interval_secs' name='ping_interval_secs' inputmode='numeric' value='{}'></div>\
                                    <div><label for='web_port'>Web 端口</label><input id='web_port' name='web_port' inputmode='numeric' value='{}'></div>\
                                </div>\
                                <div class='actions'><button class='primary' type='button' onclick=\"postAction('/save','cfgForm')\">保存配置</button></div>\
                            </form>\
                        </div>\
                    </section>\
                    <section class='panel' style='margin-top:18px'>\
                        <div class='panel-header'>\
                            <div><h2 class='panel-title'>快捷操作</h2><p class='panel-note'>手动登录、教程、自启和退出集中在这里。</p></div>\
                        </div>\
                        <div class='panel-body'>\
                            <div class='actions' style='margin-top:0'>\
                                <button type='button' onclick=\"postAction('/manual-login')\">手动登录</button>\
                                <button type='button' onclick=\"postAction('/tutorial')\">获取 cURL 教程</button>\
                                <button type='button' onclick=\"postAction('/toggle-autostart')\">切换开机自启</button>\
                                <button class='danger' type='button' onclick=\"postAction('/quit')\">退出程序</button>\
                            </div>\
                        </div>\
                    </section>\
                </div>\
                <section class='panel'>\
                    <div class='panel-header'>\
                        <div><h2 class='panel-title'>登录 cURL</h2><p class='panel-note'>粘贴从认证页面复制的 cURL 命令，自动登录会复用它。</p></div>\
                    </div>\
                    <div class='panel-body'>\
                        <form id='curlForm'>\
                            <label for='curl'>cURL 命令</label>\
                            <textarea id='curl' name='curl' spellcheck='false'>{}</textarea>\
                            <div class='actions'><button class='primary' type='button' onclick=\"postAction('/save-curl','curlForm')\">保存 cURL</button></div>\
                        </form>\
                    </div>\
                </section>\
            </section>\
        </main>{}</body></html>",
        WEB_CSS,
        html_escape(&s.status_text),
        if autostart { "已开启" } else { "已关闭" },
        cfg.web_port,
        cfg.ping_interval_secs,
        cfg.web_port,
        html_escape(&curl),
        WEB_JS,
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
