# 错误处理

> 应用是常驻守护进程：一个未捕获错误可能让用户在断网时「悄悄」失联，
> 因此错误处理约定围绕「能自愈的自愈、能通知的通知、绝不静默吞掉致命错误」展开。

## 错误表示

- 函数返回 `Result<T, String>`，错误消息是**给用户看的中文**（如
  `"cURL 命令缺少 URL"`、`"参数 -H 缺少值"`），不做类型化错误枚举 ——
  当前规模下 `String` 是仓库既有惯例（`run() -> Result<(), String>`、
  `send_curl_request() -> Result<LoginResult, String>`），新代码沿用，不要引入
  `thiserror`/`anyhow`。
- 文件 IO 保持 `io::Result`（`config.rs::save_config`、`single_instance::acquire`），
  由调用方决定转 String 或传播。
- 关键失败「必达用户」：`main.rs::main` 捕获 `run()` 错误后既写日志又弹系统通知
  （`"启动失败: {e}"`）；Web 路由失败把错误串放进响应体并配 `500`。
- 可自愈失败**不打扰用户**：如探针超时只是状态机的一个分支，走日志 + 状态文案。

## panic 边界（重要）

守护进程里任何线程的 panic 都不能带走整个进程：

- **Web 线程**：`web.rs::start_web_server` 对每个请求包 `panic::catch_unwind(
  AssertUnwindSafe(...))`，panic 后仅记日志（`"请求处理发生 panic: …"`）继续服务。
  新增路由处理器自动落在这个边界内，无需自加 catch。
- **通知线程内**：`notifier.rs::notify` 用 `catch_unwind` 包住 `notify-rust` 调用，
  避免通知库在无桌面会话（如 Linux SSH）时 panic 拖垮主流程。
- **主线程**：`main()` 不 catch panic（有 debug.log 兜底 + 系统通知），但正常代码路径
  不得依赖 unwind —— 资源清理（锁文件）靠 `Drop`，panic 时同样会执行。

## 明确禁止

- 对网络/文件/平台 API 的 `unwrap()` / `expect()`；全仓库现有代码均以 `match`/`if let`
  或 `map_err(|e| e.to_string())?` 处理（如 `single_instance.rs::process_exists` 全平台
  无 panic 写法）。此路径含 `fs::read_to_string(...).ok()` 这类宽容降级，允许。
- 在请求处理或监控循环里 `panic!`/`assert!` 表达业务失败 —— 那是错误分支，不是不变量。
- 静默 `let _ = ...` 吞掉的是「响应发送失败」这类次要 IO（仓库既有 `let _ = req.respond(...)`），
  其余错误要么日志、要么返回给调用方；不要新增吞错误点。
