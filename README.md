# ePortal Guard

极致轻量跨平台校园网守护程序（Rust）。

## 功能

- 后台定时 `ping` 检测网络（默认每 5 秒，默认目标 `223.5.5.5`）
- 仅在 `ping` 失败且检测到内网 IP 时，执行 `curl.txt` 中的原始 cURL 命令
- 跨平台托盘菜单（状态、手动登录、打开配置、教程、编辑 cURL、自启切换、退出）
- 极简本地 Web 后门：`http://127.0.0.1:端口`
- 跨平台开机自启（Windows Run / macOS LaunchAgents / Linux autostart）
- 系统通知（成功登录、配置更新、退出、错误）
- 单实例锁

## 配置目录

- Windows: `%APPDATA%/eportal-guard`
- macOS: `~/Library/Application Support/eportal-guard`
- Linux: `$XDG_CONFIG_HOME/eportal-guard` 或 `~/.config/eportal-guard`

首次启动自动生成：

- `config.toml`
- `curl.txt`

## 构建

```bash
cargo build --release
```

已启用体积优化：`panic = "abort"`, `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`。

## 运行

```bash
cargo run --release
```

如桌面环境托盘不可见，可直接打开浏览器访问：

- `http://127.0.0.1:18888`

## 参考

https://www.lhcloud.com.cn/archives/13/