# 网络探测与登录

> 探测在 `src/network.rs`，登录请求解析/发送在 `src/login.rs`。两条规则贯穿全部网络代码：
> **不派生子进程执行 curl/ping**（全部用 Rust HTTP 客户端）；**走直连、不走系统代理**
> （`reqwest` 客户端一律 `.no_proxy()`）。新增 HTTP 出口只许进这两个模块。

## 连通性探测（network.rs）

- **外网不可达判定 = 双探针都失败**（`internet_probe`：`miui.ok || huawei.ok`），
  探针 URL 是两个厂商的 `generate_204` 端点，常量在 `network.rs`（勿跨文件复制）。
- **204 语义是硬契约**：`generate_204_status_ok` 只认 `status == 204` —— 认证页劫持会
  返回 200/302，必须判失败（有单测 `generate_204_probe_requires_204_to_avoid_captive_portal_false_positive`）。
  新增探针端点也必须走 204 语义并加同款单测。
- 单个探针：`Client::builder().timeout(3s).redirect(Policy::none()).no_proxy()` 发 HEAD。
  禁自动跟随重定向，否则会被劫持页带偏；`HeadProbe` 记录 ok/status/elapsed/error 供日志。
- **内网可达判定** = 本机默认出口 IP 是 IPv4 私网地址（RFC1918：10/8、172.16/12、192.168/16）。
  实现是 `UdpSocket` connect 到 `8.8.8.8:80` 取 `local_addr`，不真正发包；IPv6 一律不算内网。
- 探针只读状态码与错误串，**不记录/不回显响应体**（响应可能被劫持页污染）。

## 登录请求（login.rs）

- 输入是 curl.txt 里的 bash 格式 cURL 原文，`send_curl_request` 依次：`parse_curl` →
  组装 `reqwest` 阻塞客户端 → 发送 → 返回 `LoginResult { method, url, status }`（无响应体）。
- 客户端固定参数：`timeout(10s)`、`redirect(Policy::limited(10))`、`no_proxy()`，
  以及 `danger_accept_invalid_certs(parsed.accept_invalid_certs)` —— `-k/--insecure` 由
  用户 cURL 显式控制，未声明就按校验证书处理。
- 解析器刻意实现 cURL **子集**（`parse_curl` + `shell_words`），范围以 tests 为准：
  `-X/--request`、`-H/--header`（含粘连短参）、`-A`/`-e`/`-b` → 映射 User-Agent/Referer/
  Cookie 头、`-d/--data*` 系列 body、`-G` 把 data 拼到 query、`-I`、`-k`、`--url`，
  静默忽略 `-s -S -i -L --compressed` 等，`-o/-m/--connect-timeout` 只消费值。
  **未列出的参数直接报错**（"暂不支持的 cURL 参数"），不要悄悄吞掉未知语义。
- 防注入与脏数据：跳过用户头里的 `Host`/`Content-Length`（`build_headers`，防止伪造
  目标/长度头）；`--data @file` 明确拒绝（"暂不支持从文件读取请求体"）；
  请求体按 `&` 拼接（`join_body_parts`）；未闭合引号报错；容忍 `$ ` 前缀与 BOM、
  反斜杠换行的多行 cURL（浏览器「复制为 cURL」产物）。
- 复用既有解析能力时改 `parse_curl`/`shell_words` + tests，不要在新文件里再造一个分词器。

## 监控线程状态机（main.rs::start_monitor）

顺序为：读配置 → 读 cURL → **空则跳过并置状态「未配置 cURL，等待配置」** →
内网不可达跳过 → 外网可达则 `reset_login_failures` + 「网络正常」→ 否则发送登录 →
**登录后必须复查探针**（以复查结果为准判成败）。

- 失败计数语义（`register_login_failure`）：连续失败累加，`>= MAX_LOGIN_FAILURES(5)`
  时只弹**一次**系统通知并置「无法登陆，已暂停重试」；cURL 内容变化、网络恢复、内网断开
  都会 `reset_login_failures` 重新计数。改动任何「重置/暂停」触发条件时，先对照上述三个
  重置点，防止计数永不复位。
- 轮询间隔取 `cfg.ping_interval_secs.max(1)`（配置 `clamp` 之外再加运行时下限）。
- 本轮循环内**不要加 sleep 之外的长阻塞**（如串行多个 3s 探针×N 会拖长单轮，
  现有实现每轮最多 2 探针 + 1 登录 + 2 复查探针）。

## 测试范式

- 纯逻辑（无 IO/网络）才有单测价值：`login.rs::tests`（curl 解析三例）与
  `network.rs::tests`（204 判定）是现有范本。探针/登录的真实网络路径不做自动测试，
  靠日志人工核对；新增解析分支照 `parse_browser_bash_curl_post` 的形态补例。
