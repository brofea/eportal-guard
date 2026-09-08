# 目录结构与模块职责

> src 下的模块全部扁平放置，无子目录。模块按「系统能力」切分，每个模块有单一职责。

## 模块地图（src/）

| 文件 | 职责 | 关键公开符号 |
|------|------|--------------|
| `main.rs` | 入口、参数解析、启动流程、**监控线程状态机**、失败计数 | `run()`、`start_monitor()`、`MAX_LOGIN_FAILURES` |
| `paths.rs` | 跨平台运行时目录/文件路径 | `app_config_dir()`、`config_path()`、`curl_path()`、`lock_path()`、`APP_DIR_NAME` |
| `config.rs` | `AppConfig`、config.toml 读写、curl.txt 读写 | `AppConfig`、`ensure_files()`、`load_config()`、`save_config()` |
| `network.rs` | 内网/外网连通性探测 | `internet_probe()`、`HeadProbe`、`has_private_ip()`、探针 URL 常量 |
| `login.rs` | cURL 命令解析 + 发送登录 HTTP 请求 | `send_curl_request()`、`LoginResult`、`ParsedCurl`(私有) |
| `web.rs` | 本机 Web 控制台（tiny_http）、嵌入页面 | `start_web_server()`、`SharedState`、`handle_request()`(私有)、`TUTORIAL_URL` |
| `autostart.rs` | 开机自启查询/设置（三平台） | `is_enabled()`、`set_enabled()` |
| `notifier.rs` | 桌面系统通知（三平台） | `notify(summary, body)` |
| `platform.rs` | 用系统默认方式打开 URL（三平台） | `open_url()` |
| `single_instance.rs` | 锁文件单实例 | `SingleInstance`、`acquire()`、`read_web_port()` |
| `debuglog.rs` | 终端 + debug.log 双写日志 | `log(component, message)`、`set_console_enabled()` |

`src/Assets.xcassets/` 只放 AppIcon（`256-mac.png` 由 `web.rs` 以 `include_bytes!` 嵌入，其余尺寸仅打包脚本使用）。

## 依赖方向（谁可以引用谁）

- `main.rs` 是装配层，引用除 `autostart.rs` 外的所有模块（`autostart` 只被 web.rs 调用）；
  **其余模块不得引用 main.rs**。
- `paths.rs` 被 `main.rs`、`debuglog.rs` 引用；`autostart.rs` 在 Windows 分支只取常量
  `APP_RUN_KEY_NAME`（`use crate::paths::APP_RUN_KEY_NAME;`）。不要从 `config.rs` 再包一层路径函数。
- `web.rs` 是唯一 HTTP 边界：它引用 `config`/`login`/`autostart`/`platform`/`notifier`/`debuglog`；
  被 `main.rs` 引用。任何「由网页触发的动作」都要经 `web.rs::handle_request` 分发，
  不要在页面 JS 里直接执行逻辑。
- 网络语义只存在于 `network.rs`（探针）与 `login.rs`（登录请求）：新增 HTTP 出口要放
  进这两个模块之一，不要散落在 main/web 里内联 `reqwest` 调用。
- 跨模块共享的可变状态只允许两种：`Arc<Mutex<SharedState>>`、`Arc<AtomicBool> running`，
  由 `main.rs` 创建、经参数传给 `start_web_server` / `start_monitor`；禁止新增全局 `static mut`。

## 模块内部布局约定

- 每个平台分支写成一个独立 `#[cfg(...)]` 函数，公共入口里用 `#[cfg]` 块分发，
  不要用 `if cfg!(...)` 把三套实现塞进同一函数（反例见 platform-support.md §平台分发）。
- 纯私有工具函数（如 `parse_curl`、`percent_decode`）放模块尾部、对应测试之前。
- 单元测试以 `#[cfg(test)] mod tests` 放在文件末尾，`use super::*;` 引用被测符号。

## 命名约定

- crate 名与二进制名 `eportal_guard`；用户可见产品名 `ePortal Guard`；目录名 `eportal-guard`。
  三处不一致是历史事实，新增常量时不要混用（发布脚本 env 见 quality-guidelines.md）。
- 常量用 `SCREAMING_SNAKE_CASE` 并放在所属模块（探针 URL 在 network.rs、`MAX_LOGIN_FAILURES`
  在 main.rs、`WINDOWS_APP_ID` 在 notifier.rs、`APP_DIR_NAME` 在 paths.rs），禁止跨文件复制。
- 日志组件标签是约定俗成的简短中文名词（"主进程" "核心" "监控" "网页" "状态" "通知"），
  新增组件沿用该风格并 grep 确认没有近似旧标签。
