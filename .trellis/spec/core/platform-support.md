# 平台支持（Windows / macOS / Linux）

> 三平台机制完全不同，代码把差异收敛在四个模块：`autostart.rs`（自启）、
> `notifier.rs`（通知）、`platform.rs`（打开 URL）、`single_instance.rs`（进程探测），
> 外加 `paths.rs` / `debuglog.rs` 里的路径与本地时间。业务代码（main/web/监控）
> 只调用这些模块的平台无关入口，**不知道**具体平台实现。

## 平台分发模式

- 每个平台一套完整实现写成独立 `#[cfg(target_os = "...")]` **私有函数**，公共函数体内用
  `#[cfg]` 块逐个 return 分发，末尾放 `#[allow(unreachable_code)]` 兜底（未知平台返回
  默认值/错误，如 `autostart::is_enabled` 末尾 `false`）。三个分发函数在
  `autostart.rs::is_enabled`、`notifier.rs::notify`、`platform.rs::open_url` 可对照。
- **禁止**在公共函数里用 `if cfg!(...)` 包多套实现（死代码会全部参与类型检查，
  需要 `#[cfg]` 才 import 的符号会编译失败）；平台专属 import 用函数内
  `use`/`#[cfg]` 前缀，见 `notifier.rs::ensure_windows_app_id_registered`。
- 新增平台能力 = 新增一个模块或扩展现有模块的入口函数：**不要在 main.rs/web.rs 里写
  Windows 注册表或 plist 代码**。
- Linux 泛指非 win/mac 平台（`#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]`），
  notify 对 Linux 直接走 notify-rust，与 mac/win 结构一致。

## 各平台机制速查（改动前先对照现状）

| 能力 | Windows | macOS | Linux |
|------|---------|-------|-------|
| 自启 | HKCU `...\CurrentVersion\Run` 值 `ePortalGuard`（免管理员） | `~/Library/LaunchAgents/com.brofea.eportal-guard.plist`（RunAtLoad） | `~/.config/autostart/*.desktop`（XDG） |
| 通知 | notify-rust + AUMID `brofea.eportal_guard`（Toast 归属，先注册 `HKCU\SOFTWARE\Classes\AppUserModelId`） | notify-rust 原生 | notify-rust |
| 打开 URL | `Shell32.dll ShellExecuteW` | CoreFoundation + `ApplicationServices LSOpenCFURLRef` | **未实现**：记日志并返回 false（Linux 分支见 `platform.rs`，不调外部命令） |
| 进程存在 | Kernel32 `OpenProcess`+`GetExitCodeProcess` | POSIX `kill(pid, 0)` | 同左 |

> Linux 打开 URL 的现状是平台缺口而非笔误：`platform.rs` Linux 分支注释
> 「需要桌面门户支持，当前版本未调用外部命令」。补实现时保持零外部命令约束，
> 考虑 zbus/桌面门户 crate 或继续返回 false 并提示。

- 自启值/plist/.desktop 里的可执行路径来自 `std::env::current_exe()`，由调用方传入
  （`is_enabled(exe_path)` / `set_enabled(exe_path, ...)`）；不要模块内自取。
- plist/.desktop 里嵌路径需转义：macOS 用 `autostart.rs::xml_escape`，新增 XML 输出时复用。
- Windows 注册表/进程 API 的写法基准：`wide()` UTF-16 辅助 + 函数内 `#[link(name="...")]
  unsafe extern "system"` 声明 + 返回码显式比对（`ERROR_SUCCESS` 等常量本地定义），
  见 `single_instance.rs::process_exists` 与 `notifier.rs::set_reg_string`。
- 时间/路径等系统交互同理：`debuglog.rs::local_time_parts` 分 unix（`localtime_r`）与
  windows（`GetLocalTime`），不要引入 chrono 依赖。

## 硬性约束

- **不派生子进程完成系统动作** —— 当前全部平台实现均为系统 API / notify-rust，
  零外部命令（进程存在探测用 `OpenProcess`/`kill` 而非 `tasklist`/`ps`；通知不启子进程；
  Linux 打开 URL 直接返回 false 也不调 xdg-open）。新增实现延续这一约束，
  确需桌面能力时考虑系统库或 crate，而不是 `Command::new(...)`。
- FFI 一律在**函数局部**声明与调用，不建全局 `extern` 块（现状如此，减少符号泄漏面）。
- 修改任一处平台机制时，`autostart`/`notify`/`open_url` 三个入口的**其余平台分支**
  行为不得退化；CI 三平台矩阵（`.github/workflows/release-on-tag.yml`）会分别构建，
  本地无法交叉编译时以 `cargo check --target ...` 验证对应平台代码（需要 rustup target）。
