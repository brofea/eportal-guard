
<div align="center">
    <img src="https://raw.githubusercontent.com/brofea/eportal-guard/refs/heads/main/src/Assets.xcassets/AppIcon.appiconset/256-mac.png" width="120" />
    <h1>ePortal Guard</h1>
    <p>轻量化的跨平台Web端锐捷校园网自动登录程序</p>
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

- **断网秒连接，无感重连**：后台自动检测网络，一旦断网自动尝试登录，再也不用手动点开登录页

- **全场景守护，随时在线**：无论是笔记本开合休眠、还是电脑关机重启，开机即联网，告别“每次重新登录”的烦恼

- **全平台通用，不挑设备**：支持 Windows、macOS 和 Linux，主力机、备用机、实验室服务器都能用

- **网页控制台，开箱即用**：免安装，无界面，使用浏览器控制台配置，轻量不占内存

<p align="center">
    <img width="700" alt="image" src="https://github.com/user-attachments/assets/eb8511f9-aaaa-46cf-b9bc-39108c6d0203" />
</p>


## 如何使用

首先需要抓取登录 HTTP 报文内容

### 以 Chrome 为例

1. 在登录页面按下 F12 打开开发者工具，选择“网络”选项卡，打开“录制”和“保留日志”
<p align="center">
    <img width="441"  alt="如何录制post请求" src="https://github.com/user-attachments/assets/5e55c829-b9c9-42cc-8179-3318855d1b5b" />
</p>

2. 输入账号密码，登录
3. 在开发者工具中找到 `InterFace.do?method=login`，右键并选择“以 cURL 格式复制”
<p align="center">
    <img width="441"  alt="如何复制cURL" src="https://github.com/user-attachments/assets/887011c2-30f6-432c-a10e-ef34b8e40710" />
</p>

4. 启动本软件，在 Web 控制台粘贴 cURL 命令并保存

### 以 FireFox 为例

1. 在登录页面按下 F12 打开开发者工具，选择“网络”选项卡，打开“持续记录”
<p align="center">
    <img height="200" alt="2b7873ef-d606-49b1-9666-661ea5e96fd0" src="https://github.com/user-attachments/assets/b2d5d2a8-9110-4eb0-84d4-f16434f9f89e" />
    <img height="200"  alt="204dd462-f20f-4abc-80dd-79a71ccd0c15" src="https://github.com/user-attachments/assets/5392dc87-748f-48a7-a6fb-d206d9c57204" />
</p>

2. 输入账号密码，登录
3. 在开发者工具中找到 `InterFace.do?method=login`，右键并选择“复制为 cURL 命令”
4. 启动本软件，在 Web 控制台粘贴 cURL 命令并保存


### 注意事项

- 若 macOS 提示软件已损坏，将其拖入“应用程序”文件夹并在终端执行如下命令
```
sudo xattr -rd com.apple.quarantine /Applications/ePortal\ Guard.app
```

## 构建和运行

本项目使用 Rust，使用命令构建：

```bash
cargo build --release
```

已启用体积优化：`panic = "abort"`, `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`。

使用命令临时运行：

```bash
cargo run --release
```

## 配置目录

首次启动自动生成：

- `config.toml`
- `curl.txt`

在

- Windows: `%APPDATA%/eportal-guard`
- macOS: `~/Library/Application Support/eportal-guard`
- Linux: `$XDG_CONFIG_HOME/eportal-guard` 或 `~/.config/eportal-guard`

## 代办事项

- [x] 补上 FireFox 的 cURL 获取方法
- [ ] 多次失败会陷入反复重连
- [ ] 即使有网，当前 cURL 不可用时也会反复重连


