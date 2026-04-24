# 0.124.0-local1 Release Notes

## English

This local1 release syncs the repository with upstream OpenAI Codex release `rust-v0.124.0` and keeps the local1 customization checklist in `docs/local1-custom-feature-checklist-2026-03-28.md`.

This GitHub Release is created through GitHub. It does not include locally built binary assets from this machine.

### Highlights

- Upstream sync target: `rust-v0.124.0`.
- Version surfaces continue to show the local build identity with the `-local1` suffix.
- `/responses` retry behavior, retry-chain log suppression, and local1 `401` recovery handling are retained.
- Provider runtime refresh remains limited to `base_url` and `experimental_bearer_token`, with Windows tray provider-copy integration retained.
- Resume history discovery remains cross-provider by default, while fork behavior keeps provider identity where needed.
- `gpt-5.4` priority fallback remains enabled by default and can still be disabled with `force_gpt54_priority_fallback = false`.
- Windows app-server and TUI default log noise reduction remains in place when `RUST_LOG` is not set.
- The local1 first-turn `你好` checklist injection remains limited to brand-new or cleared regular threads.

## 中文

本次 local1 发布将仓库同步到 OpenAI Codex 上游 `rust-v0.124.0`，并继续保留 `docs/local1-custom-feature-checklist-2026-03-28.md` 中定义的 local1 定制功能。

本 GitHub Release 通过 GitHub 创建，不包含从本机系统构建或上传的二进制资产。

### 重点

- 上游同步目标：`rust-v0.124.0`。
- 所有关键版本展示继续带 `-local1` 后缀，让用户能直接识别 local1 构建。
- `/responses` 重试行为、重试链日志降噪以及 local1 的 `401` 恢复处理继续保留。
- Provider runtime refresh 仍只刷新 `base_url` 与 `experimental_bearer_token`，并保留 Windows tray provider 字段复制联动。
- Resume 历史默认继续跨 provider 可发现；Fork 仍在需要时保留 provider 身份。
- `gpt-5.4` 默认继续启用 priority 兜底，并可通过 `force_gpt54_priority_fallback = false` 关闭。
- 未设置 `RUST_LOG` 时，Windows app-server 与 TUI 默认继续日志降噪。
- local1 首轮 `你好` 清单注入仍只限 brand-new 或 Clear 后的新普通线程触发。
