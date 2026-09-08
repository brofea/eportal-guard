# 日志规范

> 唯一日志通道是 `src/debuglog.rs`，无第三方日志库。`debuglog::log` 同时写终端
> （stderr）与配置目录下 `debug.log`，日志路径在首次调用时延迟初始化（`OnceLock`）。

## 调用方式

```rust
debuglog::log("监控", &format!("本轮检查 | cURL 已配置: {} | 间隔: {} 秒", ok, secs));
```

- 组件标签是固定中文短名词，先 grep 复用既有标签，不要自创同义标签：

| 标签 | 使用方 |
|------|--------|
| `主进程` | main.rs 进程级事件（启动/致命错误/结束） |
| `核心` | main.rs 核心流程初始化/单实例结果 |
| `监控` | main.rs 监控线程每轮检查、登录成败、失败计数 |
| `网页` | web.rs 收到的请求、状态变化、panic |
| `状态` | main.rs 状态机 `set_state` 的状态迁移 |
| `平台` | platform.rs（如 Linux open_url 未实现提示） |
| `通知` | notifier.rs 通知发送失败/panic/Windows AUMID 注册失败 |

- 消息内容是中文、可读性优先，常用 `|` 分隔字段；整条 `log` 是单行
  （`debug.log` 为逐行追加，不要在消息里换行）。
- 变更先 `format!` 再传，禁止把 `{}` 裸插值传给 `log`（没有格式化上下文）。

## 什么该记

- 监控线程的**决策依据**：每轮的内网状态、双探针明细（状态码/耗时/错误）、
  是否跳过、登录前后复查结果 —— 现有格式见 `main.rs::start_monitor`，
  改状态机时保持「一次操作一条可追溯日志」的信息量。
- 状态迁移与 Web 侧即时操作（`状态变化: X -> Y`）。
- 所有 `Err` 分支的失败原因（含探针 `error_message`、通知失败、表单错误）。

## 什么不该记（敏感数据禁令）

- **绝不记录 cURL 内容、请求体、Cookie/认证头、密码等凭据**。cURL 是登录凭据载体，
  现有代码只记 `method URL HTTP <status>` 级别信息（`login.rs` 的 `LoginResult`）。
  新增字段时若含凭据，只能进内存、不得进日志。
- 不记录 `debug.log` 的完整路径依赖日志本身的绝对路径推导，避免把用户主目录写进日志。
- 不记录可能被认证页回显的原始响应体（探针只记状态码与错误串，不记响应内容）。

## 开关

- `debuglog::set_console_enabled(false)` 用于关闭终端回显（Windows 无控制台构建时由
  `main.rs` 的 `windows_subsystem` 处理）；`debug.log` 始终写入。不要在业务逻辑里
  频繁翻转该开关。
