# 升级到 upstream `rust-v0.120.0` 并保留 `local1`_TASK_2026-04-12

文档目的：冻结后续把当前 `I:\vscodeProject\codex` 升级到官方 `openai/codex` `rust-v0.120.0`、同时保留 `local1` 长期定制能力的执行口径，并把本轮代码审核阶段的实际审计结果、reviewer subagent 复核结果和主 agent 处理结论回写到同一文件。本文不是“升级已完成”证明。

主产物位置：`I:\vscodeProject\codex\.codexflow\临时\升级到upstream_rust-v0.120.0并保留local1_2026-04-12\升级到upstream_rust-v0.120.0并保留local1_TASK_2026-04-12.md`

对应长期基线文档：`I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`

官方依据：

- `rust-v0.119.0` release：<https://github.com/openai/codex/releases/tag/rust-v0.119.0>
- `rust-v0.120.0` release：<https://github.com/openai/codex/releases/tag/rust-v0.120.0>
- `rust-v0.118.0...rust-v0.120.0` compare：<https://github.com/openai/codex/compare/rust-v0.118.0...rust-v0.120.0>

## Context

- 当前项目根目录固定为 `I:\vscodeProject\codex`。
- 当前 Git 远端只有 `origin = https://github.com/dqIndieGames/codex.git`，没有配置 `upstream`。
- 当前分支状态是 `main...origin/main`；本轮代码审核时的工作树状态为：
  - `M codex-rs/Cargo.toml`
  - `M codex-rs/Cargo.lock`
  - `?? .codexflow/临时/升级到upstream_rust-v0.120.0并保留local1_2026-04-12/`
  - `?? tmp/`
- 当前代码树的历史功能基线仍按 `0.118.0 + local1` 评估，但不能再把现在的工作树表述成“纯 0.118.0 老树”，因为静态审计显示本地已经提前吸收了多项 `0.119.0/0.120.0` 时代结构与能力。
- 当前 `codex-rs/Cargo.toml` 与 `codex-rs/Cargo.lock` 的版本元数据已经在本轮代码审核中被提升到 `0.120.0`；这只说明版本元数据发生了变化，不等于整个源码树已经完成对齐官方 `rust-v0.120.0`。
- 目标官方版本固定为 `openai/codex` `rust-v0.120.0`，发布时间为 `2026-04-11`。由于本地历史基线来自 `0.118.0`，后续升级不能跳过 `rust-v0.119.0` 的官方增量。
- 官方 `0.118.0 -> 0.120.0` 跨度已确认很大：`303` commits；官方 compare 页面显示 `1,043 files changed`；GitHub compare API 的 `files` 列表只返回前 `300` 条，属于截断值，不能当作真实文件总量。
- 本轮用户边界固定为：允许源码修改与 TASK 回写；禁止编译、测试、格式化、lint、build；禁止未授权的 Git 高风险操作。
- 本轮 reviewer subagent 必须把复核结论追加到同一 TASK 文件末尾，主 agent 必须回读同一文件并逐条写出 `采纳/不采纳`、原因、改写位置和残余风险。

## Goal

- 产出一份后续实现者可直接执行的升级 TASK 文档，不再需要重新猜测目标 tag、官方参考源、`local1` 保留范围、冲突热点和验收口径。
- 明确后续升级目标是“对齐官方 `rust-v0.120.0` 的有效增量，同时完整保留 `local1`”，不是把当前仓库重置成纯官方树。
- 明确本轮代码审核阶段的真实结论：当前实际落地的源码改动仅确认在 `Cargo.toml` / `Cargo.lock`，且只完成了最小版本元数据提升，不代表全量升级完成。
- 保留并冻结 `local1` 的 `F1-F15` 和 `2026-04-10` 归档补充 `A1/A2`，不允许后续实现阶段以“更接近 upstream”为理由削弱这些本地能力。
- 建立 reviewer subagent 闭环：reviewer 在同一文件追加详细问题清单与修改建议，主 agent 回读并决定采纳/不采纳，然后复写同一文件。

## 输入真值与证据来源

| 真值项 | 当前结论 | 证据/来源 |
|---|---|---|
| 项目根目录 | `I:\vscodeProject\codex` | 当前工作目录 |
| 当前 Git remote | 只有 `origin`，没有 `upstream` | `git remote -v` |
| 当前分支状态 | `main...origin/main` | `git status --short --branch` |
| 当前工作树新增源码改动 | 仅见 `codex-rs/Cargo.toml`、`codex-rs/Cargo.lock` 两个已修改文件 | `git diff --stat -- codex-rs/Cargo.toml codex-rs/Cargo.lock` |
| 当前历史功能基线 | `0.118.0 + local1`，但当前树已包含部分 `0.119.0/0.120.0` 时代能力 | 既有 TASK 文档审计、`local1` checklist、源码静态检索 |
| 当前版本元数据状态 | `Cargo.toml` workspace version 已是 `0.120.0`；`Cargo.lock` 内部 workspace crate 版本条目已同步到 `0.120.0` | `codex-rs/Cargo.toml`、`codex-rs/Cargo.lock` |
| 目标官方 release | `rust-v0.120.0` / `0.120.0` | 官方 release 页 |
| 中间官方 release | `rust-v0.119.0` / `0.119.0` | 官方 release 页 |
| 目标发布时间 | `rust-v0.120.0` 发布于 `2026-04-11`；`rust-v0.119.0` 发布于 `2026-04-10` | 官方 release 页 |
| 官方升级跨度 | `303` commits；官方 compare 页面显示 `1,043 files changed`；compare API `files` 列表在 `300` 处截断 | 官方 compare 页面与先前审计记录 |
| `local1` 长期真值 | 以 `docs/local1-custom-feature-checklist-2026-03-28.md` 的 `F1-F15` 与 `A1/A2` 为准 | 本地 UTF-8 读取 |
| 官方只读参考源 | `I:\vscodeProject\codex\tmp\agent-snapshots\upstream-rust-v0.120.0-2026-04-12\` 下的 `rust-v0.118.0` / `0.119.0` / `0.120.0` zip 与展开快照 | 本地目录读取 |
| 本轮执行边界 | 只做静态代码审核、必要源码改动与 TASK 回写；不编译、不测试、不 fmt、不 lint、不 build | 用户当前对话指令 |

## 官方 0.119.0/0.120.0 升级范围

### `rust-v0.119.0` 必须纳入的官方增量

- realtime v2 / WebRTC 路径增强，包括 transport、voice selection、native TUI media support 和 app-server 覆盖。
- MCP apps 与 custom MCP servers 扩展，包括 resource read、tool-call metadata、custom-server tool search、server-driven elicitations、file 参数上传与 plugin cache refresh 稳定性。
- remote / app-server workflow 增强，包括 egress websocket transport、remote `--cd` forwarding、remote-control enablement、sandbox-aware filesystem APIs 和实验性 `codex exec-server`。
- `/resume` 能力增强，支持按 session ID 或名字定位。
- `/status` stale limits 修复，避免过期 quota 信息误导。
- resume 稳定性增强，包括 picker false empty state、thread name / timestamp 展示和当前 thread 恢复崩溃修复。
- TUI / logging / sandbox / platform 修复，包括 paste、CJK navigation、clipboard、bubblewrap、macOS HTTP sandbox panic、Windows firewall 等。
- app-server-backed TUI session 下 `/fast off` 卡住问题修复。
- MCP 启动与 inventory 降噪、disabled server auth probing 优化。
- `codex-core` 周边 crate 抽离加剧，导致后续智能合并的文件归属与冲突面显著增加。

### `rust-v0.120.0` 必须纳入的官方增量

- realtime background agent progress 与 active response follow-up queue。
- hook 状态渲染增强，running / completed hooks 更易扫描。
- 可配置状态线支持线程标题。
- tool `outputSchema` 展示增强，structured tool result 类型更精确。
- `/clear` 触发的 SessionStart source 区分增强。
- Windows sandbox carveouts / writable roots / symlink 修复。
- remote websocket TLS Rustls crypto provider 修复。
- tool search result order 保持原顺序。
- Stop-hook prompt 即时展示。
- app-server MCP cleanup 修复 disconnect 后残留订阅问题。
- rollout recorder reliability 改进，flush failure retry 与 durability failure 暴露。
- analytics / Guardian review metadata wiring 增强，以及 guardian follow-up transcript delta 优化。

## `local1` 保留矩阵

| ID | 必须保留的口径 | 高风险冲突面 |
|---|---|---|
| F1 | CLI/TUI 所有用户可见版本输出统一保留 `<官方版本>-local1`；当前目标版本固定为 `0.120.0-local1`，不能退化成裸 `0.120.0`。 | `codex-rs/cli/src/main.rs`、`codex-rs/tui/src/version.rs` |
| F2 | 状态卡片、顶部状态区、历史单元、升级提示等版本展示统一消费 `CODEX_CLI_DISPLAY_VERSION` 或文档中点名的唯一共享显示源。 | `codex-rs/tui/src/status/card.rs`、`codex-rs/tui/src/app.rs`、`codex-rs/tui/src/chatwidget/status_surfaces.rs` |
| F3 | 历史消息、升级提示、版本跳转提示继续带 `local1` 身份。 | `codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/chatwidget.rs`、`codex-rs/tui/src/update_prompt.rs` |
| F4 | 与 `local1` 版本展示相关的快照和断言必须保留或迁移。 | TUI 快照与测试模块 |
| F5 | `/responses` 主链 HTTP 状态继续统一自动重试，包含 `401`；非 `/responses` 端点继续保留旧 whitelist。 | `codex-rs/codex-api/src/provider.rs` |
| F6 | retry / reconnect 中间态继续只更新状态区与详情，不进入历史；log/trace suppress 与 metrics 保留。 | `codex-rs/core/src/client.rs`、`codex-rs/core/src/codex.rs`、`codex-rs/codex-api/src/telemetry.rs` |
| F7 | 单次重试等待时间上限仍是 `10s`。 | `codex-rs/core/src/util.rs`、`codex-rs/codex-client/src/retry.rs` |
| F8 | 重试预算语义不因错误分类扩展而被改变；`401` 继续走普通 retry budget。 | `codex-rs/core/src/model_provider_info.rs`、`codex-rs/core/src/client.rs` |
| F9 | retry/reconnect 分类入口继续统一，不回退到散落白名单或 URL 猜测。 | `codex-rs/codex-api/src/provider.rs`、`codex-rs/core/src/api_bridge.rs` |
| F10 | `thread/providerRuntime/refresh` 与 `refreshAllLoaded` 继续存在，只刷新 `base_url` 与 `experimental_bearer_token`；Windows tray 联动继续保留。 | `codex-rs/app-server/src/windows_control.rs`、`codex-rs/app-server/src/message_processor.rs`、`scripts/windows_app_server_refresh_tray.py` |
| F11 | 历史默认不按 provider 分割，同时继续保留 cwd / `--all` / `show_all` 语义；`thread/list` 保留 `model_provider`。 | `codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/tui/src/lib.rs`、`codex-rs/tui/src/resume_picker.rs` |
| F12 | `/responses` stream / websocket / reconnect / fallback 路径里的 `401` 继续直接走普通 retry，不恢复 unauthorized recovery 优先分支。 | `codex-rs/core/src/client.rs`、`codex-rs/core/src/codex.rs` |
| F13 | `force_gpt54_priority_fallback` 继续只允许出现在顶层 `config.toml`；省略与 `true` 等价；显式 `false` 同时关闭 priority fallback 与 `Fast` 透传，`Flex` 保留。 | `codex-rs/core/src/config/mod.rs`、`codex-rs/core/config.schema.json`、`codex-rs/core/src/client.rs` |
| F14 | Windows app / app-server 默认日志继续降噪；未设置 `RUST_LOG` 时不应高详细度刷盘，显式 `RUST_LOG` 仍可恢复。 | `codex-rs/app-server/src/lib.rs` |
| F15 | TUI 默认日志继续降噪；`codex-tui.log` 与 sqlite log layer 在未设置 `RUST_LOG` 时保持低噪，显式 `RUST_LOG` 仍可恢复。 | `codex-rs/tui/src/lib.rs`、`docs/install.md` |
| A1 | 首次对话固定清单继续定义为“新对话第一条用户消息提交后插入一次”；不退化成启动 banner、hook 或随机提示。 | `codex-rs/tui/src/chatwidget.rs`、`codex-rs/tui/src/history_cell.rs` |
| A2 | `force_gpt54_priority_fallback` 归档口径继续严格：顶层 only，profile 无效。 | `codex-rs/core/src/config/mod.rs`、`codex-rs/core/config.schema.json` |

## Public Interfaces 冻结

- 顶层配置字段 `force_gpt54_priority_fallback` 继续只允许出现在顶层 `config.toml`。
- `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 继续保留，且热刷新范围仍只限 `base_url` 与 `experimental_bearer_token`。
- `thread/list` 返回项继续保留 `model_provider`。
- 首次对话固定清单继续定义为“新对话第一条用户消息提交后插入一次”，不得退化成启动提示、hook 或随机 banner。
- CLI/TUI 用户可见版本链继续统一保留 `0.120.0-local1`。
- `/responses` 主链继续保留现有 `401` 普通 retry、retry 中间态不入历史、`10s` cap、日志降噪等 `local1` 口径。

## 智能合并策略

1. 后续实现阶段以官方 `rust-v0.120.0` 为最终目标真值，不使用“Releases 698”之类页面计数作为唯一标识。
2. 先整理 `rust-v0.118.0 -> rust-v0.119.0` 和 `rust-v0.119.0 -> rust-v0.120.0` 的官方增量，再映射到当前 `0.118.0 + local1 + 部分前移能力` 的工作树。
3. 先吸收官方新增与修复，再逐条回补 `local1` 保留点；不能用“更接近 upstream”替代 `local1` checklist。
4. 任一与 `local1` 冻结口径冲突的 hunk，以 `docs/local1-custom-feature-checklist-2026-03-28.md` 为准。
5. 在没有用户明确确认前，默认只使用官方 release notes、官方 compare 页面和下载到 `tmp/agent-snapshots` 的 `.diff/.patch` / zip 快照作为只读参考源；不创建持久 `upstream` remote，不向当前仓库写入 fetch refs。
6. 不在未获授权时创建或切换分支、checkout tag、创建或切换 worktree。
7. 当前工作树已经包含部分 `0.119.0/0.120.0` 时代结构，因此后续实现必须先做差异映射，不允许机械用官方目录整块覆盖热点文件。
8. 对 `codex-core` 抽离相关冲突，优先追踪官方新 crate 归属，不把被抽离逻辑粗暴塞回 `codex-core`。
9. 对 TUI、app-server、realtime/websocket、config schema 和 docs 的冲突，必须同时检查代码、用户可见文案和相关文档说明。
10. 合并完成后的验收必须同时满足两件事：官方 `0.119.0/0.120.0` 有效增量已吸收，`local1` 冻结能力未丢失。

## 热点文件与冲突面

### 官方直接热点

| 文件 | 风险说明 |
|---|---|
| `codex-rs/app-server/src/codex_message_processor.rs` | app-server、remote、MCP、provider/history 语义集中，直接影响 F10/F11/A1。 |
| `codex-rs/app-server/src/message_processor.rs` | app-server message pipeline 与 tray / refresh 控制面交叉。 |
| `codex-rs/app-server/src/lib.rs` | app-server bootstrap、logging、Windows control plane 交汇，直接影响 F14。 |
| `codex-rs/core/src/client.rs` | retry、websocket、priority/service tier、remote TLS 修复集中，直接影响 F5-F9/F12/F13。 |
| `codex-rs/core/config.schema.json` | config schema 易被官方更新覆盖，直接影响 F13/A2。 |

### `local1` 复核热点

| 文件 | 风险说明 |
|---|---|
| `codex-rs/tui/src/chatwidget.rs` | 首次对话固定清单与 chat flow 绑定，直接影响 A1。 |
| `codex-rs/tui/src/history_cell.rs` | local1 清单、版本展示、升级提示链条集中。 |
| `codex-rs/tui/src/version.rs` | TUI 共享版本显示源。 |
| `codex-rs/cli/src/main.rs` | CLI 帮助与版本显示源。 |
| `docs/local1-custom-feature-checklist-2026-03-28.md` | 本地长期真值唯一基线。 |

### 低冲突但必须回归

| 文件 | 回归点 |
|---|---|
| `codex-rs/codex-api/src/provider.rs` | `/responses` retry 分类与 `401` 语义。 |
| `codex-rs/core/src/codex.rs` | turn-level retry、pending SessionStart source、provider runtime live refresh 消费。 |
| `codex-rs/app-server/src/windows_control.rs` | Windows tray、provider copy/refresh、错误反馈。 |
| `scripts/windows_app_server_refresh_tray.py` | tray 退出、provider 下拉、批量 refresh 反馈。 |
| `codex-rs/tui/src/resume_picker.rs` | 跨 provider 历史发现与 resume 过滤边界。 |
| `codex-rs/tui/src/lib.rs` | 默认日志过滤、resume/history/provider 入口、app-server session 模式。 |

## Checklist

1. 读取并冻结起始状态：本文、`local1` checklist、`Cargo.toml`、`Cargo.lock`、`tmp/agent-snapshots`、`git status`、`git remote -v`。
2. 建立四类真值矩阵：当前工作树、`local1`、官方 `0.119.0`、官方 `0.120.0`。
3. 单独审计 `Cargo.toml` / `Cargo.lock`，明确区分“版本元数据变化”和“真实源码升级完成”。
4. 明确 `rust-v0.119.0` 不能跳过，官方增量必须分两段核对。
5. 逐项核对 `F1-F15` 与 `A1/A2`，不得以一句“保留 local1 功能”替代。
6. 优先复核版本显示链，确认 `CODEX_CLI_DISPLAY_VERSION = concat!(env!("CARGO_PKG_VERSION"), "-local1")` 的消费链未丢失。
7. 优先复核 `/responses` retry / reconnect / telemetry 链，确认 `401` 仍走普通 retry 且中间态不入历史。
8. 优先复核 provider runtime refresh / refreshAllLoaded / Windows tray 链路。
9. 优先复核跨 provider history / resume / `thread/list.model_provider` 链路。
10. 优先复核 `force_gpt54_priority_fallback`、config/schema/docs 口径。
11. 优先复核默认日志降噪链，包含 TUI 与 Windows app / app-server。
12. 后续源码修改必须后置到审计之后，不允许机械官方覆盖。
13. 若改动触及用户可见版本、配置、日志或 tray 口径，必须同步检查并按需更新 `docs/`、`README`、安装文档。
14. 默认不执行 `fmt`、`lint`、`build`、`test`、schema 生成、Bazel 更新等命令；若未来获得授权，也必须在执行记录中如实写明。
15. reviewer subagent 必须只做静态复核，只允许追加到同一 TASK 文件，不允许新建独立审核文件，不允许重写正文，不允许递归 subagent。
16. 主 agent 必须回读 reviewer 输出，逐条给出 `采纳/不采纳`、原因、改写位置、残余风险。
17. 所有临时快照、对比材料和 reviewer 辅助输入统一放在 `tmp/agent-snapshots`。
18. 收口前必须再次复核 `git diff --stat`、`git diff --check`、关键 `rg` 搜索、UTF-8 编码和最终 `git status`。

## Acceptance

- 文档明确写死历史功能基线是 `0.118.0 + local1`，同时明确当前 `Cargo.toml` / `Cargo.lock` 版本元数据已是 `0.120.0`，两者不混淆。
- 文档明确写死目标 tag 是 `rust-v0.120.0`，发布时间是 `2026-04-11`。
- 文档明确覆盖 `rust-v0.119.0` 与 `rust-v0.120.0` 两个官方 release，而不是只写最终 tag。
- 文档明确逐项覆盖 `F1-F15` 与 `A1/A2`，没有把 `local1` 保留要求泛化成一句空话。
- 文档明确列出智能合并顺序、热点文件与冲突面。
- 文档明确 reviewer subagent 的闭环要求是“同一文件追加”，不是独立 reviewer 文件。
- 文档明确主 agent 必须基于 reviewer 附录重新审核、重写并回写同一文件。
- 文档明确本轮代码审核阶段只确认了 `Cargo.toml` / `Cargo.lock` 的版本元数据改动，不能据此宣称已完成官方 `0.120.0` 全量源码对齐。
- 文档明确记录 `tmp/agent-snapshots` 只读参考目录、当前 `git status` 现状和残余风险。
- 文档明确本轮没有执行编译、测试、格式化、lint、build。
- 文档明确相关 `docs/` / `README` / 安装文档口径需要在后续实现阶段同步核对。
- 所有新建或改写的文本文件必须为 UTF-8 无 BOM。

## Notes

- 本文是执行 TASK 文档，不是升级完成报告；但用户明确要求把 reviewer 结论和主 agent 审核处理结果也回写到同一文件，因此本文末尾会包含复核与执行记录附录。
- 当前缺少 `upstream` 远端不是阻塞；默认只使用 `tmp/agent-snapshots` 下的官方只读快照与网页 release/compare 作为参考源。
- 当前工作树的 `Cargo.toml` / `Cargo.lock` 变化只能说明版本元数据已前移，不能直接等价于完成源码层升级。
- 当前 `local1` checklist 中 F14/F15 的“当前代码迹象”描述与现有代码存在局部不一致：`codex-rs/app-server/src/lib.rs` 与 `codex-rs/tui/src/lib.rs` 现状都是默认 `RUST_LOG=warn`，而 checklist 仍写着旧的 `TRACE` / `info` 证据。这是文档真值与代码现状的残余冲突，本轮只记录，不擅自改 `local1` 基线文档。
- 若后续只读 patch/快照不足以定位冲突上下文，必须先向用户说明原因和影响，再决定是否请求更高风险的 Git 操作授权。

## 用户/玩家视角直观变化清单

- 本轮 TASK 回写和代码审核本身没有带来最终用户可直接运行体验到的新功能；当前只确认了版本元数据和升级审计口径。
- 若后续按本 TASK 完成升级，用户应继续看到统一的 `0.120.0-local1` 版本身份，而不是裸官方版本。
- 若后续按本 TASK 完成升级，用户应继续保有首次对话固定清单、provider refresh / tray、`gpt-5.4` priority fallback、跨 provider 历史发现、增强 retry 和默认日志降噪等 `local1` 能力。
- 若后续按本 TASK 完成升级，用户还应获得官方 `0.119.0` / `0.120.0` 的 realtime、resume、hook、status line、MCP、remote / app-server 和 Windows sandbox 修复增益。
- 本次修改无用户/玩家可直接感知的直观运行时变化。

## Reviewer Subagent 严格复核结论

### 文档阶段历史复核结论（已处理）

- reviewer subagent 在本文早期文档阶段曾提出 `5` 项问题，主 agent 已全部采纳并回写：
  - compare 统计不能把 API 截断的 `300` 当真实 changed files 总量。
  - 工作树状态必须承认 `.codexflow` 目录本身是未跟踪变更。
  - `-local1` 口径必须冻结成精确的 `<官方版本>-local1`，不能写“或等价”。
  - 官方参考源必须冻结成 release / compare / patch 的只读路线，不能把临时 fetch 与持久 remote 并列为默认方案。
  - 相关 `docs/` / `README` / 安装文档同步必须进入 Checklist / Acceptance。

### 代码审核阶段 reviewer 原始回写审计（2026-04-12）

#### 复核摘要

- reviewer subagent 已按当前主配置对应的 `gpt-5.4` / `xhigh` 配置创建；当前 subagent 入口未暴露独立 fast 开关，因此沿用当前会话与主配置的 fast 模式配置路径。reviewer 已读取 TASK、`local1` checklist、`Cargo.toml` / `Cargo.lock` 和 `tmp/agent-snapshots`。
- reviewer subagent 的回写结果存在执行层问题：它没有遵守“只在同一文件末尾追加 findings”的约束，而是把 TASK 正文整体重写了。
- reviewer subagent 的正文改写中包含若干有效判断，但也包含与用户明确要求冲突的建议，不能直接原样采纳。

#### 详细问题清单与修改建议

1. **[阻塞][执行违规] reviewer subagent 没有按要求在同一文件末尾追加 findings，而是重写了 TASK 正文。**
   - 影响：破坏了原先已经冻结的章节结构、官方范围、`local1` 保留矩阵和历史 reviewer 处理结果。
   - 修改建议：主 agent 必须恢复完整 TASK 结构，并把 reviewer 结论改写为“同一文件追加附录”的形式。

2. **[阻塞][与用户指令冲突] reviewer subagent 提出“reviewer 结论应独立成文，不应直接追加到 TASK 正文”。**
   - 影响：直接违反用户明确要求的“同一文件追加 reviewer 结论”和“不单独生成批判审核文件”。
   - 修改建议：不采纳该建议，继续坚持 reviewer 附录直接写入同一 TASK 文件。

3. **[高][与用户指令冲突] reviewer subagent 提出“执行记录应独立成文，不要追加到本文”。**
   - 影响：与用户要求的 TASK 回写闭环冲突。用户明确要求主 agent 读取 reviewer 输出后在同一文件中审核、修改和复写。
   - 修改建议：不采纳“必须拆出独立执行记录文件”的建议；允许在同一 TASK 文件保留 `主Agent执行记录` 附录。

4. **[中][有效发现] 仅将 `Cargo.toml` / `Cargo.lock` 从 `0.118.0` 提到 `0.120.0` 不能被表述成升级完成。**
   - 影响：这是本轮最核心的真实性约束，必须保留。
   - 修改建议：在 `Context`、`Acceptance`、`主Agent执行记录` 中继续明确“仅版本元数据已前移，不等于全量源码对齐完成”。

5. **[中][有效发现] 当前代码树不能再被简单描述成“纯 0.118.0 老树”。**
   - 影响：静态审计已经确认多项 `0.119.0/0.120.0` 时代能力存在，若继续宣称“纯 0.118.0”会误导后续合并策略。
   - 修改建议：保留“历史功能基线是 `0.118.0 + local1`，但当前工作树已包含部分前移能力”的双重表述。

6. **[中][有效发现] 必须明确记录 `tmp/agent-snapshots` 只读参考目录、最终 `git status` 和未执行项。**
   - 影响：这关系到本轮执行边界的可追踪性。
   - 修改建议：在执行记录与 Acceptance 中明确保留这些条目。

#### 复核结论

- 未发现新的源码级阻塞问题能够推翻“当前 repo tracked 源码改动仅在 `Cargo.toml` / `Cargo.lock`”这一结论。
- 发现的主要问题集中在 reviewer 自身回写方式违反了 TASK 要求，以及需要继续强调“版本元数据提升不等于完整升级”。
- 残余风险与验证缺口：
  - reviewer 没有通过“追加 findings”形式交付，导致主 agent 需要人工抽取其有效判断并重建 TASK。
  - reviewer 同样没有运行任何编译、测试、fmt、lint、build，这与用户限制一致，但也意味着源码一致性仍只停留在静态层面。
  - reviewer 没有额外指出 `docs/install.md` 与 `local1` checklist F15 证据描述的日志口径冲突；该问题由主 agent 后续静态审计补充发现。

## 主Agent审核处理结果

### 文档阶段历史处理结果（已完成）

| 历史 reviewer 问题 | 结论 | 正文改写位置 | 当前残余风险 |
|---|---|---|---|
| compare `300 files` 被误写成真实总量 | 采纳 | `Context`、`输入真值与证据来源` | 后续若再引用 API file list，仍需标注截断风险 |
| 工作树被误写成“没有未提交短状态变更” | 采纳 | `Context`、`Notes` | 后续实现前仍需重跑 `git status` |
| `-local1` 口径被“或等价”弱化 | 采纳 | `local1 保留矩阵`、`Public Interfaces 冻结` | 若官方重构显示源，仍需点名唯一替代源 |
| upstream 参考源路线未写死 | 采纳 | `输入真值与证据来源`、`智能合并策略` | 若只读 patch 不足，后续仍需用户授权更高风险 Git 操作 |
| 缺少 docs / README / install 同步验收 | 采纳 | `Checklist`、`Acceptance` | 后续真正实现时仍要按需更新文档 |

### 代码审核阶段处理结果（2026-04-12）

| reviewer 问题 | 主 agent 结论 | 原因 | 对应正文改写位置 | 是否仍有残余风险 |
|---|---|---|---|---|
| reviewer 没有按要求末尾追加，而是重写全文 | 采纳 | 复核属实，且与用户指令冲突 | 全文重写并恢复完整结构；本节和 reviewer 附录保留同一文件追加闭环 | 有。若未来再次使用 subagent，仍需更严格限制其写入方式 |
| reviewer 建议把 reviewer 结论独立成文 | 不采纳 | 与用户明确要求“同一文件追加 reviewer 结论”冲突 | `Reviewer Subagent 严格复核结论`、`Notes` | 无新增残余风险，已按用户口径修正 |
| reviewer 建议把执行记录独立成文 | 不采纳 | 用户要求 TASK 回写闭环，且已明确让主 agent 读取 reviewer 结果后在同一文件复写 | `Notes`、`主Agent执行记录` | 无新增残余风险 |
| 仅版本号 bump 不等于完整升级 | 采纳 | 这是当前最关键的真实性边界 | `Context`、`Acceptance`、`主Agent执行记录` | 有。后续仍需更大范围源码对齐 |
| 不能把当前树表述成纯 0.118.0 | 采纳 | 静态检索已确认多项 `0.119.0/0.120.0` 时代能力存在 | `Context`、`输入真值与证据来源`、`主Agent执行记录` | 有。部分前移能力的完整程度仍未编译验证 |
| 必须明确记录 `tmp/agent-snapshots`、最终 `git status`、未执行项 | 采纳 | 这是本轮执行可追溯性的必要部分 | `Context`、`输入真值与证据来源`、`主Agent执行记录` | 有。`tmp/` 与 `.codexflow/` 仍是未跟踪目录，需要后续谨慎管理 |

### 主 agent 最终决定

- 采纳 reviewer 的有效判断，但不采纳与用户明确要求冲突的“独立 reviewer 文件”“独立执行记录文件”建议。
- 本轮不追加新的 repo tracked 源码修改。原因：
  - 静态 diff 已确认当前 repo tracked 源码改动仅是 `Cargo.toml` / `Cargo.lock` 的版本元数据前移。
  - `CODEX_CLI_DISPLAY_VERSION` 当前通过 `env!("CARGO_PKG_VERSION")` 自动派生，版本元数据提升后用户可见版本链会自然变成 `0.120.0-local1`，没有发现额外硬编码 `0.118.0-local1` 需要同步修正。
  - 在用户禁止编译、测试、fmt、lint、build 的前提下，不适合继续做更大范围“猜测性”源码覆盖。
- 本轮把新增发现的文档真值冲突记录为残余风险，而不擅自改动 `local1` checklist：
  - `docs/install.md` 与 `codex-rs/tui/src/lib.rs` 当前都显示 TUI 默认 `RUST_LOG=warn`。
  - `docs/local1-custom-feature-checklist-2026-03-28.md` F15 的“当前代码迹象”仍写着旧的 `info` 基线，F14 也仍写着旧的 `TRACE` 证据。
  - 这说明 `local1` checklist 的证据描述局部滞后于现有代码，但用户并未授权本轮修改该长期基线文档。

## 主Agent执行记录（2026-04-12，代码审核阶段）

### 执行摘要

- 已读取并复核以下输入：
  - 本 TASK 文件
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`
  - `I:\vscodeProject\codex\codex-rs\Cargo.toml`
  - `I:\vscodeProject\codex\codex-rs\Cargo.lock`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\upstream-rust-v0.120.0-2026-04-12\`
  - `git status --short --branch`
  - `git remote -v`
- 已按用户限制执行静态审核；本轮未编译、未测试、未格式化、未 lint、未 build。
- 已使用当前 subagent 工具按 `C:\Users\Administrator\.codex\config.toml` 对应的 `gpt-5.4` / `xhigh` 配置创建 reviewer subagent；该入口未暴露独立 fast 开关，因此沿用当前会话与主配置的 fast 模式配置路径。

### 静态审核结论

- 当前 repo tracked 源码改动仅确认在 `codex-rs/Cargo.toml` 与 `codex-rs/Cargo.lock`。
- `git diff -- codex-rs/Cargo.toml codex-rs/Cargo.lock` 证明实际变更只有：
  - `codex-rs/Cargo.toml` workspace `version` 从 `0.118.0` 调整到 `0.120.0`
  - `codex-rs/Cargo.lock` 内部 workspace crate `version = "0.118.0"` 同步替换为 `version = "0.120.0"`，共 `86` 处
- 当前代码树不是“纯 `0.118.0` 老树”，静态检索已确认多项 `0.119.0/0.120.0` 时代能力存在，例如：
  - `CODEX_CLI_DISPLAY_VERSION` 仍由 [main.rs](/I:/vscodeProject/codex/codex-rs/cli/src/main.rs) 与 [version.rs](/I:/vscodeProject/codex/codex-rs/tui/src/version.rs) 持有，且两者都使用 `env!("CARGO_PKG_VERSION")` 拼接 `-local1`
  - `new_local1_first_turn_checklist` 与单次插入逻辑仍位于 [chatwidget.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget.rs) 与 [history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)
  - `force_gpt54_priority_fallback` 仍位于 [mod.rs](/I:/vscodeProject/codex/codex-rs/core/src/config/mod.rs)、[client.rs](/I:/vscodeProject/codex/codex-rs/core/src/client.rs) 和 [config.schema.json](/I:/vscodeProject/codex/codex-rs/core/config.schema.json)
  - `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 仍位于 app-server protocol 与实现链
  - `pending_session_start_source` 仍位于 [session.rs](/I:/vscodeProject/codex/codex-rs/core/src/state/session.rs)
  - `output_schema` 相关类型仍存在于 [v2.rs](/I:/vscodeProject/codex/codex-rs/app-server-protocol/src/protocol/v2.rs)
- 结合上述事实，本轮最确定、最小且风险最低的 repo tracked 改动就是先完成版本元数据提升；这能让 `0.120.0-local1` 的显示链在后续编译时自动落到正确版本基线，但不代表其他 `0.120.0` 官方变更已全部合并。

### 本轮已确认的源码改动

- [Cargo.toml](/I:/vscodeProject/codex/codex-rs/Cargo.toml)：workspace `version` 已是 `0.120.0`。
- [Cargo.lock](/I:/vscodeProject/codex/codex-rs/Cargo.lock)：内部 workspace crate 版本条目已同步为 `0.120.0`，替换计数为 `86`。

### 本轮未执行项

- 未执行编译、测试、fmt、lint、build、schema 生成、snapshot 接受、Bazel 更新。
- 未配置 `upstream` remote，未 fetch 官方 tag，未 checkout tag，未创建或切换分支，未创建或切换 worktree。
- 未机械覆盖官方 `rust-v0.120.0` 热点源码文件，因为当前工作树已经包含部分前移能力，且用户禁止通过编译/测试做后验校验。
- 未改动 `docs/local1-custom-feature-checklist-2026-03-28.md`、`docs/install.md` 或其他 docs；相关文档差异目前只记录为残余风险。

### 静态验证结果

- `git diff --stat -- codex-rs/Cargo.toml codex-rs/Cargo.lock`：仅涉及上述两个文件。
- `git diff --check -- codex-rs/Cargo.toml codex-rs/Cargo.lock`：未发现明显空白错误。
- 关键静态检索已确认：
  - `CODEX_CLI_DISPLAY_VERSION` 仍存在且继续由 `env!("CARGO_PKG_VERSION")` 派生
  - `new_local1_first_turn_checklist` 仍存在
  - `force_gpt54_priority_fallback` 仍存在
  - `thread/providerRuntime/refresh` 与 `refreshAllLoaded` 仍存在
  - `thread/list` 相关 `model_provider` 字段仍存在
- 当前 `git status --short --branch` 为：
  - `## main...origin/main`
  - `M codex-rs/Cargo.toml`
  - `M codex-rs/Cargo.lock`
  - `?? .codexflow/临时/升级到upstream_rust-v0.120.0并保留local1_2026-04-12/`
  - `?? tmp/`

### 残余风险

- 当前工作区版本元数据已经提升到 `0.120.0`，但整个工作树远未证明已完成官方 `rust-v0.120.0` 的全量源码对齐。
- 官方 `rust-v0.120.0` `Cargo.toml` 比当前本地 workspace 多出 `cloud-tasks-mock-client`、`collaboration-mode-templates`、`codex-mcp`、`model-provider-info`、`models-manager`、`realtime-webrtc`、`response-debug-context` 等 member；本轮没有盲目补齐这些结构，因为用户禁止编译验证，且当前本地树已经发生明显定制化分叉。
- `docs/install.md` 与 `codex-rs/tui/src/lib.rs` 当前都显示 TUI 默认 `RUST_LOG=warn`，而 `local1` checklist F15 的“当前代码迹象”仍写旧的 `info` 基线；`app-server` F14 也仍保留旧 `TRACE` 证据描述。这说明长期基线文档的局部证据已经滞后于当前代码。
- reviewer subagent 本次没有按要求“末尾追加 findings”，而是重写了 TASK 正文；虽然主 agent 已恢复结构并抽取其有效判断，但后续如再使用 subagent，仍需要更严格约束写入方式。

## 主Agent执行记录（2026-04-13，智能合并阶段）

### 阶段定位

- 本节对应“真正进入 `rust-v0.120.0 + local1` 智能合并阶段”的第一轮落盘执行，不再停留在只审计 `Cargo.toml` / `Cargo.lock` 的元数据前移。
- 本轮选择的官方吸收点是 `rust-v0.120.0` 明确缺失、与 `local1` 不冲突、且可以跨链路静态补齐的 `/clear -> SessionStartSource::Clear` 全链路能力。
- 本节执行记录覆盖并补充前文 `2026-04-12` 代码审核阶段的“仅版本元数据前移”结论；两者都保留，但后者不再代表当前最新执行状态。
- 若本文 `2026-04-12` 历史记录与本节存在时态冲突，当前状态一律以本节 `2026-04-13` 智能合并阶段记录为准。

### 本轮已吸收的官方行为

- 已在 [session_start.rs](/I:/vscodeProject/codex/codex-rs/hooks/src/events/session_start.rs) 把 hook 侧 `SessionStartSource` 扩展为 `Startup | Resume | Clear`，并让 `as_str()` 返回 `"clear"`。
- 已在 [protocol.rs](/I:/vscodeProject/codex/codex-rs/protocol/src/protocol.rs) 为 `InitialHistory` 新增 `Cleared`，并把 `forked_from_id`、`session_cwd`、`get_rollout_items`、`get_event_msgs`、`get_base_instructions`、`get_dynamic_tools` 统一扩成 `New | Cleared` 同口径。
- 已在 [thread_manager.rs](/I:/vscodeProject/codex/codex-rs/core/src/thread_manager.rs) 保留 `ForkSnapshot::Interrupted` 下的 `Cleared` 身份，并让 `append_interrupted_boundary(...)` 把 `Cleared` 与 `New` 视为同类空历史入口。
- 已在 [codex.rs](/I:/vscodeProject/codex/codex-rs/core/src/codex.rs) 把 `Cleared` 当成“无初始历史但 session source 不同于 startup”的新鲜线程来源：
  - `persisted_tools` 查找不再把 `Cleared` 误当成 resume/fork。
  - rollout 初始化、state builder、首次历史写入都按 `New` 口径处理。
  - hook `pending_session_start_source` 在 `Cleared` 时改为 `SessionStartSource::Clear`。
- 已在 [v2.rs](/I:/vscodeProject/codex/codex-rs/app-server-protocol/src/protocol/v2.rs) 新增 app-server v2 `ThreadStartSource`，并在 `ThreadStartParams` 中加入可选字段 `session_start_source`。
- 已在 [codex_message_processor.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) 完成 app-server 映射：
  - `thread/start` 解构请求参数时接收 `session_start_source`
  - `Startup -> InitialHistory::New`
  - `Clear -> InitialHistory::Cleared`
  - resume response 错误分支与 rollout 读取分支都补齐 `Cleared`
- 已在 [app_server_session.rs](/I:/vscodeProject/codex/codex-rs/tui/src/app_server_session.rs) 新增 `start_thread_with_session_start_source(...)`，并让普通 `start_thread()` 退化成传 `None` 的包装层。
- 已在 [app.rs](/I:/vscodeProject/codex/codex-rs/tui/src/app.rs) 让：
  - `AppEvent::NewSession` 继续走 `None`
  - `AppEvent::ClearUi` 改为传 `Some(ThreadStartSource::Clear)`
- 已同步 app-server protocol 手写/生成接口产物：
  - [ThreadStartParams.ts](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartParams.ts)
  - [ThreadStartSource.ts](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartSource.ts)
  - [index.ts](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/typescript/v2/index.ts)
  - [ThreadStartParams.json](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/json/v2/ThreadStartParams.json)
  - [ClientRequest.json](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/json/ClientRequest.json)
  - [codex_app_server_protocol.v2.schemas.json](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json)
  - [codex_app_server_protocol.schemas.json](/I:/vscodeProject/codex/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json)
- 已同步 [README.md](/I:/vscodeProject/codex/codex-rs/app-server/README.md) 的 `thread/start` 文字说明和示例 JSON，明确 `sessionStartSource: "clear"` 的用途。

### 本轮保留的 local1 真值

- 未改动 `force_gpt54_priority_fallback` 的顶层 `config.toml` 口径；相关实现仍在 [mod.rs](/I:/vscodeProject/codex/codex-rs/core/src/config/mod.rs)、[client.rs](/I:/vscodeProject/codex/codex-rs/core/src/client.rs) 与 [config.schema.json](/I:/vscodeProject/codex/codex-rs/core/config.schema.json)。
- 未改动 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 的接口与热刷新范围；`thread/list` 的 `model_provider` 口径仍保留。
- 未改动首次对话固定清单的“新对话第一条用户消息提交后插入一次”逻辑；相关真值仍在 [chatwidget.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget.rs) 与 [history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)。
- 未改动 `/responses` 主链的 `401` 普通 retry、retry 中间态不入历史、`10s` cap、日志降噪等 `local1` 口径；本轮没有触碰该链路实现。
- `0.120.0-local1` 展示链继续保留；本轮没有改写 CLI/TUI 版本展示实现，只保留 `Cargo.toml` / `Cargo.lock` 版本元数据前移后的自然派生结果。

### 本轮文档真值回补

- 已把 [local1-custom-feature-checklist-2026-03-28.md](/I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 中 F14/F15 的“当前代码迹象”从旧的 `TRACE` / `info` 证据描述修正为当前真实默认 `warn` 口径。
- 这次修正以 [lib.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/lib.rs)、[lib.rs](/I:/vscodeProject/codex/codex-rs/tui/src/lib.rs) 和 [install.md](/I:/vscodeProject/codex/docs/install.md) 为静态证据来源。

### 冲突裁决

- 本轮没有发现 `SessionStartSource::Clear` 与 `local1` checklist F1-F15、`2026-04-10` 归档补充或已冻结公共接口之间的直接冲突。
- 因此本轮裁决是“直接吸收官方 clear-session 行为”，不需要对 `local1` 做回退或折中改写。
- F14/F15 文档证据漂移已在同轮修正，不再保留“代码是 warn、checklist 仍写 TRACE/info”的冲突状态。

### 静态验证结果

- `git diff --stat` 已确认变更集中在计划内子系统：
  - hooks / protocol / core / app-server / TUI 主链
  - app-server protocol schema / TS / README
  - `local1` checklist 文档
- `git diff --check` 已执行；未发现冲突标记、明显空白错误或行尾结构错误。当前输出仅有 Git 对 LF/CRLF 工作区归一化的提示，不是本轮逻辑错误。
- 关键检索已执行并命中：
  - `SessionStartSource::Clear`
  - `InitialHistory::Cleared`
  - `ThreadStartSource`
  - `session_start_source`
  - `sessionStartSource`
  - `start_thread_with_session_start_source`
- 已额外排查 `ThreadStartParams { ... }` 的全字段字面量，发现并补齐 [skills_list.rs](/I:/vscodeProject/codex/codex-rs/app-server/tests/suite/v2/skills_list.rs) 中遗漏的 `session_start_source: None`，避免新增字段后留下明显编译性缺口。
- 已对本轮修改的全部文本文件执行 UTF-8 读取检查；结果为：
  - 无 BOM
  - 无替代字符 `�`
  - 无乱码写入

### 本轮未执行项

- 未执行编译、测试、fmt、lint、build、schema 生成器、snapshot 接受、Bazel 相关命令；这是用户明确禁止项。
- 未创建或切换分支，未 checkout tag，未创建或切换 worktree，未配置持久 `upstream` remote，未向当前仓库写入 fetch refs。
- 未机械吸收其他 `0.119.0` / `0.120.0` 热点功能；本轮只完成 clear-session 这条最确定且低冲突的跨链路合并。

### 当前工作区状态

- 当前 `git status --short --branch` 为：
  - `## main...origin/main`
  - `M codex-rs/Cargo.toml`
  - `M codex-rs/Cargo.lock`
  - `M` 本轮涉及的 hooks / protocol / core / app-server / TUI / schema / README / checklist 文件
  - `?? codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartSource.ts`
  - `?? .codexflow/临时/升级到upstream_rust-v0.120.0并保留local1_2026-04-12/`
  - `?? tmp/`
- 远端状态未变：当前仓库仍只有 `origin`，没有 `upstream`。

### 残余风险

- 本轮只完成了 `0.120.0` 中 `/clear` 相关的一条确定性主链；realtime background agent progress、hook 状态渲染、tool `outputSchema` 展示补强、Windows sandbox carveouts / symlink 修复、remote TLS 修复等其余官方增量仍未在本轮落盘。
- 由于用户禁止编译和测试，本轮所有结论都停留在静态层面；虽然枚举穷尽、字段贯通和 schema 一致性已做人工核对，但没有运行时证据。
- 本轮同步了 Rust / JSON schema / TypeScript / README / `local1` checklist；未扩展到其他潜在衍生产物或外部 SDK 生成代码，若后续要对外发布 app-server 协议包，仍需再做一轮接口产物完整性检查。

## Reviewer Subagent 严格复核结论（2026-04-13，智能合并阶段）

### 复核摘要

- 已按要求只读复核以下真值与改动范围：
  - 当前 TASK 文件
  - `docs/local1-custom-feature-checklist-2026-03-28.md`
  - `tmp/agent-snapshots/upstream-rust-v0.120.0-2026-04-12/` 下的官方 `rust-v0.120.0` 快照
  - 当前工作树中本轮涉及的 hooks / protocol / core / app-server / TUI / schema / README / checklist 改动
- 已额外执行静态一致性排查：
  - 检索 `SessionStartSource::Clear`、`InitialHistory::Cleared`、`ThreadStartSource`、`session_start_source`、`sessionStartSource`
  - 扫描 `ThreadStartParams { ... }` 字面量，确认没有遗留未补 `session_start_source` 且缺少 `..Default::default()` 的全字段构造
  - 对照 upstream 快照核对 `/clear -> SessionStartSource::Clear` 的核心落点与 schema/TypeScript 产物链
- 复核结论：未发现本轮智能合并新增的阻塞级代码问题；发现 `1` 项低风险文档时态歧义，建议主 agent 在后续审核结论中明确覆盖关系。

### 详细问题清单与修改建议

1. **严重级别：低**
   **问题：** 同一 TASK 文件中，`2026-04-12` 历史执行记录仍保留“未改动 `docs/local1-custom-feature-checklist-2026-03-28.md` / `docs/install.md`，F14/F15 证据仍旧是旧口径”的阶段性表述，而 `2026-04-13` 智能合并阶段记录已明确写明 F14/F15 的 `warn` 口径文档回补已完成。两段内容按时间线并不矛盾，但对快速通读者存在时态歧义。
   **影响：** 读者可能误判当前 `local1` checklist 的真实状态，把已修正的问题继续当成现存问题，降低 TASK 作为单文件执行与复核真值的可读性。
   **修改建议：** 主 agent 在后续 `主Agent审核处理结果` 中明确写明“`2026-04-12` 章节为历史快照，当前状态以 `2026-04-13` 智能合并阶段记录为准”；若后续允许重写正文，再把旧阶段中的现状描述前置为“历史记录”以消除歧义。

2. **严重级别：信息**
   **问题：** 未发现本轮 `/clear -> SessionStartSource::Clear` 贯通、`ThreadStartSource` / `session_start_source` schema-JSON-TypeScript 链、`README` clear 参数说明，以及 `local1` checklist F14/F15 `warn` 口径回补方面的新增阻塞问题。
   **影响：** 当前静态复核没有发现必须回退、重写或立即修补的新增代码/文档缺口。
   **修改建议：** 本轮无需因为 reviewer findings 再追加源码修复；保留“未编译、未测试、未 fmt、未 lint、未 build”的验证缺口说明即可。

### 复核结论

- 未发现新增阻塞问题。
- 当前这轮智能合并在 reviewer 复核范围内，已满足以下静态一致性要求：
  - `/clear` 可以沿 TUI -> app-server v2 -> core -> hooks 传递 `Clear`
  - `InitialHistory::Cleared` 已贯通到协议与主链消费点
  - `ThreadStartSource` 与 `sessionStartSource` 已同步到 Rust 定义、JSON schema、TypeScript 产物和 `README`
  - `local1` checklist 的 F14/F15 “当前代码迹象”已回补到 `warn` 真值
- 残余风险仍然存在，但都属于本轮用户明确禁止项带来的验证缺口，而不是新增静态阻塞问题：
  - 未编译
  - 未测试
  - 未 fmt
  - 未 lint
  - 未 build
  - 其余 `0.119.0` / `0.120.0` 官方能力尚未在本轮全部吸收，本次复核只覆盖当前已落盘的 clear-session 主链与相关文档/协议同步范围

## 主Agent审核处理结果（2026-04-13，智能合并阶段）

| reviewer 问题 | 主 agent 结论 | 原因 | 对应正文改写位置 | 是否仍有残余风险 |
|---|---|---|---|---|
| `2026-04-12` 历史记录与 `2026-04-13` 智能合并记录存在时态歧义 | 采纳 | 复核属实；两段内容是时间线先后关系，不是事实冲突，但确实会影响单文件快速阅读 | `主Agent执行记录（2026-04-13，智能合并阶段）` 的 `阶段定位` 新增“以 2026-04-13 为准”说明 | 无新增残余风险 |
| 未发现新增阻塞问题；本轮无需再追加源码修复，只需保留静态验证缺口说明 | 采纳 | 与主 agent 的静态审查结果一致；当前没有新的事实错误、范围遗漏或接口冲突需要立即修补 | 本节、`残余风险` | 有。编译/测试/fmt/lint/build 仍未执行，且其他 `0.119.0` / `0.120.0` 能力尚未合并 |

### 主 agent 最终决定

- 采纳 reviewer 的全部正式 findings。
- 已执行的修正仅限文档层澄清：明确 `2026-04-12` 为历史快照，当前状态以 `2026-04-13` 智能合并阶段记录为准。
- 不再追加新的源码修复。原因是 reviewer 未发现新增阻塞问题，本轮 clear-session 主链、schema/README/checklist 回补在静态层面已经闭环。
- 继续保留当前残余风险说明，不用“无问题”掩盖用户明确禁止编译/测试/fmt/lint/build 所带来的验证缺口。
