# Codex CLI local1 中文介绍

本仓库是 OpenAI Codex CLI 的 local1 维护分支，当前已同步到上游 `rust-v0.124.0`，同时继续保留 local1 定制功能。本仓库不是 OpenAI 官方发布渠道，官方上游仍是 `openai/codex`。

## 当前同步目标

- 上游版本：`rust-v0.124.0`
- local1 清单：`docs/local1-custom-feature-checklist-2026-03-28.md`
- local1 Release 说明：`docs/local1-release-rust-v0.124.0-2026-04-24.md`

## local1 保留能力

- CLI 帮助、TUI 状态区、历史单元、升级提示等版本展示继续显示 `<上游版本>-local1`，让使用者能直接看出当前是 local1 构建。
- `/responses` 主请求链继续保留更宽的临时失败重试能力，包括 local1 对 `401` 恢复与重试链日志降噪的处理。
- Provider runtime refresh 继续只刷新当前 provider 的 `base_url` 与 `experimental_bearer_token`，并保留 Windows tray 从配置复制 provider 字段后触发 refresh 的联动。
- Resume 历史列表默认可跨 provider 发现旧会话；Fork 场景仍保留当前 provider 过滤，避免把新分支接到错误来源上。
- `gpt-5.4` 默认继续走 local1 的 priority 兜底；顶层配置 `force_gpt54_priority_fallback = false` 会关闭该模型的 priority 兜底和 Fast 透传。
- 未显式设置 `RUST_LOG` 时，Windows app-server 与 TUI 默认保持日志降噪，减少普通使用时看到的后台噪声。
- brand-new 或 Clear 后的新普通线程，只有首个纯文本输入精确为 `你好` 时，才会在第一条可见 assistant 回复里注入 local1 清单；resume、fork、subagent、reviewer、guardian 以及不匹配的首轮输入都不会触发。

## 发布说明

本次发布通过 GitHub 仓库与 GitHub Release 记录完成，不使用本机系统打包二进制产物。远端编译与检查以 GitHub Actions 结果为准。
