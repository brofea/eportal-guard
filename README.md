
<div align="center">
    <img src="https://raw.githubusercontent.com/brofea/eportal-guard/refs/heads/main/src/Assets.xcassets/AppIcon.appiconset/256-mac.png" width="120" />
    <h1>ePortal Guard</h1>
    <p>极致轻量的跨平台Web端锐捷校园网自动登录程序</p>
    <p>
        <a href="https://www.gnu.org/licenses/gpl-3.0.en.html">
            <img src="https://img.shields.io/badge/license-GPL--3.0-orange" />
        </a>
        <a href="https://github.com/brofea/eportal-guard/actions/workflows/release-on-tag.yml">
            <img src="https://img.shields.io/github/actions/workflow/status/brofea/eportal-guard/release-on-tag.yml?label=build" alt="Build Status">
        </a>
        <a href="https://github.com/brofea/eportal-guard/releases">
            <img src="https://img.shields.io/github/v/tag/brofea/eportal-guard?color=blue&label=version" alt="Latest Version">
        </a>
        <a href="https://github.com/brofea">
            <img src="https://img.shields.io/badge/brofea-brofea?label=GitHub&logo=github&color=purple" alt="GitHub Profile">
        </a>
    </p>
</div>

## 功能

- 后台定时 `ping` 检测网络
- `ping`失败且连接内网时尝试登陆
- 无GUI，使用系统托盘和本地 Web 后门操作
- 跨平台（Windows / macOS / Linux）

## 如何使用

1. 在登录页面按下F12打开开发者界面，选择“网络”选项卡，打开“保留日志”与“录制网络日志”
<p align="center">
    <img width="441"  alt="如何录制post请求" src="https://github.com/user-attachments/assets/5e55c829-b9c9-42cc-8179-3318855d1b5b" />
</p>

2. 输入账号密码登录，找到`InterFace.do?method=login`右键，并选择“以cURL格式复制”
<p align="center">
    <img width="441"  alt="如何复制cURL" src="https://github.com/user-attachments/assets/887011c2-30f6-432c-a10e-ef34b8e40710" />
</p>

3. 启动本软件，在浏览器输入`http://127.0.0.1:18888`粘贴 cURL 并保存

### 注意事项

- 若 macOS 用户提示软件已损坏，将其拖入“应用程序”文件夹并在终端执行如下命令
```
sudo xattr -rd com.apple.quarantine /Applications/ePortal\ Guard.app
```

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

## 代办事项
- [x] macOS 系统托盘
- [ ] Windows 系统托盘
- [ ] KDE 系统托盘


## 致谢

- [Lucide Icons](https://lucide.dev/) - 基于 [ISC 许可证] 开源
- [GitHub Copilot](https://github.com/copilot) - 任劳任怨的 Agent


