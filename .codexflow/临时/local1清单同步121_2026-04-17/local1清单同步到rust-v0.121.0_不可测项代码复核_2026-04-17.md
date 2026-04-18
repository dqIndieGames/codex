# local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17

## 复核边界

- 本报告是第二轮 reviewer 的正式“不可测项代码复核”结论，只覆盖无法由 release-only smoke 直接证明的条目。
- 本轮重点覆盖 `F2-F15`、`A1`、`A2`；`F1` 仅补做版本链静态 spot check，不重复输出新的 runtime 结论。
- 真值边界严格限制为三层：
  1. 官方 `rust-v0.121.0` release：<https://github.com/openai/codex/releases/tag/rust-v0.121.0>
  2. 官方 `rust-v0.120.0...rust-v0.121.0` compare：<https://github.com/openai/codex/compare/rust-v0.120.0...rust-v0.121.0>
  3. 当前工作区静态源码、当前 checklist、当前 TASK、以及既有 release-only smoke 证据
- 本报告不扩展为代码实现任务，不要求、不建议、也不依赖 `debug build`、`cargo test`、`cargo run`。
- 既有 smoke 证据仅作为已存在前提引用，不扩写为新的 runtime 结论：
  - `I:\vscodeProject\codex\tmp\agent-snapshots\smoke_version_stdout.txt`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\smoke_help_stdout.txt`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\smoke_app_server_help_stdout.txt`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\smoke_resume_help_stdout.txt`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\smoke_fork_help_stdout.txt`

## Findings

### Finding 1

- 严重度：中
- 问题描述：`F11` 的 `thread/list` / rollout summary 链路在历史元数据缺失 `model_provider` 时，会静默回填当前 fallback provider。这样只能证明“字段没有缺席”，不能证明“历史 provider 身份被真实保留”。
- 证据：
  - 官方 `rust-v0.121.0` release note 在 2026-04-15 明确把 Windows `resume --last` / `thread/list` cwd/session matching 修复列为 bug fix，并同时把 `codex-thread-store` 与 local thread listing 迁移列为 `0.121.0` 范围内的结构性变更。这说明 `F11` 所在链路正是本轮 121 同步的高风险热区。
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md` 的 `F11` 明确要求：`thread/list` 返回项需要保留 `model_provider`，避免跨 Provider 历史发现时 provider 身份被静默丢失。
  - `I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs:8334-8345` 的 `summary_from_thread_list_item(...)` 当前使用：
    - `it.model_provider.clone().unwrap_or_else(|| fallback_provider.to_string())`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs:8518-8521` 的 `read_summary_from_rollout(...)` 当前使用：
    - `session_meta.model_provider.clone().unwrap_or_else(|| fallback_provider.to_string())`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs:8578-8581` 的 `extract_conversation_summary(...)` 当前使用：
    - `session_meta.model_provider.clone().unwrap_or_else(|| fallback_provider.to_string())`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs:9163-9195` 的现有测试把这一行为固化为预期：当 rollout 里的 `SessionMeta.model_provider = None` 时，`read_summary_from_rollout(..., "fallback")` 返回的 `ConversationSummary.model_provider == "fallback"`。
  - 与此相对，`I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs:2863-2868` 这类 brand-new thread 路径会写入真实 `model_provider`；问题集中在 legacy / migrated / incomplete metadata 路径，而不是新线程创建路径。
- 为什么是问题：
  - `F11` 要求的是“跨 Provider 历史发现时仍能保留真实 provider 身份信息”，不是“列表里始终塞进一个非空字符串”。
  - 当前实现把“字段存在”和“身份真实”混成一件事。对缺失历史元数据的旧 rollout、迁移记录或不完整记录，`thread/list` 看起来像是保留了 provider，实际只是把当前 provider 补写进去。
  - 在官方 `0.121.0` 已经动到 Windows `thread/list` / `resume --last` 匹配和 thread-store listing 迁移的前提下，这个 provenance 缺口会直接削弱 `F11` 的收口可信度。
- 修改建议：
  - 在没有额外证据证明“所有历史记录都带真实 `model_provider`”之前，不应把 `F11` 收口为 `Passed Static` 或 `Passed Mixed`。
  - 若后续要修正，实现层应避免把缺失历史 provider 静默回填成当前 fallback provider；更稳妥的方向是保留 `unknown/missing` 语义，或补充可证明的迁移链证据。
  - 本轮 reviewer 建议保持 `F11 = Blocked`，其余条目可继续按当前静态证据收口。
- 建议落到章节：
  - `local1-custom-feature-checklist-2026-03-28.md` 的 `F11 | 历史默认不按 Provider 切割`
  - `local1清单同步到rust-v0.121.0_TASK_2026-04-17.md` 的 `最终证据矩阵`
  - `local1清单同步到rust-v0.121.0_TASK_2026-04-17.md` 的 `主Agent审核处理结果`

## 条目级代码复核结论

| 条目 | 建议状态 | 依据文件 | 复核结论 |
|---|---|---|---|
| `F1` | `Passed Mixed` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md`；`I:\vscodeProject\codex\codex-rs\Cargo.toml`；`I:\vscodeProject\codex\codex-rs\Cargo.lock`；`I:\vscodeProject\codex\codex-rs\cli\src\main.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\version.rs`；`I:\vscodeProject\codex\tmp\agent-snapshots\smoke_version_stdout.txt` | 既有 smoke 已证明 `codex-cli 0.121.0-local1`；本轮静态 spot check 也确认工作区版本链已对齐 `0.121.0`，且 CLI/TUI 显示后缀仍为 `-local1`。 |
| `F2` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\tui\src\app.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\status\card.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\chatwidget\tests.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\status\tests.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\version.rs` | 状态卡片、状态区和相关测试/快照仍统一依赖 `CODEX_CLI_DISPLAY_VERSION`，未被 121 文本面改写。 |
| `F3` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\tui\src\history_cell.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\update_prompt.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\chatwidget\tests.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\version.rs` | 历史单元和升级提示链路仍引用 local1 版本显示口径，未被 Guardian/TUI history 相关 121 改动冲掉。 |
| `F4` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\tui\src\app.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\history_cell.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\chatwidget\tests.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\update_prompt.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\status\tests.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\status\snapshots\` | 当前静态守护面仍同时覆盖断言与快照，符合 checklist 已冻结的 `F4` 审查口径。 |
| `F5` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\telemetry.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs` | `/responses` 路径仍统一把远端 HTTP 状态视为可重试，`401` 仍在覆盖范围内；非 `/responses` 端点继续保留旧 whitelist。 |
| `F6` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\codex-api\src\telemetry.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs`；`I:\vscodeProject\codex\codex-rs\otel\src\events\session_telemetry.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\endpoint\session.rs` | request/websocket 主链 suppress 与 metrics 保留逻辑仍集中在统一入口，静态上未见 retry 中间态回流到历史面。 |
| `F7` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\core\src\util.rs`；`I:\vscodeProject\codex\codex-rs\codex-client\src\retry.rs` | request 与 stream 两条退避链的单次等待上限仍被 clamp 到 `10s`。 |
| `F8` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\core\src\model_provider_info.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex.rs` | bounded / unbounded 预算语义、端点级例外和 exhaustion 分界仍在，未被 121 改写。 |
| `F9` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\telemetry.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\endpoint\session.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex.rs` | retry 分类、route/path 透传、request/websocket suppress 和 UI retry 详情仍沿统一主链工作。 |
| `F10` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\scripts\windows_app_server_refresh_tray.py`；`I:\vscodeProject\codex\codex-rs\app-server\README.md`；`I:\vscodeProject\codex\codex-rs\app-server\tests\suite\v2\thread_provider_runtime_refresh.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex_tests.rs` | Windows tray 仍是 config-first：source 只读显式 `[model_providers.*]`，target 固定顶层 `model_provider`，写回只覆盖 `base_url` 与 `experimental_bearer_token`。 |
| `F11` | `Blocked` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md`；`I:\vscodeProject\codex\codex-rs\exec\src\lib.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\app_server_session.rs`；`I:\vscodeProject\codex\codex-rs\tui\src\lib.rs`；`I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs`；`I:\vscodeProject\codex\tmp\agent-snapshots\smoke_resume_help_stdout.txt`；`I:\vscodeProject\codex\tmp\agent-snapshots\smoke_fork_help_stdout.txt` | 默认跨 provider discover、`--all` / cwd 语义、embedded/remote 注入边界都还在，但 `thread/list` 对缺失历史 `model_provider` 的记录会回填当前 provider，无法静态证明 provider 身份被真实保留。 |
| `F12` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`；`I:\vscodeProject\codex\codex-rs\codex-api\src\telemetry.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex.rs` | `/responses` 主链上的 `401` 仍走统一普通 retry classifier，请求层、websocket 和 fallback 路径未恢复旧的 unauthorized recovery 优先分支。 |
| `F13` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\core\src\config\mod.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client_tests.rs`；`I:\vscodeProject\codex\codex-rs\core\config.schema.json` | 顶层 `force_gpt54_priority_fallback`、`gpt-5.4` priority fallback、`Fast` 关闭与 `Flex` 保留语义仍一致。 |
| `F14` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\app-server\src\lib.rs` | app-server 默认日志过滤仍为 `warn`，显式 `RUST_LOG` 覆盖能力仍在。 |
| `F15` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\tui\src\lib.rs`；`I:\vscodeProject\codex\docs\install.md` | TUI 默认日志过滤仍为 `warn`，`codex-tui.log` 仍保留，文档口径与实现一致。 |
| `A1` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\core\src\stream_events_utils.rs`；`I:\vscodeProject\codex\codex-rs\core\src\stream_events_utils_tests.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex.rs`；`I:\vscodeProject\codex\codex-rs\core\src\codex_tests.rs` | `brand-new / Clear + 单个纯文本 你好` 触发条件、`SessionSource::SubAgent` / `SessionSource::Mcp` 边界，以及三条可见消息消费链共用同一前缀逻辑的静态约束仍成立。 |
| `A2` | `Passed Static` | `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；`I:\vscodeProject\codex\codex-rs\core\src\config\mod.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client.rs`；`I:\vscodeProject\codex\codex-rs\core\src\client_tests.rs`；`I:\vscodeProject\codex\codex-rs\core\config.schema.json` | `force_gpt54_priority_fallback` 仍是顶层 `config.toml` 字段；静态上没有 `[profiles.*]` 覆盖入口。 |

## 结论摘要

- 本轮第二次 reviewer 代码复核共确认 `1` 条 finding，按当前证据应收口为 `1` 个 `Blocked` 条目：`F11`。
- 除 `F11` 外，未发现新的阻断性问题；其余 `F2-F15`、`A1`、`A2` 可按当前静态证据继续收口，其中 `F1` 维持既有 smoke + spot check 的 `Passed Mixed`。
- 残余风险：
  - `F6`、`F10`、`A1` 的结论仍以源码 / 测试 / 文档静态真值为主，没有新增 live runtime 证据。
  - `F1` 的 runtime 通过结论完全继承既有 smoke，本报告没有重复做新的运行时验证。
  - 官方 `rust-v0.120.0...rust-v0.121.0` compare 涉及范围较大；本轮 reviewer 仍只针对 checklist / TASK 已冻结的 local1 条目收口，不把 121 的其他官方新能力混入 local1 结论。
