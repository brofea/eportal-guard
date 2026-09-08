# Web 控制台

> `src/web.rs`：tiny_http 只监听 `127.0.0.1:<web_port>`，页面/样式/脚本以字符串常量
> 嵌入二进制，无前端构建链。控制台是「本机后门」，一切交互经 HTTP 端点触发，由
> `handle_request` 统一分发。

## 路由表（新增端点时两端同步）

| 方法与路径 | 作用 | 注意 |
|------|------|------|
| `GET /` | 首页：每次动态渲染当前配置/cURL/状态（改文件后刷新即最新） | `render_home` |
| `GET/HEAD /app-icon.png`、`/favicon.ico` | 内嵌 PNG（`include_bytes!` 的 256-mac.png），`Cache-Control: public, max-age=86400` | HEAD 不带 body |
| `GET /status` | `SharedState.status_text` 纯文本（页面每 2s 轮询） | 响应 `text/plain; charset=utf-8` |
| `POST /save` | 保存探针间隔/端口 → `save_config` | 字段沿用 `ping_interval_secs`（兼容） |
| `POST /save-curl` | 保存 curl.txt | 成功/失败通知用户 |
| `POST /manual-login` | 立即用已存 cURL 登录，**不参与失败计数** | 调 `login::send_curl_request` |
| `POST /tutorial` | `platform::open_url(TUTORIAL_URL)` 打开教程 | |
| `POST /toggle-autostart` | `autostart::is_enabled` 取反后 `set_enabled` | |
| `POST /quit` | 置 `running=false` 让主线程自然退出（勿直接 exit） | 同时更新状态文案 |
| 其他 | `404 "not found"` | 未匹配一律拒绝 |

页面 JS（`WEB_JS` 常量）与路由是**跨文件字符串耦合**：新增端点必须同时加
`handle_request` 的 match 分支与页面按钮/fetch 调用；页面统一用
`postAction(action, formId)` 发 `application/x-www-form-urlencoded`。

## 响应与表单约定

- 成功 `200`、业务失败 `500`、未匹配 `404`；响应体是给页面 `#result` 展示的中文串
  （`saved` / `curl saved` / `http 200` 等简短 ASCII + 失败时的中文错误）。
- 表单只解析 `application/x-www-form-urlencoded`（`read_form`），`percent_decode` 宽容：
  非法 % 编码原样保留、最终非 UTF-8 返回空串 —— 这是**有意容错**，勿改成报错。
- 所有回显用户数据（cURL 内容、状态文本、配置值）进 HTML 前必须
  `html_escape`（`& < > "`），页面里用 `textContent` 而非 `innerHTML` 赋值。
- `render_home` 直接拼 HTML 字符串，不引入模板引擎 —— 新增字段照既有 `{}` 占位格式，
  注意转义顺序与占位符个数一一对应。

## 共享状态与线程

- `SharedState { status_text: String }`（`Arc<Mutex<...>>`）：**监控线程**
  `main.rs::set_state` 与 **Web 侧** `web.rs::set_shared_status` 都会写；两处逻辑相同
  （锁内比对、变化才日志+覆盖）。改动共享状态读写逻辑时要同步两处，或抽公共 helper。
- 每请求 `catch_unwind` 隔离（见 error-handling.md）；对锁用 `if let Ok(...)`，
  不 `unwrap`。
- Web 端口绑定失败只通知用户并退出该线程（返回后进程继续跑，属可接受降级，
  见 `start_web_server` 的 `Err(_)` 分支）——改进时优先考虑主线程感知端口失败。

## 状态文案契约（重要）

监控线程/Web 侧写入的 `status_text` 会被页面 JS `statusToneMap` 做**关键字子串匹配**
以决定卡片底色：`正常/成功`→good、`掉线/尝试/退出`→warn、`失败`→bad、
`未连接/初始化`→idle，**其余文案落回 idle**。

- 新增状态文案必须包含上表某个关键字，否则页面永远显示 idle（视觉失联）。
- 匹配顺序是数组序（`正常` 先于 `成功` 等），新增时注意与既有文案的包含关系。
- 页面刷新周期 2s（`setInterval(refreshStatus, 2000)`），任何状态变更要有对应日志，
  便于排查「状态没刷新」。

## 嵌入资源

- 图标 `include_bytes!("Assets.xcassets/AppIcon.appiconset/256-mac.png")` —— 换图标时
  三个尺寸位（`/app-icon.png`、`<link rel=icon>`、brand-mark）共用同一常量，勿各自路径。
- `WEB_JS`/`WEB_CSS` 是超大 raw 字符串常量：改动页面后必须保证引号/反斜杠不破坏
  `r#"..."#` 边界；涉及 `\n` 或 `#` 的场景谨慎（raw string 用 `r##"..."##` 需要时）。
