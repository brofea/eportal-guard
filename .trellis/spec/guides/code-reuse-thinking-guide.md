# 代码复用思维指南

> **目的**：写新代码前先确认仓库里是否已存在等价物。本仓库规模小，重复的代价是
> 「同一处修复漏掉另一份拷贝」——跨文件字符串耦合（端点、状态文案、组件标签）
> 尤其危险，因为没有编译器帮你查。

## 常量与字符串的唯一来源（先搜后改）

改动任何一处前先 grep 全仓库，确认有没有第二份：

| 值 | 唯一来源 | 改动影响面 |
|----|----------|-----------|
| 探针 URL（miui/huawei generate_204） | `network.rs` 顶部常量 | 网络探测行为 |
| `MAX_LOGIN_FAILURES`（=5） | `main.rs` 顶部 | 失败暂停阈值 |
| `WINDOWS_APP_ID`（=brofea.eportal_guard） | `notifier.rs` | Toast 归属 + 注册表键 |
| `APP_DIR_NAME` / `APP_RUN_KEY_NAME` | `paths.rs` | 全部运行时路径；`autostart.rs` 引 `APP_RUN_KEY_NAME` |
| `TUTORIAL_URL` | `web.rs` | 教程按钮 |
| HTTP 端点路径（`/save`、`/manual-login`…） | `web.rs` match 分支 **与** `WEB_JS` fetch | 双侧必须同步 |
| 状态文案（`status_text` 取值） | `main.rs` / `web.rs` 写点 **与** WEB_JS `statusToneMap` 关键字 | 关键字决定页面底色 |
| 日志组件标签（"监控" "网页"…） | `debuglog.rs` 日志行内 | 日志检索口径 |
| config.toml 字段名 | `config.rs` 读写 **与** `web.rs` `/save` 表单名 | 用户已有配置文件 |
| 发布命名（APP_ID/APP_NAME/APP_DISPLAY_NAME） | `.github/workflows/release-on-tag.yml` env | CI 产物名与元数据 |

「只在一个文件里搜到」≠ 安全：浏览器复制的 cURL 解析、表单字段等用户侧输入
会以字符串形式跨文件存在（如 `ping_interval_secs` 在 config.rs 与 web.rs 两处）。
**搜索时把 HTML/JS/TOML 文件都算上**。

## 本仓库的重复模式地图（新增第三处前先抽 helper）

- 「锁内比对状态并写日志」：`main.rs::set_state` 与 `web.rs::set_shared_status`
  逻辑相同（仅日志组件标签不同）—— 已两处；写第三处前抽
  `fn update_status(state: &Arc<Mutex<SharedState>>, component: &str, status: &str)`。
- 双探针明细日志的 12 行 `format!`：`main.rs::start_monitor` 在登录前/后各一份 ——
  改探针字段时要同步两份，或抽 `fn describe_probe(p: &network::HeadProbe) -> String`。
- Web 响应头构造（`Content-Type` header 逐路由重复）—— 新增路由不要再来一份，
  抽 `fn text_response(body: String) -> Response`。
- 转义函数：`web.rs::html_escape` 与 `autostart.rs::xml_escape` 转义集不同
  （HTML：`& < > "`；XML 另有 `'`），是各自用途的正确实现，别合并 —— 但新的
  HTML 模板插入点必须走 `html_escape`。
- 「某文件读全文、宽容解析」模式（`load_config` / `parse_curl` / `percent_decode`）：
  都已存在且带容错语义，新输入解析先复用，勿另造解析器。

## 何时抽象 / 何时不抽象

- **抽**：相同逻辑第 3 处出现、逻辑足够复杂容易改漏（如锁内状态更新）、
  跨文件字符串需同步。
- **不抽**：只用一次的小工具（如 `append_query` 只服务 `parse_curl`，留在 login.rs）；
  抽象引入的间接超过重复本身的代价；两组语义不同仅形似的代码（html vs xml 转义）。
- 抽离的 helper 放回**所属模块**（网络→network.rs、HTTP→login.rs、状态→main/web
  共享的归属按目录结构.md 的依赖方向定），不新建 `util.rs` 垃圾桶。

## 提交前检查

- [ ] 新常量/URL/端点/文案已 grep，无第二份需要同步
- [ ] 没复制第三个相似函数体（抽了 helper 或确认不值得抽）
- [ ] 引用的符号存在且属于正确的模块（未被重命名/移动）
