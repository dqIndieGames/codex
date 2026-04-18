# local1_release构建与全量功能验真_TASK_2026-04-13

## 输出位置说明

- 项目根目录按当前任务仓库解析为 `I:\vscodeProject\codex`。
- 因用户显式要求将本轮正式产物落到 `.codexflow/临时/` 路径，本轮唯一正式文件固定写入：
  - `I:\vscodeProject\codex\.codexflow\临时\local1_release构建与全量功能验真_2026-04-13`
- 本轮主文件固定为：
  - [local1_release构建与全量功能验真_TASK_2026-04-13.md](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/local1_release构建与全量功能验真_TASK_2026-04-13.md)
- 本轮不创建独立 review 文件；reviewer subagent findings 与主 agent 的处理结果都必须追加在同一主文件末尾。
- 本文前半部分保留文档阶段冻结计划，后半部分回写同日实际执行结果；正式产物仍集中落在本目录，不复用旧目录，不镜像写入 `docs/`，也不把阶段性笔记散落到项目根目录。

## Summary

- 本文件上半部分保留 2026-04-13 文档冻结阶段的原始 TASK 规划；下半部分已回写同日实际执行结果、raw evidence 与逐项结论。
- 本文的后续执行目标固定为两段式：
  1. 用最小范围命令完成 `codex.exe` 的 release 编译。
  2. 对 [local1-custom-feature-checklist-2026-03-28.md](/I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 中 `F1-F15` 与归档项 `A1/A2` 做“混合验真”验证，不遗漏任何条目。
- 实际执行阶段已完成 release 构建、release-only CLI smoke、既有定向测试证据归档与逐项静态核对；未新增公共 API、接口或类型变更。
- 用户在执行阶段后半程额外收紧口径为“采用已经编译好的 release 做校验，不要再做 debug 编译”，因此后续未再新增任何会触发 debug 编译的 `cargo test`、`cargo run` 或 `cargo build`。
- 本文冻结的真值来源固定为：
  - [local1-custom-feature-checklist-2026-03-28.md](/I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
  - [justfile](/I:/vscodeProject/codex/justfile)
  - [install.md](/I:/vscodeProject/codex/docs/install.md)
  - 当前仓库源码现状与脏工作区基线
- 功能组边界仍固定为：
  - `G1 = F1-F4 + A1`
  - `G2 = F5-F9 + F12`
  - `G3 = F10`
  - `G4 = F11`
  - `G5 = F13 + A2`
  - `G6 = F14-F15`
- 但后续执行阶段不得只输出 `G1-G6` 粗粒度结论，必须逐条对 `F1-F15`、`A1`、`A2` 收口。

## Context

- 当前仓库工作区不是干净基线；后续执行阶段必须以“执行前当前工作树状态”为准，不得擅自回滚无关改动，不得切分支，不得创建 worktree。
- 2026-04-13 03:08:33 +08:00 读取到的 `git status --short` 快照如下，后续执行阶段必须把它视为既有基线，而不是待清理项：
  - `M codex-rs/Cargo.lock`
  - `M codex-rs/Cargo.toml`
  - `M codex-rs/app-server-protocol/schema/json/ClientRequest.json`
  - `M codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json`
  - `M codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json`
  - `M codex-rs/app-server-protocol/schema/json/v2/ThreadStartParams.json`
  - `M codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartParams.ts`
  - `M codex-rs/app-server-protocol/schema/typescript/v2/index.ts`
  - `M codex-rs/app-server-protocol/src/protocol/v2.rs`
  - `M codex-rs/app-server/README.md`
  - `M codex-rs/app-server/src/codex_message_processor.rs`
  - `M codex-rs/app-server/tests/suite/v2/skills_list.rs`
  - `M codex-rs/core/src/codex.rs`
  - `M codex-rs/core/src/thread_manager.rs`
  - `M codex-rs/hooks/src/events/session_start.rs`
  - `M codex-rs/protocol/src/protocol.rs`
  - `M codex-rs/tui/src/app.rs`
  - `M codex-rs/tui/src/app_server_session.rs`
  - `M docs/local1-custom-feature-checklist-2026-03-28.md`
  - `?? .codexflow/临时/升级到upstream_rust-v0.120.0并保留local1_2026-04-12/`
  - `?? codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartSource.ts`
  - `?? tmp/`
- 当前仓库内已经存在与 local1 定制链相关的历史任务文档；本轮不复用旧目录，不覆盖旧文件，只把它们作为结构和范围参考。
- 当前任务的关键源码锚点必须按条目补齐，不得仅凭组级摘要笼统验收：
  - `A1` 必须同时覆盖 TUI 路径与 app-server 路径，而不是只覆盖 TUI。
  - `F9` 必须显式覆盖 [session.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/endpoint/session.rs) 与 [api_bridge.rs](/I:/vscodeProject/codex/codex-rs/core/src/api_bridge.rs)。
  - `F10` 的真实入口是 [README.md](/I:/vscodeProject/codex/codex-rs/app-server/README.md) 中的 `thread/providerRuntime/refresh`、`thread/providerRuntime/refreshAllLoaded`，以及 [windows_control.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/windows_control.rs) 中的 `refresh_all_loaded_threads`。
  - `F11` 必须显式覆盖 [lib.rs](/I:/vscodeProject/codex/codex-rs/tui/src/lib.rs)、[resume_picker.rs](/I:/vscodeProject/codex/codex-rs/tui/src/resume_picker.rs)、[codex_message_processor.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) 与 `thread/list` 返回项里的 `model_provider` 语义。
- 当前仓库中可直接支持本任务范围判断的真实入口包括：
  - [main.rs](/I:/vscodeProject/codex/codex-rs/cli/src/main.rs)
  - [version.rs](/I:/vscodeProject/codex/codex-rs/tui/src/version.rs)
  - [history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)
  - [chatwidget.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget.rs)
  - [provider.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/provider.rs)
  - [telemetry.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/telemetry.rs)
  - [thread_provider_runtime_refresh.rs](/I:/vscodeProject/codex/codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs)
  - [lib.rs](/I:/vscodeProject/codex/codex-rs/app-server-test-client/src/lib.rs)
  - [windows_app_server_refresh_tray.py](/I:/vscodeProject/codex/scripts/windows_app_server_refresh_tray.py)

## Goal

- 输出一份可直接执行的单文档任务书，明确后续如何：
  - 只用最小范围 release 命令构建 `codex.exe`
  - 逐项验证 `F1-F15` 与 `A1/A2`
  - 记录 `release_build.log`、`release_runtime_smoke.log`、`targeted_validation.log` 三类原始证据
  - 在保持当前脏工作区不被误清理的前提下完成验真
- 把以下边界写死，避免后续执行者自行改口径：
  - `codex.exe` 主构建只能是最小 `release`，不能用 debug 构建替代
  - 允许定向 `cargo test -p <crate>` 与少量 `cargo run -p <tool>` 作为补充验证，但它们不能替代最小 release 构建
  - `F1-F15`、`A1`、`A2` 全部必须进入验证矩阵与验收口径，不能选做
  - `codex-app-server-test-client` 只能使用其现有支持的 CLI 子命令做补充联调，不得把不存在的 `providerRuntime/refresh*` 子命令写进执行步骤

## Execution Constraints

- 当前文档落盘阶段禁止执行以下动作：
  - 任何代码修改
  - 任何编译
  - 任何测试
  - 任何 `cargo run`
  - 任何格式化、lint、codegen、snapshot 更新
- 上述“禁止执行”只对应 2026-04-13 的文档冻结阶段；实际执行结果、执行态 workaround 和最终条目状态以本文下半部分 `执行回写（2026-04-13）` 为准。
- 后续执行阶段的仓库操作约束固定为：
  - 不得创建分支
  - 不得切换分支
  - 不得创建或进入新 worktree
  - 不得回滚当前工作区里的无关改动
- 后续执行阶段允许的验证方式固定为：
  - 最小 release 构建
  - 基于 release 二进制的 CLI/runtime smoke
  - 定向 `cargo test -p <crate>`
  - 少量 `cargo run -p <tool>` 联调
  - 静态源码与文档核对
- 后续执行阶段不允许把以下动作冒充为本任务完成条件：
  - `cargo build` workspace 全量构建
  - `cargo build -p codex-cli` debug 构建
  - `cargo run --bin codex` 直接替代 release 构建
  - 只做静态核对却声称已完成运行态验证
- 允许定向测试是显式补充约束，而不是主构建替代品：
  - 允许 `cargo test -p <crate>` 与少量 `cargo run -p <tool>` 作为验证手段
  - 但不得把这些命令替代 `codex.exe` 的最小 release 构建
  - 也不得把主构建改成 debug build

## Build Recipe

- 后续执行阶段唯一允许作为 `codex.exe` 主构建验收门的命令固定为：

```powershell
Set-Location "I:\vscodeProject\codex\codex-rs"
$env:CARGO_TARGET_DIR = "I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist"
cargo build -p codex-cli --bin codex --release
```

- 上述 recipe 是冻结的主命令体，不因执行回写而改写。
- 实际执行回写显示：Build Run 3 为了规避 Windows 跨盘 `v8` symlink 权限问题，额外设置了 `CARGO_HOME=I:\cargo-home-local1`；该值属于执行态 same-drive workaround，不改变本文冻结的 release 构建命令体。

- 后续执行阶段必须记录以下 release 产物与证据：
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist\release\codex.exe`
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist\release\codex.pdb`
  - `I:\vscodeProject\codex\.codexflow\临时\local1_release构建与全量功能验真_2026-04-13\release_build.log`
  - `I:\vscodeProject\codex\.codexflow\临时\local1_release构建与全量功能验真_2026-04-13\release_runtime_smoke.log`
  - `I:\vscodeProject\codex\.codexflow\临时\local1_release构建与全量功能验真_2026-04-13\targeted_validation.log`

### 三类 raw evidence 最小 schema

| 日志 | 必填字段 | 额外要求 |
|---|---|---|
| `release_build.log` | `started_at`、`finished_at`、`cwd`、`command`、`arguments`、`key_env`、`exit_code`、`stdout_excerpt`、`stderr_excerpt`、`related_items`、`evidence_paths` | 还必须记录 `CARGO_TARGET_DIR`、`codex.exe` 是否存在、`codex.pdb` 是否存在、二者文件大小、最后写入时间 |
| `release_runtime_smoke.log` | `started_at`、`finished_at`、`cwd`、`command`、`arguments`、`key_env`、`exit_code`、`stdout_excerpt`、`stderr_excerpt`、`related_items`、`evidence_paths` | 至少覆盖 `<release codex.exe> --version` 与 `<release codex.exe> --help`；每条记录必须标注对应的 `F*` / `A*` |
| `targeted_validation.log` | `started_at`、`finished_at`、`cwd`、`command`、`arguments`、`key_env`、`exit_code`、`stdout_excerpt`、`stderr_excerpt`、`related_items`、`evidence_paths` | 还必须记录 `validation_case`、`test_case_names` 或 `source_files_checked`、`status`、阻塞原因或静态核对结论 |

- 上表中的 `related_items` 必须直接写明关联条目，例如 `F10`、`A1`、`F14-F15`，不得只写 `G3` 或 `G6`。
- 上表中的 `evidence_paths` 必须记录实际证据位置，例如日志文件、截图索引、附加片段或目标二进制路径。
- 若某条记录属于 mixed evidence，`exit_code` 也不得省略；必须显式写成 `mixed; <子证据退出码或缺失原因>`，不能靠读者自行推断。
- 若需临时快照或中间审计文件，统一写到：
  - `I:\vscodeProject\codex\tmp\agent-snapshots`
  - 使用后清理，不放在项目根目录

## Checklist 验证矩阵

- 功能组边界保持冻结计划不变，但后续执行必须在组内继续拆到逐项条目。
- 下表中的 `主证据` 用于决定该条目是否通过；`补充证据` 只用于增强证据密度，不能替代 `主证据`。

| 条目 | 所属功能组 | 主证据 | 补充证据 | 必查文件/入口 | 通过口径 |
|---|---|---|---|---|---|
| `F1` | `G1` | `<release codex.exe> --version`、`<release codex.exe> --help` | 静态核对 [main.rs](/I:/vscodeProject/codex/codex-rs/cli/src/main.rs)、[version.rs](/I:/vscodeProject/codex/codex-rs/tui/src/version.rs) | `release_runtime_smoke.log` | CLI 与 TUI 共用的展示版本必须仍带 `-local1` 后缀 |
| `F2` | `G1` | `cargo test -p codex-tui` | 静态核对 [card.rs](/I:/vscodeProject/codex/codex-rs/tui/src/status/card.rs)、[app.rs](/I:/vscodeProject/codex/codex-rs/tui/src/app.rs)、[status_surfaces.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget/status_surfaces.rs) | `targeted_validation.log` | 卡片、顶部状态区、相关面板的版本展示口径一致，不回退到官方裸版本 |
| `F3` | `G1` | `cargo test -p codex-tui` | 静态核对 [history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)、[update_prompt.rs](/I:/vscodeProject/codex/codex-rs/tui/src/update_prompt.rs)、[tests.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget/tests.rs) | `targeted_validation.log` | 历史单元与升级提示仍展示 `local1` 版本名 |
| `F4` | `G1` | `cargo test -p codex-tui` | 静态核对 [tests.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget/tests.rs)、[history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)、状态快照目录 | `targeted_validation.log` | `local1` 版本展示相关断言或快照仍存在，不被上游覆盖掉 |
| `A1` | `G1` | `cargo test -p codex-tui` 覆盖首次插入一次的 TUI 路径；同时用 release 二进制执行 `cargo run -p codex-app-server-test-client -- --codex-bin <release codex.exe> send-message-v2 "local1 checklist probe"`，并在能拿到线程 ID 时继续执行 `resume-message-v2 <thread_id> "local1 checklist resume probe"` 作为 app-server 路径证据 | 静态核对 [chatwidget.rs](/I:/vscodeProject/codex/codex-rs/tui/src/chatwidget.rs)、[history_cell.rs](/I:/vscodeProject/codex/codex-rs/tui/src/history_cell.rs)、[lib.rs](/I:/vscodeProject/codex/codex-rs/app-server-test-client/src/lib.rs) | `targeted_validation.log` | 必须同时覆盖 TUI 与 app-server 路径；首次对话固定清单只插入一次；resume/continue 后不得重复插入；固定清单里必须仍含 refresh/retry + Windows tray 联动概述项 |
| `F5` | `G2` | `cargo test -p codex-api` | 静态核对 [provider.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/provider.rs) | `targeted_validation.log` | `/responses` 主链全 HTTP 状态统一可重试，非 `/responses` 端点仍保留旧 whitelist |
| `F6` | `G2` | `cargo test -p codex-api`、`cargo test -p codex-core` | 静态核对 [telemetry.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/telemetry.rs)、[client.rs](/I:/vscodeProject/codex/codex-rs/core/src/client.rs)、[codex.rs](/I:/vscodeProject/codex/codex-rs/core/src/codex.rs) | `targeted_validation.log` | retry/reconnect 中间态仍只走状态区，不写入历史；OTEL/log/trace suppress 与 UI 详情链并存 |
| `F7` | `G2` | `cargo test -p codex-core` | 静态核对重试 delay 上限实现 | `targeted_validation.log` | 单次等待上限仍是 `10s`，不会因扩大重试分类而突破 |
| `F8` | `G2` | `cargo test -p codex-api`、`cargo test -p codex-core` | 静态核对 request/stream budget 相关实现 | `targeted_validation.log` | 统一错误分类不应误写成 retry budget 语义变化 |
| `F9` | `G2` | `cargo test -p codex-api`、`cargo test -p codex-core` | 静态核对 [session.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/endpoint/session.rs)、[api_bridge.rs](/I:/vscodeProject/codex/codex-rs/core/src/api_bridge.rs)、[telemetry.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/telemetry.rs)、[provider.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/provider.rs) | `targeted_validation.log` | route 真值必须显式透传，retry/reconnect suppress 入口必须保持统一，不回退到散落逻辑 |
| `F12` | `G2` | `cargo test -p codex-api`、`cargo test -p codex-core` | 静态核对 [provider.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/provider.rs)、[client.rs](/I:/vscodeProject/codex/codex-rs/core/src/client.rs)、[codex.rs](/I:/vscodeProject/codex/codex-rs/core/src/codex.rs) | `targeted_validation.log` | `/responses` stream / websocket 中的 `401` 仍直接走普通 retry 链，不回退到 unauthorized recovery 优先分支 |
| `F10` | `G3` | `cargo test -p codex-app-server`，且证据必须点名 [thread_provider_runtime_refresh.rs](/I:/vscodeProject/codex/codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs) 中的 7 个关键用例：`thread_provider_runtime_refresh_returns_applied_for_idle_thread`、`thread_provider_runtime_refresh_returns_queued_for_active_thread`、`thread_provider_runtime_refresh_returns_invalid_request_when_provider_is_missing`、`thread_provider_runtime_refresh_returns_invalid_request_for_invalid_user_config`、`thread_provider_runtime_refresh_all_loaded_reports_mixed_statuses`、`thread_provider_runtime_refresh_all_loaded_keeps_failed_threads_empty_for_relative_agent_config_file`、`thread_provider_runtime_refresh_all_loaded_treats_zero_loaded_threads_as_success` | `cargo run -p codex-app-server-test-client -- --codex-bin <release codex.exe> model-list` 仅可作为 release 二进制 app-server 承载/连通性补充证据；静态核对 [README.md](/I:/vscodeProject/codex/codex-rs/app-server/README.md)、[windows_control.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/windows_control.rs)、[windows_app_server_refresh_tray.py](/I:/vscodeProject/codex/scripts/windows_app_server_refresh_tray.py) | `targeted_validation.log` | `refresh` / `refreshAllLoaded` 的主证据必须来自真实 RPC 测试与控制面源码，不得把 test client 的普通子命令冒充为 refresh RPC 入口；Windows tray GUI 可见项允许静态核对脚本与控制面，不要求本轮做 GUI 自动化 |
| `F11` | `G4` | `cargo test -p codex-exec`、`cargo test -p codex-tui`、`cargo test -p codex-app-server` | 静态核对 [lib.rs](/I:/vscodeProject/codex/codex-rs/tui/src/lib.rs)、[resume_picker.rs](/I:/vscodeProject/codex/codex-rs/tui/src/resume_picker.rs)、[codex_message_processor.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) | `targeted_validation.log` | 必须保留 cwd / `--all` / `show_all` 语义边界；历史默认不按 provider 收窄；`thread/list` 返回项里的 `model_provider` 仍保留 |
| `F13` | `G5` | `cargo test -p codex-core` | 静态核对 [client_tests.rs](/I:/vscodeProject/codex/codex-rs/core/src/client_tests.rs)、[mod.rs](/I:/vscodeProject/codex/codex-rs/core/src/config/mod.rs)、[config.schema.json](/I:/vscodeProject/codex/codex-rs/core/config.schema.json) | `targeted_validation.log` | `force_gpt54_priority_fallback` 顶层开关、默认 `priority` 与显式 `false` 时关闭 `Fast` 透传的语义保持不变 |
| `A2` | `G5` | `cargo test -p codex-core` | 静态核对 [client_tests.rs](/I:/vscodeProject/codex/codex-rs/core/src/client_tests.rs)、[mod.rs](/I:/vscodeProject/codex/codex-rs/core/src/config/mod.rs) | `targeted_validation.log` | 该字段只允许出现在顶层 `config.toml`，`[profiles.*]` 下的同名字段不得生效 |
| `F14` | `G6` | 静态核对 [lib.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/lib.rs) 与 [install.md](/I:/vscodeProject/codex/docs/install.md) | 必要时补最小运行态日志烟测 | `targeted_validation.log` | 必须写成“默认降噪”，而不是“移除日志能力” |
| `F15` | `G6` | 静态核对 [lib.rs](/I:/vscodeProject/codex/codex-rs/tui/src/lib.rs) 与 [install.md](/I:/vscodeProject/codex/docs/install.md) | 必要时补最小运行态日志烟测 | `targeted_validation.log` | TUI 默认日志降噪不应被误写为删除文件日志或 sqlite 日志能力 |

## Detailed Execution Checklist

1. 冻结执行前基线。  
   在后续真正执行本任务前，再次记录 `git status --short` 到 `targeted_validation.log`，并明确本轮基线允许存在现有脏改。
2. 初始化三类 raw evidence 文件。  
   先按 `Build Recipe` 中的最小 schema 写入 `release_build.log`、`release_runtime_smoke.log`、`targeted_validation.log` 的头部字段，避免后续遗漏开始时间、命令、环境变量和关联条目。
3. 执行最小 release 构建。  
   只能使用本文 `Build Recipe` 中写死的命令；不得改成 debug build，不得省略 `--release`，不得省略独立 `CARGO_TARGET_DIR`。
4. 记录 release 产物证据。  
   把 `codex.exe`、`codex.pdb` 的存在性、大小、最后写入时间和退出码写入 `release_build.log`。
5. 做 release 二进制最小运行态冒烟。  
   至少记录 `<release codex.exe> --version` 与 `<release codex.exe> --help` 的完整命令、退出码和输出摘要到 `release_runtime_smoke.log`，用于覆盖 `F1`。
6. 执行 `G1` 的逐项验证。  
   `F1-F4` 以 `cargo test -p codex-tui` 与 release CLI 证据组合完成；`A1` 除 TUI 路径外，还必须补一条 app-server 路径：用 release `codex.exe` 驱动 `send-message-v2`，在拿到线程 ID 的情况下继续做 `resume-message-v2`，验证首次清单只插一次且 resume 不重复插入。若运行态受账号、环境或模型条件阻塞，必须把该条目标记为 `Blocked`，不能只保留 TUI 证据后宣称 `A1` 已全部通过。
7. 执行 `G2` 的逐项验证。  
   运行 `cargo test -p codex-api` 与 `cargo test -p codex-core`；`F9` 必须单独在 `targeted_validation.log` 中点名 [session.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/endpoint/session.rs) 与 [api_bridge.rs](/I:/vscodeProject/codex/codex-rs/core/src/api_bridge.rs) 的静态核对结果。
8. 执行 `G3` 的逐项验证。  
   `F10` 的主证据必须来自 `cargo test -p codex-app-server`，并明确引用 `thread_provider_runtime_refresh.rs` 中的 7 个关键用例。若需要 release 二进制补充证据，只允许使用 `cargo run -p codex-app-server-test-client -- --codex-bin <release codex.exe> model-list` 这类现有子命令证明 release app-server 可承载与可连通；不得把它写成 `providerRuntime/refresh*` 的直接入口，也不得用 `...` 省略关键命令。
9. 执行 `G4` 的逐项验证。  
   运行 `cargo test -p codex-exec`、`cargo test -p codex-tui`、`cargo test -p codex-app-server`；在 `targeted_validation.log` 中单列 `F11`，逐条说明 cwd、`--all`、`show_all`、历史默认跨 provider 可见、`thread/list.model_provider` 保留情况。
10. 执行 `G5` 的逐项验证。  
    运行 `cargo test -p codex-core`；分别对 `F13` 与 `A2` 记录证据，不得把 `A2` 吞并到 `F13` 里一笔带过。
11. 执行 `G6` 的逐项验证。  
    先做静态核对；只有静态证据不足时，才补最小日志烟测。不得因为日志默认降噪，就误写成“日志能力被删掉”。
12. 逐项收口。  
    执行结束后必须补一张最终表格，格式固定为：`条目 | 状态 | 主证据文件/位置 | 备注`。表格里必须逐条列出 `F1-F15`、`A1`、`A2`，不得只给 `G1-G6` 组级结论。
13. 汇总 raw evidence。  
    `release_build.log`、`release_runtime_smoke.log`、`targeted_validation.log` 三份日志必须齐全；如有额外截图、快照、片段或临时索引，也必须注明路径和关联条目。

## Acceptance

- 本文已明确写死唯一允许的 `codex.exe` 主构建命令：
  - `cargo build -p codex-cli --bin codex --release`
- 本文已明确写死独立 `CARGO_TARGET_DIR`：
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist`
- 本文已明确写死 release 产物路径：
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist\release\codex.exe`
- 本文已明确写死三类 raw evidence 日志名称与位置：
  - `release_build.log`
  - `release_runtime_smoke.log`
  - `targeted_validation.log`
- 本文已把 `release_build.log`、`release_runtime_smoke.log`、`targeted_validation.log` 的最小字段 schema 写死到文档中，不再只给文件名而不给记录口径；本轮执行回写也已补齐 `arguments`、`evidence_paths`、`stdout_excerpt`、`stderr_excerpt` 与 mixed evidence 的 `exit_code` 说明。
- 本文的验证矩阵已逐项点名 `F1-F15`、`A1`、`A2`，没有遗漏条目，也没有把任何条目标记为可选。
- 本文已把 `A1` 的 TUI 路径与 app-server 路径显式拆开，不再只覆盖 `codex-tui`。
- 本文已把 `F9` 的 [session.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/endpoint/session.rs) 与 [api_bridge.rs](/I:/vscodeProject/codex/codex-rs/core/src/api_bridge.rs) 显式写入矩阵与执行清单。
- 本文已把 `F10` 的真实 refresh 入口、真实测试用例与 test client 的职责边界写清楚：
  - refresh 语义主证据来自 `thread/providerRuntime/refresh*` 相关测试与控制面源码
  - `codex-app-server-test-client` 只作为 release app-server 承载/连通性补充证据
- 本文已明确写出“允许定向测试，但不得以 debug build 替代最小 release 构建”的边界。
- 本文已明确写出当前工作区 dirty baseline，且后续执行阶段不得回滚无关改动、不得切分支、不得建 worktree。
- 本文已要求后续执行阶段用 `条目 | 状态 | 主证据文件/位置 | 备注` 的逐项表格收口，而不是只写组级摘要。
- 本文已明确区分“冻结 recipe”和“执行态 workaround”；`CARGO_HOME=I:\cargo-home-local1` 只属于 Build Run 3 的实际成功环境，不改写冻结的 release recipe 本体。
- 本文顶部与说明区已明确标注“前半部分冻结计划 + 后半部分执行回写”，不再把无条件“本轮只落盘文档”写成当前整体事实。
- 本文末尾保留了 `Subagent严格复核附录` 与 `主Agent审核处理结果` 两节，并按“文档阶段复核 / 执行阶段复核”分轮次回写 findings 与主 agent 处理结果。

## Notes

- 上半部分规划区形成于文档落盘阶段；实际 build / smoke / targeted validation 结果见下文 `执行回写（2026-04-13）` 与同目录 raw evidence。
- 顶部“输出位置说明”和 `Execution Constraints` 里的文档阶段表述只用于保留冻结计划历史；若与执行阶段事实冲突，以 `执行回写（2026-04-13）`、三类 raw evidence 和最终结论表为准。
- 后续执行阶段若 build、运行态联调或测试失败，失败本身属于真实结果，应当记录为 `Blocked` 或 `Failed`，不能为了凑完成度省略。
- 虽然 [install.md](/I:/vscodeProject/codex/docs/install.md) 写了 Windows 11 via WSL2 的官方安装背景，但本任务是针对当前 `I:\vscodeProject\codex` 本地 Windows 工作区的现实验证任务；不得因为文档里的 WSL2 描述而改写成另一套环境方案。
- [lib.rs](/I:/vscodeProject/codex/codex-rs/app-server-test-client/src/lib.rs) 当前支持的 test client 子命令包括 `serve`、`send-message-v2`、`resume-message-v2`、`thread-resume`、`watch`、`model-list`、`thread-list` 等；它并不存在可直接调用 `thread/providerRuntime/refresh` 或 `thread/providerRuntime/refreshAllLoaded` 的 CLI 子命令。
- 因此，`cargo run -p codex-app-server-test-client -- --codex-bin <release codex.exe> ...` 在本任务中只能使用现有子命令做补充联调；不得把不存在的 refresh 子命令、`...` 占位符或人工脑补步骤写进执行记录。
- `cargo run -p codex-app-server-test-client` 在 [justfile](/I:/vscodeProject/codex/justfile) 里默认可能使用 debug `codex`；本任务后续执行时必须显式传 `--codex-bin <release codex.exe>`，不得误用 debug 路径替代 release 联调。
- Build Run 3 的 `CARGO_HOME=I:\cargo-home-local1` 只用于把 Cargo registry 与 `OUT_DIR` 放到同盘，避免 `v8` 的 Windows 跨盘 symlink 权限问题；它是执行态 workaround，不是对冻结 recipe 主命令体的改写。
- `F14-F15` 的正确口径是“未显式设置 `RUST_LOG` 时默认降噪”，不是“移除日志能力”；若需要最小烟测，也只能作为补充证据。
- 若后续执行阶段需要再写额外任务笔记、运行记录或截图索引，也必须继续写到本目录，不得散落到 `docs/`、项目根目录或源码目录。

## 用户/玩家视角直观变化清单

- 本次文档落盘阶段无用户/玩家可直接感知的直观变化。
- 本文定义的是后续验证任务，不是本轮功能实现；因此当前不会新增按钮、页面、弹窗、提示或运行时行为。

## 执行回写（2026-04-13）

### 执行摘要

- 已按 TASK 固定 recipe 完成 `codex.exe` 的 release 构建，最终成功产物为：
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist\release\codex.exe`
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-13_local1_checklist\release\codex.pdb`
- `release_build.log` 已记录：
  - Run 2 因 Windows 跨盘 `v8` symlink 权限问题失败。
  - Run 3 在不改 TASK 固定 build recipe 主体的前提下，仅通过 `CARGO_HOME=I:\cargo-home-local1` 规避跨盘 symlink 路径后成功。
- 已完成 release-only smoke：
  - `codex.exe --version`
  - `codex.exe --help`
  - `codex.exe app-server --help`
  - `codex.exe resume --help`
  - `codex.exe fork --help`
- 执行后半程遵循用户新增约束：
  - 采用已经编译好的 release 做校验。
  - 不再新增任何 debug 编译。
  - 因此未在该约束之后继续执行会触发 debug 编译的 `cargo test`、`cargo run` 或 `cargo build`。
- 逐项结论收口结果：
  - `F1-F9`, `F11-F15`, `A2` 已基于 release 证据、既有测试证据和静态核对收口。
  - `A1` 仅完成 TUI 路径与 app-server hook/snapshot 侧证据，缺少 TASK 原定的 release app-server `send-message-v2` / `resume-message-v2` 运行态链路，因此保持 `Partial / Blocked`。
  - `F10` 不能判定通过。现有 app-server 主证据里 7 个 refresh 目标用例仅 4 个通过，3 个失败，失败根因表现为 `plugins/featured?platform=codex` 额外请求污染 wiremock 计数。

### Raw Evidence

- [release_build.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_build.log)
- [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log)
- [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log)
- 关键附加证据：
  - [g1_codex_tui_run6_j1_final.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g1_codex_tui_run6_j1_final.stdout.txt)
  - [g2_codex_api_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2_codex_api_rerun.stdout.txt)
  - [g2g5_codex_core_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2g5_codex_core_rerun.stdout.txt)
  - [g3g4_codex_app_server.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g3g4_codex_app_server.stdout.txt)

### 最终结论表

| 条目 | 状态 | 主证据文件/位置 | 备注 |
|---|---|---|---|
| `F1` | `Passed` | [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log) | release `codex.exe --version` 返回 `codex-cli 0.120.0-local1`。 |
| `F2` | `Passed` | [g1_codex_tui_run6_j1_final.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g1_codex_tui_run6_j1_final.stdout.txt) | 最终 `cargo test -p codex-tui -j 1` 证据为 `1455 passed; 0 failed`。 |
| `F3` | `Passed` | [g1_codex_tui_run6_j1_final.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g1_codex_tui_run6_j1_final.stdout.txt) | `history_cell` / `update_prompt` 的 `local1` 版本显示链仍在。 |
| `F4` | `Passed` | [g1_codex_tui_run6_j1_final.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g1_codex_tui_run6_j1_final.stdout.txt) | local1 相关断言/快照保护仍存在且最终 TUI 跑绿。 |
| `F5` | `Passed` | [g2_codex_api_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2_codex_api_rerun.stdout.txt) | `/responses` retry 范围与非 `/responses` whitelist 边界保持成立。 |
| `F6` | `Passed With Core Caveat` | [g2g5_codex_core_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2g5_codex_core_rerun.stdout.txt) | 精确 OTEL / retry suppress 用例通过，但整 crate 仍有无关 baseline failure。 |
| `F7` | `Passed Static` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | `MAX_RETRY_DELAY = 10s` 静态口径仍在 request / stream 两侧保持一致。 |
| `F8` | `Passed` | [g2_codex_api_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2_codex_api_rerun.stdout.txt) | widened 分类与 budget/exhaustion 语义仍分离。 |
| `F9` | `Passed With Core Caveat` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | `session.rs` 显式透传 path，`telemetry.rs` 同时接受 `responses` / `/responses`。 |
| `F10` | `Failed` | [g3g4_codex_app_server.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g3g4_codex_app_server.stdout.txt) | 7 个 refresh 目标用例仅 4 个通过；3 个因 `plugins/featured` 额外请求污染 mock 计数失败。 |
| `F11` | `Passed Mixed` | [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log) + [g3g4_codex_app_server.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g3g4_codex_app_server.stdout.txt) | release `resume/fork --help` 证明 `--all` 保留；既有 app-server `thread_list` 证据证明无 provider filter 时返回全部 provider 并保留 `model_provider` 语义。 |
| `F12` | `Passed` | [g2_codex_api_rerun.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g2_codex_api_rerun.stdout.txt) | `/responses` 401 仍走普通 retry 链。 |
| `F13` | `Passed With Core Caveat` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | `force_gpt54_priority_fallback` 顶层默认/显式 false 行为均已静态和单测覆盖。 |
| `F14` | `Passed Static` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | app-server 默认日志口径仍是“未显式设置 `RUST_LOG` 时降噪”。 |
| `F15` | `Passed Static` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | TUI 默认日志口径仍是“未显式设置 `RUST_LOG` 时降噪”。 |
| `A1` | `Partial / Blocked` | [g1_codex_tui_run6_j1_final.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g1_codex_tui_run6_j1_final.stdout.txt) + [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log) | TUI 首次插入与不重复插入已证实；release `app-server --help` 只提供命令面证据，但 TASK 原定 release app-server `send-message-v2` / `resume-message-v2` 运行态链路未执行。 |
| `A2` | `Passed With Core Caveat` | [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) | 字段仍是顶层开关，并继续控制 `priority` / `Fast` 透传语义。 |

## Subagent严格复核附录

- reviewer subagent 执行口径：与主配置一致的 `gpt-5.4`、`xhigh`、`fast` 语义。
- reviewer subagent 的任务边界：只审主任务文档的正确性、完整性、边界与遗漏；不实施代码修改。

### 文档阶段复核（冻结计划阶段）

#### Finding 1

- 严重度：高
- 问题描述：当前文档的 `G1-G6` 验证矩阵过粗，却在 `Acceptance` 里宣称“已完整覆盖 `F1-F15` 与 `A1/A2`，没有遗漏条目”。
- 为什么是问题：
  - `A1` 明确要求同时覆盖 TUI 与 app-server 路径，但原文只落到了 `codex-tui`。
  - `F9` 应显式覆盖 [session.rs](/I:/vscodeProject/codex/codex-rs/codex-api/src/endpoint/session.rs) 与 [api_bridge.rs](/I:/vscodeProject/codex/codex-rs/core/src/api_bridge.rs)。
  - `F10/F11` 应显式覆盖 [windows_control.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/windows_control.rs)、[codex_message_processor.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs)、[lib.rs](/I:/vscodeProject/codex/codex-rs/tui/src/lib.rs)、[resume_picker.rs](/I:/vscodeProject/codex/codex-rs/tui/src/resume_picker.rs)、`thread/list.model_provider`。
- 修改建议：
  - 保留 `G1-G6` 作为功能组边界，但在组内继续展开为逐项矩阵。
  - 删除或改写“只靠组级矩阵就能证明完整覆盖”的强结论。
  - 在 `Detailed Execution Checklist` 和 `Acceptance` 中同步改成逐项收口。
- 建议落到文档的哪个章节：
  - `Context`
  - `Checklist 验证矩阵`
  - `Detailed Execution Checklist`
  - `Acceptance`

#### Finding 2

- 严重度：高
- 问题描述：原文把 `F10` 的关键联调写成 `cargo run -p codex-app-server-test-client -- --codex-bin <release codex.exe> ...`，但现有 test client 并没有可直接触发 `thread/providerRuntime/refresh` 或 `thread/providerRuntime/refreshAllLoaded` 的 CLI 子命令。
- 为什么是问题：
  - [lib.rs](/I:/vscodeProject/codex/codex-rs/app-server-test-client/src/lib.rs) 现有 CLI 子命令包括 `serve`、`send-message-v2`、`resume-message-v2`、`thread-resume`、`watch`、`model-list`、`thread-list` 等。
  - 真正的 refresh 入口真值在 [README.md](/I:/vscodeProject/codex/codex-rs/app-server/README.md) 的 JSON-RPC 方法 `thread/providerRuntime/refresh`、`thread/providerRuntime/refreshAllLoaded`，以及 [windows_control.rs](/I:/vscodeProject/codex/codex-rs/app-server/src/windows_control.rs) 的 `refresh_all_loaded_threads`。
  - 如果继续写成 `...` 或虚构 test client 子命令，后续执行记录会失真。
- 修改建议：
  - 将 `F10` 的主证据改为 `cargo test -p codex-app-server` 命中特定 refresh 测试。
  - 将 `codex-app-server-test-client` 改成“release 二进制 app-server 承载/连通性补充证据”，并写成明确、存在的固定 recipe。
  - 将 Windows tray / control plane 入口与预期输出单列写清。
- 建议落到文档的哪个章节：
  - `Context`
  - `Build Recipe`
  - `Checklist 验证矩阵`
  - `Detailed Execution Checklist`
  - `Notes`

#### Finding 3

- 严重度：中
- 问题描述：原文只给了 `release_build.log` 的较明确记录要求，而 `release_runtime_smoke.log` 与 `targeted_validation.log` 只有文件名和笼统摘要要求，没有固定字段 schema。
- 为什么是问题：
  - 若后续执行者只写简短备注，就无法保证运行态与定向验证的证据可审计、可复现、可关联到具体 `F*` / `A*` 条目。
- 修改建议：
  - 给 `release_runtime_smoke.log` 与 `targeted_validation.log` 补和 `release_build.log` 同等级的最小 schema。
  - 至少写明执行时间、工作目录、完整命令、关键环境变量或参数、退出码、stdout/stderr 摘录、关联条目、证据路径。
- 建议落到文档的哪个章节：
  - `Build Recipe`
  - `Detailed Execution Checklist`
  - `Acceptance`

### 执行阶段复核（2026-04-13 22:13:32 +08:00）

- reviewer subagent：`019d8725-9b82-73b2-bce7-8af713234cd2`
- 审核范围：主 TASK、[release_build.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_build.log)、[release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log)、[targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log)

#### Finding 4

- 严重度：高
- 问题描述：三类 raw evidence 实际没有一致满足本文写死的最小 schema。
- 为什么是问题：
  - [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log) 的各 run 当时缺 `arguments`、逐 run `evidence_paths` 与 `stderr_excerpt`。
  - [release_build.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_build.log) 的成功 run 当时缺 `stdout_excerpt` / `stderr_excerpt`。
  - [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) 的若干 mixed / static case 当时缺稳定的 `exit_code`、`stdout_excerpt` 或 `stderr_excerpt`。
- 修改建议：
  - 把 build / smoke / targeted validation 三类记录统一补齐到同一 schema。
  - 对 mixed evidence 显式写明 `exit_code` 语义，不能省略。
- 建议落到文档的哪个章节：
  - `执行回写（2026-04-13）`
  - 三类 raw evidence 日志本身
  - `Acceptance`

#### Finding 5

- 严重度：中
- 问题描述：最终结论表里 `F11` 的“主证据文件/位置”当时只写了 [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log)，但 `Passed Mixed` 实际依赖既有 app-server `thread_list` 证据。
- 为什么是问题：
  - 只看 smoke 无法单独证明 `thread/list.model_provider` 与默认无 provider filter 返回全部 provider 的语义。
  - 真正的 mixed 证据链已在 [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log) 中写明。
- 修改建议：
  - 把 `F11` 主证据列改成 smoke + app-server 既有测试证据。
  - 或者把 `F11` 备注收窄到 smoke 真正能证明的部分。
- 建议落到文档的哪个章节：
  - `最终结论表`

#### Finding 6

- 严重度：中
- 问题描述：文档顶部仍保留“本轮只落盘文档”的旧阶段表述，与后半部分已执行 build / smoke / validation 的事实冲突。
- 为什么是问题：
  - 这会误导读者把同一文件误读为纯计划文件，而忽略后半部分已经存在的执行回写。
  - 该冲突正好落在“冻结计划与执行回写如何共存”的边界上。
- 修改建议：
  - 把顶部改成“前半部分保留文档阶段冻结计划，后半部分为执行回写”，不要保留无条件“本轮只落盘文档”。
- 建议落到文档的哪个章节：
  - `输出位置说明`
  - `Summary`
  - `Notes`

#### Finding 7

- 严重度：中
- 问题描述：`Build Recipe` 写成只有冻结的 release recipe，但实际成功构建额外依赖 `CARGO_HOME=I:\cargo-home-local1`。
- 为什么是问题：
  - 当前写法会让读者误以为“冻结 recipe 本体已经被严格验证可成功”，但实际成功条件还包含执行态 same-drive workaround。
  - 若不明确区分，后续容易把计划命令体与成功环境混成一个概念。
- 修改建议：
  - 保持冻结 recipe 本体不变。
  - 在 `Build Recipe`、`执行回写（2026-04-13）` 与 `Notes` 中明确把 `CARGO_HOME` 标成执行态 workaround / 实际成功环境。
- 建议落到文档的哪个章节：
  - `Build Recipe`
  - `执行回写（2026-04-13）`
  - `Notes`

#### Finding 8

- 严重度：中
- 问题描述：appendices 原本没有清晰区分“文档阶段复核”和“执行阶段复核”，但 `Acceptance` 已写成“已回写 reviewer findings 与主 agent 处理结果”。
- 为什么是问题：
  - 这会让读者误判“执行后 reviewer 已完成且已收口”，却看不出当前页尾其实只有前一轮文档阶段 reviewer 的内容。
  - 也不利于遵守“同一文件末尾继续追加，不覆盖旧 reviewer 结果”的要求。
- 修改建议：
  - 在 `Subagent严格复核附录` 与 `主Agent审核处理结果` 中显式分轮次。
  - 在 `Acceptance` 里把“已回写”改成带阶段限定的表述。
- 建议落到文档的哪个章节：
  - `Subagent严格复核附录`
  - `主Agent审核处理结果`
  - `Acceptance`

### 执行阶段二次复核（2026-04-13 22:22:28 +08:00）

- reviewer subagent：`019d8736-2728-7c11-aae2-6222a1212e64`
- 复核结论：`no findings`
- 覆盖范围：
  - [local1_release构建与全量功能验真_TASK_2026-04-13.md](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/local1_release构建与全量功能验真_TASK_2026-04-13.md)
  - [release_build.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_build.log)
  - [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log)
  - [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log)
- reviewer 确认事项：
  - 三类 raw evidence 现在均满足 TASK 写死的最小 schema，mixed evidence 也已显式写出 `exit_code` 语义。
  - TASK 最终结论表与 `targeted_validation.log` 的 `Final Item Matrix` 已一致，重点核对的 `F10 / F11 / A1 / A2` 状态完全对齐。
  - 顶部双阶段表述、`CARGO_HOME=I:\cargo-home-local1` 的执行态 workaround 说明、以及 appendix 分轮次回写均已自洽。
  - 晚期用户约束“只采用已经编译好的 release 做校验，不再新增任何 debug 编译”仍然成立。
- reviewer 残余风险：
  - `A1` 仍应保持 `Partial / Blocked`，因为缺少 TASK 原定的 release app-server `send-message-v2` / `resume-message-v2` 运行态链路。
  - `F10` 仍应保持 `Failed`，因为 7 个 refresh 目标用例中仍是 4 个通过、3 个失败，失败根因仍是 `plugins/featured?platform=codex` 请求污染 wiremock 计数。
  - `F6 / F9 / F13 / A2` 继续保持 `Passed With Core Caveat`，因为当前结论建立在“精确相关用例通过 + 整体 core 证据里仍有无关 baseline failure”的边界上。

## 主Agent审核处理结果

### 文档阶段处理（冻结计划阶段）

#### 处理 1

- 结论：采纳
- 原因：reviewer 指出的遗漏面成立。原文只有组级矩阵，确实不足以支撑“已完整覆盖 `F1-F15` 与 `A1/A2`”的强结论。
- 实际改写位置：
  - `Context`：补入 `A1`、`F9`、`F10`、`F11` 的关键真值锚点
  - `Checklist 验证矩阵`：由粗粒度 `G1-G6` 改为“保留组边界 + 逐项条目矩阵”
  - `Detailed Execution Checklist`：改为逐项收口，并强制使用 `条目 | 状态 | 主证据文件/位置 | 备注`
  - `Acceptance`：删除仅凭组级矩阵即宣称完整覆盖的写法，改为逐项覆盖结论

#### 处理 2

- 结论：采纳
- 原因：reviewer 对 `F10` 的判断正确。test client 的现有 CLI 子命令无法直接作为 refresh RPC 真值入口，原文写法会误导后续执行。
- 实际改写位置：
  - `Context`：明确 `F10` 的真实入口在 app-server README 与 windows control plane
  - `Build Recipe`：将 raw evidence 的职责边界写清
  - `Checklist 验证矩阵`：把 `F10` 主证据改成 app-server refresh 测试，并将 test client 改成 release app-server 连通性补充证据
  - `Detailed Execution Checklist`：写死 `F10` 的固定验证 recipe，不再使用 `...`
  - `Notes`：显式写明 test client 没有 refresh 子命令，禁止脑补

#### 处理 3

- 结论：采纳
- 原因：`release_runtime_smoke.log` 与 `targeted_validation.log` 原先没有最小字段 schema，确实会导致证据粒度和可审计性不足。
- 实际改写位置：
  - `Build Recipe`：新增“三类 raw evidence 最小 schema”表格
  - `Detailed Execution Checklist`：要求先初始化日志字段，再执行 build / smoke / targeted validation
  - `Acceptance`：将“日志 schema 已写死”列为显式验收项

### 执行阶段处理（2026-04-13 22:13:32 +08:00）

#### 处理 4

- 结论：采纳
- 原因：执行阶段 reviewer 对 raw evidence schema 缺项的判断成立；这是 TASK 与日志自洽性的硬缺口，必须补齐。
- 实际改写位置：
  - [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log)：5 个 smoke run 全部补齐 `arguments`、逐 run `evidence_paths`、`stdout_excerpt`、`stderr_excerpt`
  - [release_build.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_build.log)：Build Run 3 补齐 `stdout_excerpt`、`stderr_excerpt`
  - [targeted_validation.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/targeted_validation.log)：Baseline + `V01-V10` 全部补齐 `finished_at`、`exit_code`、`stdout_excerpt`、`stderr_excerpt`
  - `Acceptance`：把 mixed evidence 的 `exit_code` 已显式写明纳入当前文档事实

#### 处理 5

- 结论：采纳
- 原因：`F11` 的 `Passed Mixed` 确实不能只靠 release `resume/fork --help` 支撑，还需要 app-server `thread_list` 既有证据。
- 实际改写位置：
  - `最终结论表`：`F11` 行的“主证据文件/位置”改为 [release_runtime_smoke.log](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/release_runtime_smoke.log) + [g3g4_codex_app_server.stdout.txt](/I:/vscodeProject/codex/.codexflow/临时/local1_release构建与全量功能验真_2026-04-13/g3g4_codex_app_server.stdout.txt)，备注同步写明两段证据各自证明的边界

#### 处理 6

- 结论：采纳
- 原因：顶部无条件“本轮只落盘文档”的表述已经不再符合当前文件状态，继续保留只会制造阶段冲突。
- 实际改写位置：
  - `输出位置说明`：改成“前半部分冻结计划 + 后半部分执行回写”的双阶段表述
  - `Execution Constraints`：补入“文档冻结阶段约束不等于执行回写阶段现状”的明确说明
  - `Notes`：强调若计划区与执行区冲突，以执行回写和 raw evidence 为准

#### 处理 7

- 结论：采纳
- 原因：`CARGO_HOME=I:\cargo-home-local1` 是 Build Run 3 实际成功的重要条件，但不能把它偷换成冻结 recipe 本体的一部分。
- 实际改写位置：
  - `Build Recipe`：新增“冻结 recipe 不改写 + `CARGO_HOME` 仅属执行态 workaround”的说明
  - `Acceptance`：新增“已区分冻结 recipe 与执行态 workaround”的验收口径
  - `Notes`：补充 same-drive workaround 的原因和边界

#### 处理 8

- 结论：采纳
- 原因：用户要求同一文件末尾继续追加 reviewer 结果，不覆盖旧结果；appendices 必须显式分轮次。
- 实际改写位置：
  - `Subagent严格复核附录`：拆为“文档阶段复核”和“执行阶段复核”两轮
  - `主Agent审核处理结果`：拆为“文档阶段处理”和“执行阶段处理”两轮
  - `Acceptance`：改成“按分轮次回写 findings 与处理结果”

### 执行阶段二次处理（2026-04-13 22:22:28 +08:00）

#### 处理 9

- 结论：采纳
- 原因：第二个 reviewer 返回 `no findings`，说明上一轮执行阶段修订已经把 reviewer 指出的结构性问题收口完毕；现阶段无需再改 TASK 或 raw evidence 的结论。
- 实际改写位置：
  - 本节仅追加“二次复核无新增问题”的正式收口记录。
  - 未再修改 `F10 / A1 / F6 / F9 / F13 / A2` 的状态；继续以现有 evidence 和 reviewer 残余风险说明为准。
