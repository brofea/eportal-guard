# 思维指南

> **目的**：动手前先想全。本仓库的 bug 大多不是「不会写」，而是没注意到跨文件耦合、
> 重复模式与平台分支遗漏。两份指南针对 eportal-guard 的真实耦合点，先读 index 再按需展开。

## 指南一览

| 指南 | 目的 | 适用时机 |
|------|------|----------|
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) | 减少重复逻辑，守住常量/字符串的唯一来源 | 写相似逻辑、改常量或用户可见文案、加工具函数 |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | 想清跨线程/文件/页面边界的契约 | 动状态文案、加端点/按钮、改监控状态机、碰 config/curl 文件 |

## 触发速查（命中任一条 → 读对应指南）

- 要新增状态文案、改 `set_state` 调用点 → Cross-Layer §状态文案契约
- 新增 Web 按钮/端点、改页面 JS → Cross-Layer §HTTP 端点的双侧耦合
- 改 config.toml 字段名/解析、curl.txt 语义 → Cross-Layer §文件是线程间契约
- 改失败计数/暂停逻辑 → Cross-Layer §状态机与失败计数
- 复制过相似函数、或发现第 3 处近似逻辑 → Code Reuse
- 改任何 `src/*.rs` 顶部常量、URL、`APP_DIR_NAME` 类字符串 → Code Reuse §先搜后改
- 新增日志组件标签/用户文案 → 先 grep 既有标签与文案再定稿

## 修改前必做（任何改动通用）

1. grep 定位所有引用点：常量、端点路径、状态文案关键字、组件标签、字段名。
2. 对照三平台 `#[cfg(target_os)]` 分支是否需要同步。
3. 改完跑 `cargo test`，涉及发布链路看 quality-guidelines 的构建章节。

## 沉淀习惯

踩到「没想全」的坑（例如新增状态忘更新色调关键字导致页面恒 idle），
把教训补进对应指南的「真实案例」小节，供后续会话复用。
