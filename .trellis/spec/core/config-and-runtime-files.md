# 配置与运行时文件

> 本应用无数据库。全部持久化是配置目录下的 4 个文件，约定都在这里。

## 配置目录与路径

路径解析收敛在 `src/paths.rs`，各平台遵循系统习惯：

- Windows：`%APPDATA%/eportal-guard`
- macOS：`~/Library/Application Support/eportal-guard`
- Linux：`$XDG_CONFIG_HOME/eportal-guard` 或 `~/.config/eportal-guard`

`paths.rs` 提供的 `config_path()` / `curl_path()` / `lock_path()` 是唯一路径来源。
**模块内不得自行拼 `env::var` 推导路径**；新增运行时文件时先在 `paths.rs` 加取路径函数。

首次启动由 `config.rs::ensure_files()` 创建文件，**已存在时不覆盖用户内容**。

## config.toml

三字段，由 `config.rs::AppConfig` 读写：

| 字段 | 含义 | 默认 | 约束 |
|------|------|------|------|
| `ping_interval_secs` | 监控轮询间隔（秒） | `3` | 加载/保存都 `clamp(1, 3600)` |
| `ping_host` | 兼容遗留字段，不再参与探测 | `"223.5.5.5"` | 仅非空校验 |
| `web_port` | 本机 Web 控制台端口 | `18888` | 须 `> 0` |

关键约定：

- **手写极小 TOML 子集**（`AppConfig::to_toml_string` + 逐行解析），刻意不引入 toml 依赖；
  新增字段必须同时改这两个函数与 `/save` 路由，并保持「字段解析失败回默认值」的容错语义
  （`load_config` 逐字段容错，配置局部损坏不影响启动）。
- **字段名兼容优先**：界面叫「探针间隔」，文件字段仍叫 `ping_interval_secs`，
  避免破坏用户已生成的 config.toml。改字段名会制造迁移负担，先确认收益。
- 写回是整文件覆盖（`save_config` → `fs::write`）。由于只有 Web `/save` 写、监控线程只读，
  无并发写；若未来新增写方，先加锁或临时文件+rename，不要假设 write 原子。
- 保存成功的用户反馈用系统通知（`notifier::notify("ePortal Guard", "配置更新成功")`），
  监控线程检测到值变化时也会弹「配置更新」。

## curl.txt

- 存用户从浏览器复制的**cURL 原文**，是登录请求的唯一数据源（`main.rs` 读、
  `/save-curl` 与 `/manual-login` 写/读）。
- 内容语义在 `login.rs`（解析规则见 network-and-login.md）；`curl.txt` 本身不做校验，
  空内容 = 未配置。监控线程检测到内容变化会重置登录失败计数并记录日志。

## app.lock（单实例锁）

- 由 `single_instance.rs` 维护，内容为 `pid=<u32>\nweb_port=<u16>\n`；
  `parse_pid` 仍兼容旧版只写 PID 的格式（向前兼容，勿删）。
- 生命周期：`SingleInstance::acquire` 用 `create_new` 原子创建，`Drop` 时删除。
  **进程退出路径必须经过 `Drop`** —— `/quit` 只把 `running` 置 false，由主线程自然退出
  释放锁，绝不直接 `process::exit`。
- 陈旧锁：PID 不存在即视为上次异常退出残留，允许删除重取（`process_exists` 用系统 API，
  不派生子进程）。第二个实例读旧实例 `web_port` 并代开 Web 控制台。

## debug.log

- 追加式文本日志，格式 `[HH:MM:SS][pid:<PID>][<组件>] <消息>`，见 logging-guidelines.md。
- 与其余文件不同，它**只写不读**，无大小轮转 —— 不要把它当查询接口。
