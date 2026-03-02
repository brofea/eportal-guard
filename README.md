<h1 align="center">ePortal Guard</h1>

极致轻量的跨平台Web端锐捷校园网自动登录程序

## 功能

- 后台定时 `ping` 检测网络
- `ping`失败且连接内网时尝试登陆
- 无GUI，使用系统托盘和本地 Web 后门操作
- 跨平台（Windows / macOS / Linux）

## 如何使用



## 配置目录

首次启动自动生成：

- `config.toml`
- `curl.txt`

在

- Windows: `%APPDATA%/eportal-guard`
- macOS: `~/Library/Application Support/eportal-guard`
- Linux: `$XDG_CONFIG_HOME/eportal-guard` 或 `~/.config/eportal-guard`



## 构建

```bash
cargo build --release
```

已启用体积优化：`panic = "abort"`, `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`。

## 运行

```bash
cargo run --release
```

如桌面环境无系统托盘，可用浏览器访问：

- `http://127.0.0.1:18888`

## 致谢

- [Lucide Icons](https://lucide.dev/) - Licensed under the [ISC License].
