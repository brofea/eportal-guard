
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

- **断网自重连，安静守护**：后台通过网络探针检测状态，仅在内网可达、互联网不可达且已配置时尝试登录

- **全场景守护，随时在线**：无论是笔记本开合休眠、还是电脑关机重启，开机即联网，告别“每次重新登录”的烦恼

- **全平台通用，不挑设备**：支持 Windows、macOS 和 Linux，主力机、备用机、实验室服务器都能用

- **网页控制台，开箱即用**：免安装，无界面，使用浏览器控制台配置，轻量不占内存


<p align="center">
    <img width="700" alt="UI" src="https://github.com/user-attachments/assets/d554fbaa-ebda-4963-8f72-f1edd655295e" />
</p>


## 如何使用

首先需要抓取登录报文填入控制台，以下给出两种浏览器的方法

### 以 Chrome 为例 

1. 在登录页面按下 F12 打开开发者工具，选择“网络”选项卡，打开“录制”和“保留日志”
<p align="center">
    <img width="441"  alt="Chrome如何录制post请求" src="https://github.com/user-attachments/assets/5e55c829-b9c9-42cc-8179-3318855d1b5b" />
</p>

2. 输入账号密码，登录
3. 在开发者工具中找到 `InterFace.do?method=login`，右键并选择“以 cURL 格式复制”。Windows 用户选择“以 cURL (bash) 格式复制”
<p align="center">
    <img width="441"  alt="Chrome如何复制cURL" src="https://github.com/user-attachments/assets/887011c2-30f6-432c-a10e-ef34b8e40710" />
</p>

4. 启动本软件，在 Web 控制台粘贴 cURL 命令并保存
5. 将软件设置为开机启动（可选）

### 以 FireFox 为例

1. 在登录页面按下 F12 打开开发者工具，选择“网络”选项卡，打开“持续记录”
<p align="center">
    <img height="200" alt="Firefox如何录制post请求" src="https://github.com/user-attachments/assets/b2d5d2a8-9110-4eb0-84d4-f16434f9f89e" />
    <img height="200"  alt="FireFox如何复制cURL" src="https://github.com/user-attachments/assets/5392dc87-748f-48a7-a6fb-d206d9c57204" />
</p>

2. 输入账号密码，登录
3. 在开发者工具中找到 `InterFace.do?method=login`，右键并选择“复制为 cURL 命令”
4. 启动本软件，在 Web 控制台粘贴 cURL 命令并保存
5. 将软件设置为开机启动（可选）


### 注意事项

- 软件默认 Web 控制台地址为 [http://127.0.0.1:18888/](http://127.0.0.1:18888/)，如果默认端口冲突可考虑修改 `config.toml` 文件切换
- 若 macOS 提示软件已损坏，将软件放入“应用程序”文件夹，并在终端执行如下命令
```
sudo xattr -rd com.apple.quarantine /Applications/ePortal\ Guard.app
```
- 若 macOS 提示 Where is use_default，请选择你的默认浏览器
- 若 Windows Defender 提示软件存在风险，请无视并选择"更多信息"和“仍要运行”

## 开发者指南 / 快速开始

非常欢迎任何形式的贡献！无论是发现了 Bug，还是想开发新功能，欢迎提交 Issue 或 PR

### 构建

```bash
cargo build --release
```

已启用体积优化参数

### 运行

```bash
cargo run --release
```

### 配置目录

首次启动自动生成：

- `config.toml`
- `curl.txt`
- `debug.log`

在

- Windows: `%APPDATA%/eportal-guard`
- macOS: `~/Library/Application Support/eportal-guard`
- Linux: `$XDG_CONFIG_HOME/eportal-guard` 或 `~/.config/eportal-guard`
