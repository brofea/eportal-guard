# Copilot Instructions for ePortal Guard

## 1) 项目定位

`ePortal Guard` 是一个**极致轻量**的跨平台校园网守护程序，核心目标：

- 后台定时巡检网络连通性（默认每 5 秒）
- 仅在 `ping` 失败且本机仍处于内网环境时，自动执行用户提供的 `cURL` 登录命令
- 提供两套无 GUI 依赖的操作入口：
  - 系统托盘
  - 本地 Web 控制页（`127.0.0.1:端口`）

项目优先级：**稳定性 > 体积 > 功能扩展**。

---

## 2) 当前代码结构（src）

- `main.rs`
  - 程序入口
  - 单实例控制
  - 核心守护线程（ping + 自动登录）
  - Web 服务启动
  - 托盘/托盘子进程编排（macOS）
- `config.rs`
  - `config.toml` / `curl.txt` 读写
  - 默认配置生成
- `paths.rs`
  - 跨平台配置目录映射
- `single_instance.rs`
  - 单实例锁（含陈旧锁恢复）
- `network.rs`
  - `ping` 探测与耗时采样
  - 内网 IP 判断
  - `curl` 可用性检测
- `platform.rs`
  - 打开文件/URL
  - Shell 命令执行
  - cURL 命令归一化（多行、CRLF、续行）
- `web.rs`
  - `tiny_http` 路由
  - Web 页面渲染与状态接口
- `tray.rs`
  - `tray-icon` 托盘菜单及事件分发
- `autostart.rs`
  - 跨平台开机自启
- `notifier.rs`
  - 系统通知封装

---

## 3) 架构约束与跨平台策略

### 3.1 轻量化约束

- 禁止引入重量级 HTTP 客户端（如 `reqwest`）
- 网络登录必须通过系统 `curl` 执行用户原始命令
- 保持发布配置的体积优化选项

### 3.2 平台差异

- Windows
  - `Run` 注册表实现自启
  - 使用 `cmd /C` 执行 shell
- macOS
  - 托盘使用 `tray-icon`
  - 由于 AppKit/事件循环限制，采用核心进程 + 托盘子进程隔离策略
  - 自启使用 `System Events` 登录项（login item）
- Linux
  - `~/.config/autostart/*.desktop` 实现自启
  - 使用 `xdg-open` 打开文件与 URL

### 3.3 Web 行为规范

- 所有 Web 操作必须异步提交，不允许跳转到 `/manual-login`、`/save-curl` 等动作路由页面
- Web 首页状态区仅展示“主状态”；“错误 / 最近 ping / 托盘状态”不在页面展示
- 运行日志与错误信息统一输出到终端（同时可保留文件日志）
- Web 页面不提供“打开 config”按钮

---

## 4) 开发规范

- 只做与需求直接相关的最小改动，避免大面积重构
- 修改跨平台逻辑时，优先使用 `cfg(target_os = "...")`
- 任何会影响稳定性的改动，需要提供降级路径（例如托盘失败时仍可 Web 操作）
- 新增功能时优先复用现有模块，不在 `main.rs` 堆砌复杂逻辑
- 代码保持无 panic 设计；错误优先转换为可通知、可展示的信息
- 若新增配置项，必须：
  - 更新默认值生成逻辑
  - 保持向后兼容（旧配置缺字段时可运行）

---

## 5) 使用的主要库

- `tray-icon`
  - 跨平台托盘菜单
- `tiny_http`
  - 极简本地 HTTP 服务
- `notify-rust`
  - 系统通知
- `objc2` + `objc2-app-kit`（仅 macOS）
  - AppKit 初始化与运行循环支持

> 不要添加与目标无关的 UI 框架、异步运行时或大型网络库。

---

## 6) 构建与发布要求

`Cargo.toml` 发布配置必须保留：

- `panic = "abort"`
- `lto = true`
- `opt-level = "z"`
- `codegen-units = 1`
- `strip = true`

Windows 需保持：

- `#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`

### macOS App Bundle 和图标

生成 macOS .app bundle 和配置应用图标时：

1. **自动化方式（推荐）**：
   ```bash
   ./scripts/build_app_bundle.sh
   ```
   脚本会自动处理以下工作：
   - 编译发布版本二进制
   - 转换任何 WebP 格式的图标为 PNG
   - 创建完整的 App Bundle 目录结构
   - 配置 Info.plist 和资源文件

2. **手动方式**：
   - 将应用图标放在 `src/Assets.xcassets/AppIcon.appiconset/` 目录
   - 支持的尺寸：16, 32, 64, 128, 256, 512, 1024 像素
   - 文件命名规范：`{size}-mac.png`
   - 在 Info.plist 中添加 `<key>CFBundleIconFile</key><string>AppIcon</string>`
   - macOS 会自动查找 `AppIcon.png` 或 `AppIcon.icns`

3. **图标格式要求**：
   - 必须是真实的 PNG 格式（不能是 WebP）
   - 建议使用 `Contents.json` 定义图标集（见 ICON_BUILD_GUIDE.md）
   - iconutil 转换可能因格式差异失败，优先使用 AppIcon.png 方式

4. **更新 App Bundle 时**：
   - 只需修改 `src/Assets.xcassets/AppIcon.appiconset/` 中的图标文件
   - 运行 `./scripts/build_app_bundle.sh` 重新生成 dist/ePortal Guard.app

详细说明见 [ICON_BUILD_GUIDE.md](ICON_BUILD_GUIDE.md)

---

## 7) 常见改动清单（给 Copilot）

当你实现新需求时，请按以下顺序检查：

1. 是否破坏“Web 不跳转”约束
2. 是否破坏“无 curl 时给出通知”的行为
3. 是否影响单实例锁恢复
4. 是否保持 Web 状态区仅展示主状态（不回显错误/ping/托盘）
5. 是否兼容 macOS 托盘失败时的降级能力
6. 是否仍可通过 `cargo check` 与 `cargo build --release`7. 修改图标时，是否正确更新了 src/Assets.xcassets/AppIcon.appiconset/ 中的 PNG 文件
8. 修改后是否运行了 `./scripts/build_app_bundle.sh` 来生成 dist/ePortal Guard.app
9. 是否验证了 dist/ePortal Guard.app 中 Resources/AppIcon.png 的存在和 Info.plist 的 CFBundleIconFile 配置
---

## 8) 禁止事项

- 禁止把用户 cURL 解析重写成自定义 HTTP 请求（会损失兼容性）
- 禁止引入会显著放大体积的依赖
- 禁止删除 Web 后门
- 禁止将托盘作为唯一控制入口

---

## 9) 当前开发进度及状态（2026-03-02）

### 已完成的主要功能和改动

#### 阶段 1：托盘 UI 和图标优化
- ✅ 托盘菜单简化：仅保留"打开控制面板"和"退出程序"两项
- ✅ 菜单项图标化：使用 bolt.png（黄色）和 log-out.png（红色）
- ✅ 托盘图标替换：从内置默认图标改为 earth.png（地球图标）
- ✅ PNG 图标内嵌：通过 `include_bytes!()` 在 tray.rs 中加载
- ✅ 跨平台兼容：Windows/Linux 降级到无图标模式正常运作
- **核心改动**：[src/tray.rs](src/tray.rs) - 完全重作菜单架构为 IconMenuItem 模式

#### 阶段 2：Bug 修复
- ✅ **关键修复：退出时的"Choose Application"弹窗**
  - 根本原因：notifier::notify() 在关闭时被调用，导致系统要求手动选择应用
  - 解决方案：移除 tray.rs、web.rs、main.rs 中所有退出路径的通知调用
- **文件改动**：tray.rs, web.rs, main.rs

#### 阶段 3：Web UI 简化
- ✅ 移除冗余状态列：删除错误提示、最近 ping、托盘状态三栏
- ✅ 删除"打开 config"按钮：简化 Web 页面操作
- ✅ 移除 `/open-config` 路由处理
- ✅ Web 页面仅显示主要状态和自启动切换
- **核心改动**：[src/web.rs](src/web.rs) - 简化 HTML/JavaScript 状态展示逻辑

#### 阶段 4：终端日志和启动参数系统
- ✅ 条件化终端输出：
  - 默认启动：无终端输出（0 字节）
  - 携带额外参数启动：自动启用终端日志
- ✅ 实现 `-help` 参数：显示完整的参数列表（中文）
- ✅ 原子化日志开关：使用 AtomicBool guard，无锁开销
- **核心改动**：
  - [src/main.rs](src/main.rs) - 参数解析和启动逻辑
  - [src/debuglog.rs](src/debuglog.rs) - 条件化控制台输出

#### 阶段 5：macOS App Bundle 和图标集成
- ✅ **自动化构建脚本**：[scripts/build_app_bundle.sh](scripts/build_app_bundle.sh)
  - 自动编译 Cargo Release
  - 创建完整 App Bundle 目录结构
  - WebP→PNG 图标转换（使用 sips）
  - Info.plist 自动生成
  - 资源文件复制和验证
- ✅ **图标系统集成**：
  - 标准位置：`src/Assets.xcassets/AppIcon.appiconset/`
  - 支持尺寸：16, 32, 64, 128, 256, 512, 1024 像素
  - 格式要求：PNG（WebP 自动转换）
  - App Bundle 配置：Info.plist 中 `CFBundleIconFile: AppIcon`
- ✅ **详细文档**：[ICON_BUILD_GUIDE.md](ICON_BUILD_GUIDE.md)
  - 图标更新指南
  - 故障排除章节
  - 最佳实践建议
- **核心改动**：
  - [.github/copilot-instructions.md](.github/copilot-instructions.md) 第 6 节已更新

### 验证清单

| 功能 | 状态 | 验证方式 |
|------|------|--------|
| 托盘菜单简化 | ✅ | 运行后检查菜单只有 2 项 |
| 图标加载 | ✅ | 日志输出 "tray icon loaded from embedded earth.png" |
| 退出弹窗修复 | ✅ | 运行后关闭，无系统对话框弹出 |
| Web 页面简化 | ✅ | 打开 127.0.0.1:18888，无错误/ping/托盘列 |
| 参数系统 | ✅ | `./target/release/eportal_guard -help` 显示帮助文本 |
| 无终端启动 | ✅ | 直接运行：0 字节 stderr；--debug 运行：500+ 字节 |
| App Bundle | ✅ | `open "dist/ePortal Guard.app"` 正常启动 |
| 图标显示 | ✅ | AppIcon.png 已复制到 Resources，Info.plist 已配置 |

### 跨平台适配情况

#### macOS
- ✅ 完全支持，已验证所有功能
- 托盘使用 tray-icon + muda
- App Bundle 标准化

#### Windows
- ✅ 托盘支持（基础菜单）
- ⚠️ 图标显示取决于 tray-icon 库支持
- 待测试

#### Linux
- ✅ 托盘支持（基础菜单）
- ⚠️ 图标显示取决于桌面环境和 tray-icon 库
- **待测试**（当前阶段）

### Linux 测试待办事项

1. **构建验证**
   - [ ] `cargo build --release` 在 Linux 环境成功
   - [ ] 检查是否有平台特定的编译错误

2. **运行验证**
   - [ ] 默认启动（无终端）
   - [ ] `--debug` 参数启动（有终端日志）
   - [ ] `-help` 参数显示帮助
   - [ ] 托盘图标显示（取决于 WM）
   - [ ] Web UI 访问 127.0.0.1:18888

3. **功能验证**
   - [ ] 网络巡检运作（ping 探测）
   - [ ] 自启动配置（~/.config/autostart/*.desktop）
   - [ ] cURL 登录命令执行

4. **已知限制**
   - App Bundle 仅 macOS 专用；Linux 不需要此形式
   - 托盘在 Linux Wayland 下可能需要额外处理

### 构建和发行流程

**macOS 流程**（已自动化）：
```bash
./scripts/build_app_bundle.sh
# 生成 dist/ePortal Guard.app（可直接双击运行）
```

**Linux 流程**（标准 Rust 二进制）：
```bash
cargo build --release
# 生成 target/release/eportal_guard（直接可运行）
```

**Windows 流程**（标准 Rust 二进制）：
```bash
cargo build --release
# 生成 target/release/eportal_guard.exe
```

### 代码质量指标

- ✅ `cargo check` 通过（无警告）
- ✅ `cargo build --release` 通过（优化关闭）
- ✅ 发布体积优化：LTO + panic=abort + opt-level=z + strip
- ✅ 单实例锁机制正常运作
- ✅ 跨平台兼容性维持

### 后续工作优先级

1. **Linux 环境验证**（当前阶段）
   - 构建、运行、托盘功能测试

2. **可选优化**（如需要）
   - 代码签名（macOS）：`codesign -s ...`
   - 分发打包（.dmg for macOS）
   - CI/CD 自动化（GitHub Actions）

3. **文档完善**（后续）
   - README.md 更新（发行说明）
   - 用户部署指南
   - 故障排除指南

