# 质量规范

> 本应用的代码质量基准：极简依赖、守护进程级健壮性、中文可读日志、纯逻辑单测。

## 依赖纪律（当前 Cargo.toml 仅 3 个直接依赖）

- `reqwest 0.12`（blocking + rustls-tls，default-features 关闭）、`tiny_http 0.12`、
  `notify-rust 4`（default-features 关闭；Linux target 额外开 `z` 特性）。
- **新增依赖前先自问**：是否能用系统 API / std / 既有 crate 实现？（先例：
  config.toml 手写子集避免 toml 依赖、本地时间用 FFI 避免 chrono。）
  必须新增时保持 default-features 关闭 + 只开所需特性，并在提交说明里写理由。
- 编译期约束看 `Cargo.toml` 的 `[profile.release]`：`lto`、`opt-level="z"`、
  `codegen-units=1`、`strip` —— 体积/大小优化是发布目标，别随意改。

## 代码风格

- edition 2024；注释说明「为什么」，不逐行复述代码；注释与日志中文（见 index 语言约定）。
- 不写死魔法值：`MAX_LOGIN_FAILURES = 5`、探针超时 3s、登录超时 10s、轮询下限 1s、
  端口 18888 等现有常量改用时先在所属模块确认是否已有符号。
- 命名：类型/函数 snake_case、常量 SCREAMING_SNAKE_CASE、公开 API 才 `pub`；
  模块间尽量只暴露入口函数与必要结构体（如 `ParsedCurl` 保持私有）。

## 测试要求

- 单测只写给「无 IO、无网络、可离线确定性验证」的纯逻辑，集中在 `login.rs`（curl 解析）
  与 `network.rs`（204 判定）模块内 `#[cfg(test)] mod tests`；文件末尾追加，
  `use super::*;`。
- 有意义的行为断言（方法推导、URL 拼装、body 字节、204/200/302 判定），
  不写无谓的 smoke 测试。验证命令：`cargo test`（仓库无集成测试目录）。
- 真实网络/系统调用路径不做自动化测试，靠 `debug.log` 人工核对 —— 改动探针/登录/
  平台代码后至少本地 `cargo run` 一次走通主流程。

## 构建与发布（改动涉及打包时）

- 本地验证：`cargo build --release` 后跑 `--help` 自检（CI 也以此冒烟）。
- 发布链路（勿在功能 PR 中破坏）：`.github/workflows/release-on-tag.yml` 打 `v*` tag 触发，
  产出 Windows EXE（rcedit 打图标/版本）、macOS universal DMG（lipo +
  `scripts/build_app_bundle.sh` 生成 `.app` 与 Info.plist 并 ad-hoc 签名）、
  Linux AppImage/DEB/RPM。脚本内 `APP_ID=com.brofea.eportal_guard`、
  `APP_NAME=eportal_guard`、`APP_DISPLAY_NAME=ePortal Guard` env 是发布命名单一来源，
  改动命名时三平台 job 同步。
- 验证脚本校验产物元数据（版本号、作者、图标存在）后才上传；`dist/` 由脚本生成，
  已被 .gitignore 排除。

## 提交信息

仓库历史（`git log --oneline`）约定：`type(scope): 中文描述`，type 为
`feat`/`fix`/`docs`/`style`/`refactor` 等，scope 如 `readme`；描述用中文。
改动 `.trellis/` 的提交沿用此格式（如 `feat: 初始化 Trellis`）。

## 反模式清单（改动时警惕既有坏味道，新增代码不犯）

1. 复制 `set_state` / `set_shared_status` 这类「锁内比对更新」逻辑到第三处 —— 该抽 helper。
2. 在 `main.rs` 探针日志处把 12 行双探针 format 串再复制一份 —— 结构信息量虽高，
   新日志若相似度高应抽私有格式化函数（现有两处已是历史重复）。
3. 新增状态文案不含 `statusToneMap` 关键字（视觉失联）、新增端点漏改 match 分支或页面。
4. 任何 `unwrap()`/`expect()` 落在网络/文件/平台 IO 路径；外部命令执行（curl/ping）。
5. 平台逻辑下沉进业务模块 / `if cfg!` 混排平台实现 / 新依赖未关 default-features。
