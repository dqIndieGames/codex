# local1_首次对话清单与gpt54优先级开关_TASK_2026-04-10

对应基线文档：[local1-custom-feature-checklist-2026-03-28.md](E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md)

文档目的：冻结本轮 local1 文档、归档与后续实现边界。当前轮次先完成 TASK 文档落盘、严格复核、主 agent 复写与 checklist 归档，不执行任何代码修改。

## Context

- 当前 local1 基线文档已经冻结了 `-local1` 显示链、`/responses` 主链重试增强、provider runtime 热刷新、跨 provider 历史发现、`gpt-5.4 priority` 请求兜底、Windows/TUI 默认日志降噪等长期保留能力。
- 本轮新增并冻结两项新能力：首次对话固定显示 local1 自定义清单；为 `gpt-5.4` 的 `/responses` `service_tier=priority` 兜底增加一个可关闭的顶层配置开关 `force_gpt54_priority_fallback`。
- 首次对话清单展示口径已经明确为：覆盖 TUI 与 app-server 路径；不是应用启动即展示；而是在新对话的第一条用户消息提交后立即插入固定清单，且每个新对话只插入一次；resume、continue、历史线程重开与同线程后续轮次都不得重复插入；实现路线冻结为内建主逻辑注入，不依赖 hook、启动 banner、外层包装器或旁路拦截链。
- `force_gpt54_priority_fallback` 的配置口径已经明确为：只支持顶层 `config.toml`；省略时默认按 `true` 处理；显式写 `false` 时关闭 `gpt-5.4` 的 priority 强制兜底；不支持 `[profiles.*]` 版本。
- 本轮交付不是代码实现文档，也不是变更 patch；重点是把需求边界、配置口径、归档目标、验收标准和复核流程一次写清，避免后续执行时再出现歧义。

## Goal

- 产出一份可直接指导后续实现的 local1 TASK 文档，完整覆盖本轮 12 项需求，不遗漏、不改意图。
- 明确首次对话清单展示、`gpt-5.4 priority` 开关、provider/runtime/history/logging 相关能力的冻结口径。
- 明确本轮正式产物输出路径、review 流程、回写归档目标与最终验收标准。
- 确保后续执行时只需按本文落地，不再对关键配置作用域、展示时机、实现路线和归档目标做二次猜测。

## Checklist

1. 全链路版本显示统一保留 `-local1`：CLI、TUI、状态区、历史单元、升级提示及其他用户可见版本展示继续统一保留 `-local1` 后缀，不允许出现裸官方版本回流。
2. 首次对话固定显示本清单，不显示随机启动提示：TUI 与 app-server 路径仅在新对话的第一条用户消息提交后立即插入一次固定 local1 清单，替代随机启动提示；同线程后续轮次、resume、continue、历史线程重开均不得重复插入；该能力必须由首轮主逻辑内建注入，禁止把该能力实现成应用启动即显示、hook、启动 banner、外层包装器或旁路拦截链。
3. `/responses` 除 `401` 外统一自动重试：所有非 `401` 的 `/responses` 远端错误继续进入自动重试；`401` 必须优先进入认证恢复，不直接按普通重试处理。
4. 重试中间态只更新状态，不写入历史：request/websocket/reconnect 的 retry 中间态继续只更新状态区与状态详情，不在历史区新增脏错误单元；重连详情与 metrics 必须继续保留。
5. 单次重试等待上限保持 `10s`：扩大 `/responses` 主链自动重试分类后，单次退避等待上限仍保持 `10s`，不得改变现有 retry budget 的 bounded/unbounded 语义。
6. 保留 `providerRuntime refresh / refreshAllLoaded`：继续保留 thread 级 provider runtime 刷新与批量刷新入口，并冻结只热刷新 `base_url` 与 `experimental_bearer_token`，不把其他 provider 配置混入 runtime 热刷新范围。
7. `[agents.*].config_file` 继续按用户 `config.toml` 相对解析：`refresh` 与 `refreshAllLoaded` 场景下，agent config 相对路径继续以用户 `config.toml` 所在目录为基准解析；Windows tray、app-server、named pipe 批量刷新链路继续可用。
8. 历史与 resume 默认支持跨 provider：历史列表、最近会话、resume picker 默认继续支持跨 provider 发现，同时继续保留 `model_provider` 字段，不丢失 provider 身份信息；跨 provider 发现不改变现有 continue/resume provider 选择语义，CLI 与 embedded TUI 继续旧线程时仍使用当前 provider，remote TUI 维持现状，不自动切换到历史线程记录的 provider。
9. `gpt-5.4` 的 `/responses` 请求继续强制 `service_tier=priority`：在默认口径下，`gpt-5.4` 的 `/responses` 出站请求继续强制携带 `service_tier=priority`，避免旧线程或旧 session 缺失 tier 时回退。
10. Windows app 与 TUI 默认日志继续降噪：未显式设置 `RUST_LOG` 时，Windows app 与 TUI 继续使用降噪后的默认日志口径；显式设置 `RUST_LOG` 后再恢复详细日志。
11. 新增顶层配置 `force_gpt54_priority_fallback`：该字段只允许写在顶层 `config.toml`；省略不填时默认视为 `true`；显式写 `force_gpt54_priority_fallback = false` 时关闭 `gpt-5.4` 的 `/responses` `service_tier=priority` 强制兜底；此前的 `[profiles.local1]` 示例不采纳。
12. 将“首次对话清单与 gpt54 优先级开关”功能写回基线 checklist 归档：本轮执行需要把这次新增的“首次对话固定清单展示”和“`force_gpt54_priority_fallback` 顶层开关”回写到 [local1-custom-feature-checklist-2026-03-28.md](E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 中，作为 local1 自定义范围的正式归档记录；现有 checklist 文档是本轮唯一允许回写的 repo tracked 归档载体，不另起独立 archive 文档。

## Acceptance

- TASK 文档正文必须完整包含 `Context`、`Goal`、`Checklist`、`Acceptance`、`Notes`、`用户/玩家视角直观变化清单` 六个主体章节，以及文末的 review/处理结论章节。
- 12 项 Checklist 必须全部出现，且语义与已冻结计划一致，不得合并丢项、改写成别的实现路线或遗漏 app-server、history/resume、Windows tray 等关键边界。
- `force_gpt54_priority_fallback` 必须明确写成“仅顶层 `config.toml` 支持”，并同时写清默认值为 `true`、显式 `false` 的关闭效果、`[profiles.*]` 不支持。
- 首次对话清单展示必须明确写成“仅在新对话的第一条用户消息提交后插入一次固定清单”，并明确覆盖 TUI 与 app-server；同线程后续轮次、resume、continue、历史线程重开均不得重复插入；固定清单必须由 TUI 与 app-server 的首轮主逻辑自然注入，禁止写成启动提示、随机提示、hook 注入、外层包装器或旁路拦截链。
- 验收场景至少覆盖以下四类：省略 `force_gpt54_priority_fallback` 时默认按 `true` 处理；显式 `false` 时关闭 `gpt-5.4` 的 priority 兜底；非 `gpt-5.4` 请求不受该兜底逻辑误伤；归档文档需同步记录本次新增能力。
- `force_gpt54_priority_fallback = true` 必须与省略该配置时的行为完全等价；任何 `[profiles.*].force_gpt54_priority_fallback` 都不得覆盖顶层配置，且应视为不支持。
- `codex/.codexflow/临时/local1_首次对话清单与gpt54优先级开关_2026-04-10/` 仅承载本轮 TASK、review 与中间文档产物；正式归档必须回写到现有 [local1-custom-feature-checklist-2026-03-28.md](E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md)，不得另起独立 archive 文档。
- 本轮执行边界必须明确写清：允许创建/修改文档文件；禁止进行任何代码逻辑修改、格式化源码、运行会改写 repo tracked 代码的流程。
- reviewer subagent 的问题清单与修改建议必须写入本文末尾；主 agent 必须回读并给出逐项处理结论，不允许 review 结果只停留在聊天窗口。

## Notes

- 本文件是“文档执行任务单”，不是代码实现说明文档；除非后续进入代码实现阶段，否则不展开函数级改法、patch 位置或实现细节。
- `codex/.codexflow/临时/local1_首次对话清单与gpt54优先级开关_2026-04-10/` 只承载本轮 TASK、review 与中间文档产物；现有 [local1-custom-feature-checklist-2026-03-28.md](E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 是本轮唯一允许回写的 repo tracked 归档文档。
- 对应基线 checklist 的归档回写属于文档归档动作，不属于代码修改；但归档内容必须与本文件冻结口径一致，避免出现“TASK 文档一套口径、归档文档另一套口径”的分裂，也不得另起平行 archive 文档。
- 如果后续实现时发现旧文档与本文件冲突，以本文件本轮冻结口径与 2026-04-10 归档补充为准，并同步修正基线 checklist 的归档说明。
- `force_gpt54_priority_fallback` 的有效来源只有顶层 `config.toml`；省略与显式 `true` 等价，显式 `false` 才表示关闭；任何 `[profiles.*]` 下的同名字段都不得生效。
- 历史与 resume 的跨 provider 发现边界只影响“发现/展示”，不改变 continue/resume 的既有 provider 选择语义；`model_provider` 字段必须继续保留。
- 若后续再次扩展 local1 私有能力，应继续按“先补 TASK 文档、再补基线 checklist 归档”的顺序执行，避免功能定义只存在于聊天记录。

## 用户/玩家视角直观变化清单

- 用户在每个新对话的第一条真实消息提交后，不再看到随机启动提示，而是只会看到一次固定的 local1 自定义清单；同一线程后续轮次或恢复旧线程时不会重复刷出这串清单。
- 用户在 CLI、TUI、状态区、历史单元等版本展示位置继续看到带 `-local1` 的版本身份，不会误以为自己正在使用纯官方版本。
- 使用 `gpt-5.4` 且未关闭开关时，请求继续自动兜底为 `service_tier=priority`；需要关闭该行为时，用户只需在顶层 `config.toml` 里显式写 `force_gpt54_priority_fallback = false`。
- 用户在重试、历史、resume、provider refresh、Windows tray/app-server 批量刷新、默认日志降噪等既有 local1 能力上，不会因为这次文档冻结而丢失原有行为边界。
- 本次修改不直接新增可点击按钮或页面；直观变化主要体现在首次对话提示内容、配置项口径更明确，以及 local1 私有能力被正式归档，后续更容易持续维护。

## Reviewer Subagent 严格复核结论（2026-04-10）

### 总体结论

当前 TASK 文档主线正确，12 项需求主体基本齐全，但仍有 5 处需要补强。未补强前，不建议将本文视为“后续实现无需二次猜测”的最终冻结稿。主要缺口集中在：首次对话清单的触发边界、内建主逻辑注入、归档写回落点与输出目录表述冲突、`force_gpt54_priority_fallback` 的负向验收，以及 history/resume 的 provider 延续边界。

残余风险：本次结论基于主文档与基线文档的静态对照；若聊天记录中还有未落入文档的冻结口径，仍需按聊天原文再做一轮逐条对照。

### 问题清单

1. 高：首次对话固定清单的“首次”边界仍不够清楚。当前只写“首个用户消息提交后立即插入固定清单”，但没有明确这是“新对话首轮仅一次”，也没有明确 `resume / continue / 历史线程重开` 时是否禁止重复插入。建议在 `Checklist`、`Acceptance`、`Notes` 中明确：仅在新对话的第一条用户消息提交后插入一次；后续轮次、resume、continue、历史线程重开均不得重复插入；TUI 与 app-server 两路按同一规则验收。
2. 高：归档写回目标与输出目录说明存在冲突。正文要求把新增能力回写到现有 `docs/local1-custom-feature-checklist-2026-03-28.md`，但 `Notes` 又写“本轮正式新产物输出目录按临时目录执行，不写入源码目录”，容易把“归档回写”误读成也只能落在临时目录。建议明确：临时目录只承载 TASK/review/中间文档产物；现有 checklist 文档是本轮唯一允许回写的 repo tracked 文档，不得另起独立 archive 文档。
3. 中：`内建主逻辑注入` 仍停留在 `Context`，尚未上升为硬验收。当前只写“禁止 hook 注入”，但没有正向写死“必须落在内建主逻辑路径”。建议在 `Acceptance` 增补：固定清单必须由 TUI 与 app-server 的首轮主逻辑自然注入，不得通过 hook、启动 banner、外层包装器或旁路拦截链实现。
4. 中：`force_gpt54_priority_fallback` 的负向验收不完整。虽然正文写了“仅顶层”“默认 true”“显式 false”“不支持 profiles”，但 `Acceptance` 没有专门覆盖 `[profiles.*]` 无效/不生效，也没有覆盖显式 `true` 与省略等价。建议补充这两类验收场景。
5. 中：history/resume 的 provider 延续边界在 TASK 中被压缩丢失。基线文档已冻结“跨 provider 发现”与“continue/resume 时不自动改写既有 provider 语义”是两件事；TASK 当前只保留了前者。建议补一句：跨 provider 发现不改变 CLI/embedded TUI 与 remote TUI 既有 provider 选择/注入语义。

### 建议补充的冻结口径

- 首次对话固定清单仅在“新对话的第一条用户消息提交后”插入一次；resume、continue、历史线程重开或后续轮次不得重复插入。
- 首次对话固定清单必须覆盖 TUI 与 app-server 两条自然主路径，并由内建主逻辑直接注入；不得通过 hook、启动提示、随机提示替换器、外层包装器或旁路拦截链实现。
- `force_gpt54_priority_fallback` 仅允许出现在顶层 `config.toml`；省略与显式 `true` 等价；显式 `false` 仅关闭 `gpt-5.4` 的 `/responses` `service_tier=priority` 强制兜底；任何 `[profiles.*]` 下的同名字段都不得生效。
- `codex/.codexflow/临时/...` 目录仅用于本轮 TASK、review 与中间文档产物；正式归档必须回写到既有 `docs/local1-custom-feature-checklist-2026-03-28.md`，不得另起独立 archive 文档。
- 历史与 resume 的“跨 provider 发现”不改变既有 continue/resume provider 选择语义；`model_provider` 字段必须继续保留。

## 主Agent复核处理结论

1. 采纳：已在 `Context`、`Checklist`、`Acceptance`、`Notes`、`用户/玩家视角直观变化清单` 中补足“新对话首轮仅一次”的边界，并明确同线程后续轮次、resume、continue、历史线程重开都不得重复插入。
2. 采纳：已把临时目录与正式归档目标拆开表述，明确 `.codexflow/临时/...` 只承载 TASK/review/中间文档产物，现有 checklist 文档是本轮唯一允许回写的 repo tracked 归档载体。
3. 采纳：已把“内建主逻辑注入”从 `Context` 提升到 `Checklist` 和 `Acceptance` 的硬约束，明确禁止 hook、启动 banner、外层包装器和旁路拦截链。
4. 采纳：已在 `Acceptance` 与 `Notes` 中补上 `force_gpt54_priority_fallback = true` 与省略等价、`[profiles.*]` 同名字段无效且不支持的负向验收。
5. 采纳：已在 `Checklist` 与 `Notes` 中恢复 history/resume 的 provider 延续边界，明确“跨 provider 发现”不改变 CLI、embedded TUI 与 remote TUI 的既有 provider 选择语义。
6. 审核结论：5 条 reviewer 建议均已采纳，并已重写正文；当前版本可以作为后续实现与归档的正式 TASK 冻结稿。

## 执行期静态审核记录（2026-04-10）

- 审核边界：本轮只阅读当前脏工作区源码、静态 diff、现有测试与现有文档；不编译、不跑测试、不格式化、不生成代码。
- 审核对象：以当前实际存在的内层 TASK 与内层仓库 `E:/vscodeProject/codex_github/codex` 为准；外层 `.codexflow` 路径未作为执行目标。
- 总体判断：当前改动为“部分满足”。静态证据明确显示第 2 项与第 11 项未满足，第 4 项只能判为部分满足，其余条目静态上基本可成立。

### 1. 全链路版本显示统一保留 `-local1`

- 状态：已满足
- 证据位置：`codex-rs/cli/src/main.rs:56,64`，`codex-rs/tui/src/version.rs:3`，`codex-rs/tui/src/status/card.rs:418`，`codex-rs/tui/src/app.rs:1457,8701`，`codex-rs/tui/src/chatwidget/status_surfaces.rs:481`，`codex-rs/tui/src/history_cell.rs:536`，`codex-rs/tui/src/update_prompt.rs:112`
- 静态判断：CLI 与 TUI 两条用户可见版本展示链都仍然直接消费 `CODEX_CLI_DISPLAY_VERSION`，其值固定追加 `-local1`。
- 缺口描述：本轮静态阅读未见裸官方版本回流点。
- 是否建议修复：否。

### 2. 首次对话固定显示本清单，不显示随机启动提示

- 状态：未满足
- 证据位置：`codex-rs/tui/src/chatwidget.rs:2006-2015,2025-2030`，`codex-rs/tui/src/history_cell.rs:1155-1198,3135-3147`
- 静态判断：当前仍在 `SessionConfigured` 阶段调用 `new_session_info(...)`；随后才继续处理 `initial_user_message`。`is_first_event` 分支插入的仍是 `/init`、`/status`、`/permissions`、`/model`、`/review` 欢迎提示，现有测试也仍在锁定这条首事件欢迎链。
- 缺口描述：这不是“local1 固定清单还没接上”那么简单，而是“当前触发时机与冻结口径相反”。冻结口径要求“新对话第一条用户消息提交后立即插入固定清单”，而当前实现是“session configured 阶段先渲染欢迎链，再处理首条用户消息”；另外，本轮也未见 app-server 侧存在独立的首条用户消息后固定清单静态入口证据。
- 是否建议修复：是，优先级高。

### 3. `/responses` 除 `401` 外统一自动重试；`401` 先走认证恢复

- 状态：已满足
- 证据位置：`codex-rs/codex-api/src/provider.rs:42-62`，`codex-rs/core/src/client.rs:272-303,1292-1335,1648-1662`，`codex-rs/core/src/codex.rs:6588-6594`
- 静态判断：`responses_http_status_is_retryable(...)` 明确写成“除 `401` 外统一重试”；request 层和 stream/websocket 路径都先尝试 unauthorized recovery，再决定是否继续普通 retry。
- 缺口描述：本轮静态阅读未见与冻结口径冲突的分支。
- 是否建议修复：否。

### 4. 重试中间态只更新状态，不写入历史；保留重连详情与 metrics

- 状态：部分满足
- 证据位置：`codex-rs/core/src/codex.rs:6590-6624,7321-7378`，`codex-rs/codex-api/src/telemetry.rs:70-133`，`codex-rs/otel/src/events/session_telemetry.rs:398-425,463-481,598-669`，`codex-rs/tui/src/chatwidget.rs:4097-4105,7029-7037`，`codex-rs/tui/src/chatwidget/tests.rs:12879-12948,13023-13032,13100-13103`
- 静态判断：retry 中间态当前通过 `EventMsg::StreamError(StreamErrorEvent { ... additional_details: Some(details) })` 发出，`Reconnecting... N` 与详情字段保留；request/websocket telemetry 仍保留 metrics 路径，并把 `/responses` 主链中间态 log/trace 做了 suppress。TUI 范围内还可以进一步静态确认：`StreamError` 事件只更新状态，现有测试也明确断言这些 retry 中间态不会插入 history。
- 缺口描述：TUI 路径上的“不写历史”已经有较强静态证据，但 remote/app-server 可见历史链本轮未单独做等强度静态证明；因此总体仍保持“部分满足”，不写成全路径绝对已满足。
- 是否建议修复：暂不直接修复代码，先把该项作为静态高风险点保留；优先级中。

### 5. 单次重试等待上限保持 `10s`，不改变现有 retry budget 语义

- 状态：已满足
- 证据位置：`codex-rs/core/src/util.rs:15,206-222`，`codex-rs/codex-client/src/retry.rs:8,54-83,123-124`，`codex-rs/codex-api/src/provider.rs:107`
- 静态判断：stream 侧与 request 侧都仍把单次 backoff clamp 到 `Duration::from_secs(10)`；请求层还保留了现有 `max_attempts` / 端点例外入口，没有看到“因为扩大 `/responses` 错误分类而重写 budget 语义”的静态迹象。
- 缺口描述：本轮未见 retry budget 语义回归证据。
- 是否建议修复：否。

### 6. 保留 `providerRuntime refresh / refreshAllLoaded`，且只热刷新 `base_url` 与 `experimental_bearer_token`

- 状态：已满足
- 证据位置：`codex-rs/core/src/codex.rs:4271-4303,4307-4341`
- 静态判断：`apply_provider_runtime_refresh(...)` 与 `resolve_provider_runtime_refresh(...)` 当前只读写 `base_url`、`experimental_bearer_token` 两个字段，并继续走 model client runtime refresh。
- 缺口描述：本轮静态阅读未见其他 provider 字段被混入 runtime 热刷新。
- 是否建议修复：否。

### 7. `[agents.*].config_file` 继续按用户 `config.toml` 相对解析；Windows tray / app-server / named pipe 批量刷新继续可用

- 状态：已满足
- 证据位置：`codex-rs/core/src/codex.rs:4333-4341`，`codex-rs/app-server/src/windows_control.rs:167-221`，`scripts/windows_app_server_refresh_tray.py:363-374`
- 静态判断：refresh 重读配置前继续调用 `resolve_relative_paths_in_config_toml(...)`，基于 user `config.toml` 目录做相对路径归一化；Windows 侧 named pipe 控制面与 tray 批量刷新脚本都仍在。
- 缺口描述：本轮静态阅读未发现 `agents` 相对路径修复被回退，也未发现 Windows bulk refresh 链路被删改。
- 是否建议修复：否。

### 8. 历史与 resume 默认支持跨 provider，且继续保留 `model_provider`

- 状态：已满足
- 证据位置：`codex-rs/tui/src/lib.rs:566-585,1802-1848`，`codex-rs/exec/src/lib.rs:865-874,1240-1245,1864-1911`，`codex-rs/tui/src/resume_picker.rs:1131-1141,1933-1945`，`codex-rs/tui/src/app_server_session.rs:135-139,1144-1159`，`codex-rs/app-server/tests/suite/v2/thread_list.rs:490-507,536-553`
- 静态判断：TUI 的 `LatestSessionProviderFilter::Any` 与 exec 的 `resume_lookup_model_providers(...) -> None` 保持跨 provider 发现；resume picker 继续把 remote session 的 provider filter 置为 `Any`；`app_server_session` 继续让 embedded 保留当前 provider、remote 不注入本地 `model_provider`；`thread/list` 相关现有测试也继续证明空 provider filter 时会返回跨 provider 线程且保留 `model_provider` 字段。
- 缺口描述：本轮静态阅读未见 `model_provider` 丢失。
- 是否建议修复：否。

### 9. `gpt-5.4` 的 `/responses` 请求继续强制 `service_tier=priority`

- 状态：已满足
- 证据位置：`codex-rs/core/src/client.rs:998-1003`
- 静态判断：请求体构造出口在 `model_info.slug == "gpt-5.4"` 时直接强制序列化 `Some("priority")`。
- 缺口描述：默认兜底逻辑仍在，但它目前没有被第 11 项要求的顶层配置开关控制。
- 是否建议修复：暂不就本项单独修复；与第 11 项合并处理，优先级高。

### 10. Windows app 与 TUI 默认日志继续降噪；显式设置 `RUST_LOG` 时再恢复详细日志

- 状态：已满足
- 证据位置：`codex-rs/app-server/src/lib.rs:91-92,335-339`，`codex-rs/tui/src/lib.rs:74-75,601-605,843-859`
- 静态判断：app-server 与 TUI 都在未显式设置 `RUST_LOG` 时走默认 quiet filter，设置后回到 `EnvFilter::from_default_env()`。
- 缺口描述：本轮静态阅读未见日志默认口径回退。
- 是否建议修复：否。

### 11. 新增顶层配置 `force_gpt54_priority_fallback`

- 状态：未满足
- 证据位置：全仓库搜索 `force_gpt54_priority_fallback` 仅命中文档 `docs/local1-custom-feature-checklist-2026-03-28.md:93-95`；`codex-rs/core/src/config/mod.rs:217-236,1079-1098` 的 `Config` / `ConfigToml` 结构中也没有该字段；`codex-rs/core/src/client.rs:998-1003` 仍对 `gpt-5.4` 无条件强制 `priority`；`codex-rs/core/src/client_tests.rs:137-145` 现有测试继续锁定这种无条件行为。
- 静态判断：当前不只是“搜索不到字段”，而是“配置解析层缺失 + 请求消费层缺失 + 现有测试仍锁死无条件 priority”。因此既没有顶层 `config.toml` 解析入口，也没有默认值处理、显式 `false` 关闭逻辑，`[profiles.*]` 不支持这条文档口径也没有任何代码级支撑。
- 缺口描述：这项能力目前只冻结在文档里，未进入任何运行时代码。
- 是否建议修复：是，优先级高。

### 12. 将“首次对话清单与 gpt54 优先级开关”功能写入基线 checklist 归档

- 状态：已满足
- 证据位置：`docs/local1-custom-feature-checklist-2026-03-28.md:89-95`
- 静态判断：基线 checklist 已追加 `2026-04-10 归档补充：首次对话清单与 gpt-5.4 priority 开关`，文档归档载体存在且口径与当前 TASK 基本一致。
- 缺口描述：归档文档已回写，但第 2 项与第 11 项的代码落地仍然缺失；不能把“归档已写”误当成“实现已完成”。
- 是否建议修复：文档层否；代码层仍需跟进第 2 项与第 11 项。

## Subagent 执行期严格复核问题清单与修改建议

### 总体结论

- 以下结论仅基于当前源码、现有 diff 和现有测试文件的静态阅读，未编译、未运行测试、未修改任何文件。
- 主 agent 的 12 项静态结论总体方向正确；目前没有新增条目需要从“已满足”直接降级为“未满足”。
- 其中第 2 项未满足、第 11 项未满足、第 4 项部分满足的判断可以保留。
- 需要调整的是证据链和表述边界，而不是整体方向。第 2 项应明确写成“当前实现时机与冻结口径相反”的阻塞项；第 8、1、11 项应补强证据位置；第 4 项应拆开“已静态证实范围”和“未单独证实范围”。

### 问题清单

1. 严重级别：高；对应 TASK 项：2；问题：当前“未满足”结论方向正确，但阻塞点描述不够硬。原因/证据不足点：`chatwidget.rs` 在 `SessionConfigured` 阶段就调用 `new_session_info(...)`，随后才继续处理 `initial_user_message`；`history_cell.rs` 的 `is_first_event` 分支仍直接渲染 `/init /status /permissions /model /review` 欢迎清单，现有测试也仍在锁定这条首事件欢迎链。建议：明确写成“当前实现仍是 SessionConfigured 阶段欢迎链，和‘首条用户消息提交后插入固定清单’的冻结口径直接冲突”；同时单独说明 app-server 路径未见独立静态证据覆盖。
2. 严重级别：中；对应 TASK 项：8；问题：第 8 项判为“已满足”可以接受，但原证据位置不足以单独撑满“跨 provider 发现 + 保留 `model_provider` + local/embedded/remote 语义不变”。原因/证据不足点：原结论主要引用了 `tui/src/lib.rs` 与 `exec/src/lib.rs`，但还需要 `resume_picker.rs`、`app_server_session.rs` 与 `thread_list.rs` 测试一起补强 picker / remote / thread-list 三层证据。建议：补齐这些证据后再保留“已满足”。
3. 严重级别：中；对应 TASK 项：11；问题：第 11 项“未满足”的判断是对的，但不能只靠“全仓库搜索 0 命中”作为完整代码级证明。原因/证据不足点：`Config` / `ConfigToml` 都没有该字段，请求构造口仍无条件强制 `priority`，现有测试也在锁定这种无条件行为。建议：把这项说明升级为“配置解析层缺失 + 请求消费层缺失 + 现有测试仍锁死无条件 priority”。
4. 严重级别：低；对应 TASK 项：1；问题：第 1 项“全链路版本显示已满足”结论基本成立，但原证据位置不足以支撑“历史单元、升级提示及其他用户可见版本展示”的全链路措辞。原因/证据不足点：还需要把 `history_cell.rs` 的升级历史文本和 `update_prompt.rs` 的更新弹窗当前版本链一起纳入证据。建议：补齐这两处证据即可，不需要改判。
5. 严重级别：低；对应 TASK 项：4；问题：第 4 项当前判成“部分满足”并不算过满，反而略保守，但原表述没有拆开“已证范围”和“未证范围”。原因/证据不足点：TUI 范围内其实已有更强静态证据，`StreamError` 处理只更新状态，现有测试也明确断言 retry 中间态不会插入 history。真正未单独证实的是 remote/app-server 可见历史链。建议：若继续保留“部分满足”，应显式写成“TUI 静态已证实，remote/app-server 可见链未单独证实”。

## 主Agent执行期处理结论与修复决策

1. 采纳：subagent 对第 2 项的意见成立。我已把该项从“欢迎提示链未替换”进一步收紧为“当前实现时机与冻结口径相反”，并补入 `SessionConfigured` 先于 `initial_user_message` 的静态证据；app-server 覆盖范围也改成了“未见独立静态证据”。
2. 采纳：subagent 对第 8 项的意见成立。我已补入 `resume_picker.rs`、`app_server_session.rs` 与 `thread_list.rs` 测试三层证据，当前“已满足”结论可以保留，但现在证据链更完整。
3. 采纳：subagent 对第 11 项的意见成立。我已把“源码 0 命中”的弱证据升级为“配置解析层缺失 + 请求消费层缺失 + 现有测试仍锁死无条件 priority”的强证据链。
4. 采纳：subagent 对第 1 项的意见成立。我已补入 `history_cell.rs` 与 `update_prompt.rs`，使“全链路版本显示”不再只有主展示链证据。
5. 采纳：subagent 对第 4 项的意见成立。我保留“部分满足”结论，但已明确区分“TUI 静态已证实”和“remote/app-server 可见历史链未单独证实”。
6. 最终执行结论：当前改动部分满足，建议修复以下项后再收口：
   - 第 2 项：首次对话固定清单目前仍是 `SessionConfigured` 阶段欢迎链，与冻结口径直接冲突。
   - 第 11 项：`force_gpt54_priority_fallback` 仍只存在于文档，没有任何配置解析或运行时消费实现。
   - 第 4 项：当前可作为静态高风险保留项；若进入下一轮修复，建议顺带补强 remote/app-server 可见历史链的静态证据或后续运行验证。
7. 当前不建议直接收口为“已满足 TASK”；更准确的收口口径应为：`当前改动部分满足，建议优先修复第 2 项与第 11 项。`
