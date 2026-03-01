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
- `/status` 作为状态轮询接口，至少返回：
  - 主状态
  - 最近错误
  - 最近 ping 结果（含耗时）
  - 托盘状态

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

---

## 7) 常见改动清单（给 Copilot）

当你实现新需求时，请按以下顺序检查：

1. 是否破坏“Web 不跳转”约束
2. 是否破坏“无 curl 时给出通知”的行为
3. 是否影响单实例锁恢复
4. 是否保持 ping 结果（含耗时）可在 `/status` 看到
5. 是否兼容 macOS 托盘失败时的降级能力
6. 是否仍可通过 `cargo check` 与 `cargo build --release`

---

## 8) 禁止事项

- 禁止把用户 cURL 解析重写成自定义 HTTP 请求（会损失兼容性）
- 禁止引入会显著放大体积的依赖
- 禁止删除 Web 后门
- 禁止将托盘作为唯一控制入口
