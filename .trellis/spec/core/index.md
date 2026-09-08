# ePortal Guard 核心开发指南

> 本层覆盖唯一的代码单元：`src/` 下的单个 Rust crate（`eportal_guard`，edition 2024）。
> 项目形态：macOS / Windows / Linux 上的校园网（锐捷 ePortal）自动登录守护程序，
> 无 GUI，提供本机 Web 控制台（`http://127.0.0.1:18888/`），所有产品文案面向中文用户。

## 架构一瞥

进程内三个线程 + 文件持久化，全部共享状态只有两处：

```
┌─ main 线程：启动流程 → start_monitor 后进入 running 轮询循环，/quit 置 false 后自然退出
│
├─ Web 线程（web.rs）：tiny_http 监听 127.0.0.1:<web_port>，每请求 catch_unwind 隔离
│
└─ 监控线程（main.rs::start_monitor）：每轮重读 config.toml 与 curl.txt，
   按「cURL 已配置 → 内网可达 → 外网不可达 → 发送登录 → 复查探针」状态机执行
```

- **持久层是文件，不是内存**：`config.toml`、`curl.txt`（用户 cURL 原文）、
  `app.lock`（单实例锁）、`debug.log`。Web 线程写文件，监控线程每轮重读并 diff，
  因此两个线程之间没有直接消息通道。
- **共享状态只有** `Arc<Mutex<web::SharedState>>`（一个 `status_text` 字符串，供页面轮询）
  与 `Arc<AtomicBool> running`（退出开关）。详见 [web-console.md](./web-console.md)。

详细规范按主题拆分，先读与本次改动最相关的文件：

| 主题 | 何时读 |
|------|--------|
| [directory-structure.md](./directory-structure.md) | 任何改动：模块职责、命名、跨模块调用边界 |
| [config-and-runtime-files.md](./config-and-runtime-files.md) | 动 config.toml / curl.txt / app.lock / 配置目录 |
| [network-and-login.md](./network-and-login.md) | 网络探针、登录请求、失败计数逻辑 |
| [web-console.md](./web-console.md) | HTTP 路由、页面/JS、状态文案、嵌入资源 |
| [platform-support.md](./platform-support.md) | 平台分发、自启 / 通知 / 打开 URL / FFI / 单实例 |
| [error-handling.md](./error-handling.md) | 错误返回、panic 边界、永不 panic 的代码路径 |
| [logging-guidelines.md](./logging-guidelines.md) | `debuglog::log` 用法、组件标签、日志禁令 |
| [quality-guidelines.md](./quality-guidelines.md) | 依赖纪律、测试、构建/发布、提交信息 |

## Pre-Development Checklist（写代码前必做）

1. 确认改动落在哪个模块（见 directory-structure.md 的模块表），若跨模块先确认归属。
2. 通读上表中与该主题对应的 guideline 文件 —— index 只是导航，规范在具体文件里。
3. 读 `.trellis/spec/guides/index.md`，并对照触发条件决定是否需要读两份思维指南。
4. 若改动涉及：新增/修改 `set_state` 的状态文案 → 核对 WEB_JS `statusToneMap`
   关键字（cross-layer 指南 §状态文案契约）；新增按钮/端点 → 同时改 `handle_request`
   match 分支与页面 JS；新增平台行为 → 三个 `cfg(target_os)` 分支 + 兜底都要有。
5. 复用优先：先在仓库里搜相似函数/常量（code-reuse 指南的重复模式地图）。

## Quality Check（提交前自检）

- [ ] `cargo test` 通过（现有单测在 login.rs / network.rs 模块内）。
- [ ] `cargo build --release` 通过；新代码不引入未使用的依赖。
- [ ] 状态文案、端点、日志组件标签等跨文件字符串没有漏改（先 grep 后提交）。
- [ ] 网络/文件/平台代码路径没有新增 `unwrap()` / `expect()` 与外部命令调用。
- [ ] 平台相关改动覆盖 Windows / macOS / Linux 三分支并有默认兜底。
- [ ] 注释与日志为中文、解释「为什么」而非复述代码；用户可见文案保持中文。

**语言约定**：代码注释、日志消息、用户可见文案、提交描述均为中文；
标识符、URL、技术术语保留英文。README 亦为中文，新增用户文档沿用。
