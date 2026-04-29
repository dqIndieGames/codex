# runtime负担开关默认关闭_TASK_2026-04-29

## Context

- 本文最初用于冻结后续代码修改范围；本次已按用户要求进入代码实现执行，执行过程记录在 `.codexflow/临时/runtime默认关闭代码执行_2026-04-29/runtime默认关闭代码执行_总控记录_2026-04-29.md`。
- `rollout`（中文解释：会话过程记录，用户之后能恢复或回放历史）批量 flush 属于 runtime 优化，但默认必须关闭，不能静默改变历史落盘安全感。
- `app-server`（中文解释：桌面端、前端或其他客户端连接 Codex 的后台服务）高频通知合并属于 runtime 优化，但默认必须关闭，不能静默改变客户端看到命令输出、文件变更、token usage、diff/plan 更新的刷新方式。
- `analytics`（中文解释：产品使用统计，用户通常看不到）、`feedback`（中文解释：用户主动反馈和反馈日志上传能力）、`log_db`（中文解释：本地 sqlite 日志入库，主要用于排查问题）需要分层配置开关，并在 local2 默认关闭；需要时通过 `config.toml` 显式打开。
- 当前代码线索显示：
  - `codex-rs/rollout/src/recorder.rs` 中 rollout JSONL 写入存在逐行 `flush` 和批次末尾 `flush` 路径。
  - `codex-rs/app-server/src/outgoing_message.rs` 和 `codex-rs/app-server/src/bespoke_event_handling.rs` 负责高频通知发送与构造。
  - `codex-rs/config/src/types.rs` 和 `codex-rs/core/src/config/mod.rs` 已存在 `analytics` / `feedback` 配置入口，但默认口径需要按本文调整。
  - `codex-rs/app-server/src/lib.rs` 是 app-server 启动时挂载 feedback logger layer、feedback metadata layer、log_db layer 的关键入口。
  - `codex-rs/state/src/log_db.rs` 当前提供 log db layer，后续需要接入可配置开关。

## Goal

- 新增或调整配置，使三类 runtime 负担能力都遵守“默认关闭，显式开启”的 local2 口径。
- 保证默认行为更轻：默认不启用 rollout 批量 flush 优化、不启用 app-server 高频通知合并、不启用 analytics / feedback / log_db。
- 保证可恢复能力：用户或开发者需要这些能力时，可以通过 `config.toml` 明确打开。
- 把回归保护写入 `docs/local2-custom-feature-checklist-2026-04-27.md`，后续合并 upstream 时逐条核对。

## Checklist

- [ ] **C1. 冻结配置口径**
  明确配置命名和默认值。建议将 rollout 批量 flush、app-server 高频通知合并放在独立 runtime 优化配置下，默认值均为 `false`；`analytics.enabled`、`feedback.enabled`、`log_db.enabled` 默认值均为 `false`。如果最终命名不同，必须在代码、schema、文档和测试中保持同一口径。

- [ ] **C2. 实现 rollout 批量 flush 可选开关**
  修改 `codex-rs/rollout/src/recorder.rs` 及必要调用链，让批量 flush 只在配置显式开启时生效；默认路径继续保持当前落盘语义。开启后必须保留强制 flush 点，例如 turn 完成、shutdown、显式 flush、关键 session metadata 写入、fork 前父线程落盘。

- [ ] **C3. 实现 app-server 高频通知合并可选开关**
  修改 `codex-rs/app-server/src/outgoing_message.rs`、`codex-rs/app-server/src/bespoke_event_handling.rs` 及必要协议调用点。默认不合并；开启后才允许对 command output delta、file change delta、token usage、diff updated、plan updated 做 100-250ms 窗口合并或按块发送。交互式终端输入、错误、完成事件、需要即时反馈的状态事件不得被延迟到影响用户判断。

- [ ] **C4. 实现 analytics / feedback / log_db 默认关闭**
  修改 `codex-rs/config/src/types.rs`、`codex-rs/core/src/config/mod.rs`、`codex-rs/app-server/src/lib.rs`、`codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/state/src/log_db.rs` 及相关初始化路径。默认不发送 analytics、不允许 feedback 上传、不挂载 feedback logger layer、不挂载 feedback metadata layer、不启动或不接入 log_db layer；当 `config.toml` 显式打开对应项时，恢复对应能力。

- [ ] **C5. 更新配置 schema 与用户文档**
  若新增或调整 `ConfigToml` 字段，更新 `codex-rs/core/config.schema.json`；同步 `docs/config.md` 或等效配置文档，写清默认关闭、开启示例、用户可见影响和排查取舍。

- [ ] **C6. 补测试覆盖**
  覆盖默认关闭和显式开启两组路径：rollout 批量 flush 默认不改变现有写盘语义；app-server 高频通知默认逐条发送、开启后合并；analytics / feedback / log_db 默认关闭，显式开启后恢复。涉及 config schema 的测试按项目既有流程更新。

- [ ] **C7. 回写 local2 长期清单**
  保持 `docs/local2-custom-feature-checklist-2026-04-27.md` 中 `L2-F22`、`L2-F23`、`L2-F24` 的口径：默认关闭、显式开启、用户影响写清楚；后续合并 upstream 时必须逐条核对这些条目没有被冲掉。

- [ ] **C8. Subagent 文档审核**
  审核范围：`docs/local2-custom-feature-checklist-2026-04-27.md`、`docs/runtime-load-reduction-default-off-task-2026-04-29.md`。
  关联代码真值范围：`codex-rs/rollout/src/recorder.rs`、`codex-rs/app-server/src/outgoing_message.rs`、`codex-rs/app-server/src/bespoke_event_handling.rs`、`codex-rs/config/src/types.rs`、`codex-rs/core/src/config/mod.rs`、`codex-rs/app-server/src/lib.rs`、`codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/state/src/log_db.rs`。
  一致性审核：检查默认关闭、显式开启、用户影响、配置入口、强制 flush 点、高频通知例外项是否互相冲突。
  遗漏性审核：检查 Checklist 与 Acceptance 是否覆盖 rollout、app-server、analytics、feedback、log_db、schema、测试、local2 清单回写；若无发现，分别写明“一致性无发现”和“遗漏性无发现”。

## Acceptance

- [ ] **A1. 默认关闭口径成立**
  未显式配置时，rollout 批量 flush 优化、app-server 高频通知合并、analytics、feedback、log_db 均不启用；用户默认获得更轻、更安静的运行时行为。

- [ ] **A2. config.toml 显式开启可用**
  用户在 `config.toml` 中打开对应开关后，相关能力恢复或启用；每个开关互相独立，不得打开一个能力时连带打开其他无关能力。

- [ ] **A3. rollout 历史安全点保留**
  开启 rollout 批量 flush 后，turn 完成、shutdown、显式 flush、fork 前父线程落盘等关键点必须强制落盘；默认关闭时不得改变当前历史恢复口径。

- [ ] **A4. app-server 用户反馈不被误延迟**
  开启通知合并后，命令输出和文件变更可以小批量更新，但错误、完成、交互式终端输入和关键状态变化必须及时送达；默认关闭时仍保持当前逐条通知语义。

- [ ] **A5. 文档与测试同步**
  配置 schema、配置文档、local2 长期清单和测试覆盖均同步更新；subagent 文档审核已完成，并且一致性问题和遗漏性问题都已处理或明确记录。

## Notes

- 本次执行允许并实际包含代码修改；总控记录负责沉淀执行证据、验证结果、GitHub release 结果和 subagent 审核结论。
- “默认关闭”是 local2 口径，不等同于 upstream 官方默认值；合并 upstream 后必须按 local2 清单复核。
- rollout 批量 flush 的核心风险是极端崩溃时最后少量记录未及时写盘，因此必须有关键强制 flush 点。
- app-server 高频通知合并的核心风险是用户误以为命令没有输出或状态没有变化，因此交互、错误和完成事件必须保留即时性。
- analytics / feedback / log_db 关闭后，用户默认少一些后台统计、反馈日志和本地日志入库；需要排查问题或上传反馈时，必须能通过配置打开。

## 用户/玩家视角直观变化清单

- 默认情况下，用户不会看到新的按钮、页面或弹窗。
- 默认情况下，Codex 运行更安静：少做统计、反馈日志采集和 sqlite 日志入库；如果后续代码实现完成，默认也不会自动启用 rollout 批量 flush 或 app-server 通知合并。
- 当用户在 `config.toml` 中显式打开某个开关后，才会感受到对应变化：历史写盘可能更少打扰磁盘、命令输出可能变成小批量刷新、analytics / feedback / log_db 能力恢复。
- 如果用户不改配置，现有可见交互应保持稳定，不应突然少历史、不应突然延迟输出、不应突然上传反馈或写入额外日志。
