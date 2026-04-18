# local1 清单同步到 rust-v0.121.0_TASK_2026-04-17

## Context

- 本文是一份“direct sync 执行依据 + 审计轨迹”的 TASK，目标是把 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 直接对齐到官方 `rust-v0.121.0` 基线，同时保住现有 local1 定制内容不变，不让上游 121 冲掉私有语义。
- 本轮主产物不是只有这一份 TASK 文档；主回写对象是现有 checklist [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)，而本文同时承担执行依据、过程记录和审计轨迹。
- 当前仓库处于大量未提交改动状态；本 TASK 以“当前 local1 checklist + 当前仓库静态真值 + 官方 `rust-v0.121.0` 发布真值”为执行基线，不清理、不回滚、不扩写无关脏改。
- 用户显式点名了 `$codexflow-temp-output-writer`。当前会话没有通过已注册技能列表挂载该 skill，因此本 TASK 只沿用其输出目录规则落盘到 `.codexflow/临时/local1清单同步121_2026-04-17/`，实施仍按普通 Markdown 文档工作流完成，不等待该 skill 的实际挂载或自动化能力。

## Goal

- 直接把 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 智能同步到官方 `rust-v0.121.0` 基线，并在同步时保住 local1 私有能力定义不变，不让上游 121 覆盖本地定制语义。
- 对当前 checklist 中的 `F1-F15` 和归档 `A1/A2` 全量覆盖，逐项标记为 `保持不变`、`仅文案刷新`、`需要新增121风险提示` 三态之一，并把这些结论正式回写到 checklist 本体。
- 把官方 `rust-v0.121.0` 中与 local1 定制直接相关的变化和明确不纳入本轮的变化拆开写清，避免把“121 官方新增能力”误写进 local1 私有基线。
- 让本文同时承担执行依据与回写审计轨迹，并在 checklist 回写完成后继续承接 subagent 严格复核、主 agent 审核处理、最小 release 编译，以及 local 清单全量验证。

## Upstream Baseline

- 本轮“121”只指官方正式 release `rust-v0.121.0`。
- 官方发布日期固定为 `2026-04-15`。
- 官方 release 真值页固定为：
  `https://github.com/openai/codex/releases/tag/rust-v0.121.0`
- 官方 compare 真值页固定为：
  `https://github.com/openai/codex/compare/rust-v0.120.0...rust-v0.121.0`
- 官方 `121` 边界真值只认上述两个官方页面：
  1. `rust-v0.121.0` release 页面
  2. `rust-v0.120.0...rust-v0.121.0` compare 页面
- local1 同步与复核时，repo 侧静态真值允许使用：
  1. 当前 checklist [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
  2. 当前工作区相关源码、脚本、文档与现有静态测试/快照文件
- 明确排除 `2026-04-16` 发布的 `rust-v0.122.0-alpha.3` 以及其后的 `main` 变化；这些内容不得被混入“121 同步”定义。

## 审查边界

- 本轮总任务以 checklist 本体同步和任务链文档回写为主，并继续执行两轮 reviewer 复核、最小 release 编译与 release-only 验证。
- 本轮不以源码功能扩展为目标，不改与 checklist 同步无关的源码、测试、配置、schema、snapshot、脚本行为；若后续验证暴露实现层问题，只做恢复已冻结 local1 行为所需的最小修复，不扩成新的私有功能。
- 本轮后续允许执行最小 release 编译、release-only smoke 和 local 清单全量验证；明确禁止 debug build，不得把 debug 编译结果混入正式结论。
- 本轮不把 `rust-v0.121.0` 中的 marketplace、memory、secure devcontainer、通用 MCP/app-server 新能力自动纳入 local1 定制范围，除非它们直接冲击已冻结的 local1 六条主线或归档 `A1/A2`。
- 本轮不执行任何分支切换、worktree 创建或仓库状态变更操作。
- 本轮的 repo 侧对照真值允许直接使用当前工作区相关源码、脚本、文档与现有静态测试/快照文件；其中凡是条目自身把 tests/snapshots 作为静态真值面，例如 checklist `F4`，主 agent 与 reviewer 都必须连同相关测试与快照一起复读。
- 若某项判断当前阶段只基于官方 release/compare 文本和静态代码路径推断，而没有运行时验证，必须在文档里按“静态核对结论”表述，不能伪装成已完成实现验证。

## 执行状态

- [x] 已确认正式产物输出目录与命名规则
- [x] 已确认当前 local1 checklist 真值
- [x] 已确认官方 `rust-v0.121.0` release/compare 真值
- [x] 已按用户最新口径重写 TASK 文档为执行版框架
- [x] 已回写 checklist 本体首轮 121 同步内容
- [x] 已完成第一轮 subagent 严格复核
- [x] 已完成第一轮主 agent 审核处理结果回写
- [x] 已完成最小 release 编译
- [x] 已完成 local 清单全量验证并生成条目级状态
- [x] 已完成不可测项 reviewer 代码复核
- [x] 已完成 UTF-8 复读与最终收口（WinForms 提醒收尾因当前环境缺少 `pwsh.exe` 未执行，已登记为环境阻塞）

## 执行记录

- 2026-04-17 03:37:40：已确认本轮正式产物必须遵循 `$codexflow-temp-output-writer` 的输出目录规则，写入 `.codexflow/临时/local1清单同步121_2026-04-17/`，不写 `docs/`；当前会话未通过已注册技能列表挂载该 skill，因此实施按普通 Markdown 文档工作流完成。
- 2026-04-17 03:37:40：已复读 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)，确认当前冻结范围为六条主线 `F1-F15` 与归档 `A1/A2`。
- 2026-04-17 03:37:40：已核对官方 `rust-v0.121.0` 发布页，确认发布日期为 `2026-04-15`，且官方 release note 明确包含 marketplace、memory、MCP/app-server 扩展、secure devcontainer、Windows `resume --last` / `thread/list` cwd/session 修复等大项。
- 2026-04-17 03:37:40：已核对官方 compare `rust-v0.120.0...rust-v0.121.0`，确认 [codex_message_processor.rs](I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) 在官方 121 中发生大改，compare 统计为 `+1020/-308`，因此该文件必须作为 F11 的重点静态审计热点。
- 2026-04-17 03:37:40：已确认官方 121 明确提到 `Fixed Windows cwd/session matching so resume --last and thread/list work when paths use verbatim prefixes (#17414)`，这会直接影响“跨 Provider 历史发现链”的文案风险口径。
- 2026-04-17 03:37:40：已确认官方 121 同时引入 `codex-thread-store` 与 local thread listing 重构（release notes: `#17659`, `#17824`），因此 F11 除 Windows 修复外，还需要把本地线程列举实现迁移的结构性变化记为专项风险。
- 2026-04-17 03:37:40：已确认官方 121 同时新增 marketplace、memory、MCP Apps/tool metadata、turn item injection、filesystem metadata、external-agent migration、websocket token-hash API 等能力；本 TASK 明确将其作为排除项记录，不把它们扩写成 local1 私有能力。
- 2026-04-17 05:01:37：用户已明确本轮真实目标是直接把 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 同步到官方 `rust-v0.121.0` 基线，同时保住 local1 私有内容不被上游 121 冲掉；本文据此承接 checklist 回写、审计轨迹与后续验证。
- 2026-04-17 05:01:37：已完成 TASK 正文目标纠偏，统一改写为“direct sync 执行依据 + 审计轨迹”口径。
- 2026-04-17 05:22:55：已回写 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 的 `121 同步基线（2026-04-17）` 与 `2026-04-17 同步判定矩阵`，并保留现有 `F1-F15`、`A1/A2` 主表不被推倒重写。
- 2026-04-17 05:22:55：已把本文升级成执行版框架，补入 build recipe、validation plan、raw evidence 产物路径，并为后续 reviewer、release-only 验证与最终证据矩阵预留收口位置。
- 2026-04-17 07:02:56：第一轮 reviewer 已完成严格复核，并输出 [local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md)；主 agent 正按 findings 重写正文后再回填审计附录。
- 2026-04-17 08:15:44：同一条 release 构建命令已完成产物更新，[codex.exe](I:/vscodeProject/codex/codex-rs/target/release/2026-04-17_local1_checklist/release/codex.exe) 的 `LastWriteTime` 已更新到 `2026-04-17 08:15:44`，说明版本链修复后的 release 产物已生成。
- 2026-04-17 08:16:23：已用同一条 `cargo build -p codex-cli --bin codex --release` 做增量复跑以补齐正式 `exit_code=0` 证据，并将结果写入 [release_build.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_build.log)。
- 2026-04-17 08:16:41：已完成固定 release-only smoke；`codex.exe --version` 确认输出 `codex-cli 0.121.0-local1`，`resume --help` 与 `fork --help` 均保留 `--all` / cwd 相关说明，结果已写入 [release_runtime_smoke.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_runtime_smoke.log)。
- 2026-04-17 08:51:16：第二轮 reviewer 已完成不可测项代码复核，并输出 [local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md)；结论为 `1` 个 finding，聚焦 `F11` 的 provider provenance 阻塞。
- 2026-04-17 08:56:09：已完成条目级证据汇总并生成 [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；`F1-F15`、`A1`、`A2` 均已有最终状态，其中 `F11` 为 `Blocked`，其余条目按 `Passed Mixed` / `Passed Static` 收口。
- 2026-04-17 09:08:57：已对 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)、[local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)、两份 reviewer 报告与三份日志执行 UTF-8 无 BOM / 替代字符复读；结果为 `Utf8Bom=False`、`HasReplacementChar=False`。
- 2026-04-17 09:08:57：已尝试按 AGENTS 要求使用 `Start-Process pwsh.exe ...` 发 WinForms 非阻塞提醒，但当前环境未发现 `pwsh.exe` 可执行文件；该收尾项登记为环境阻塞，不影响本轮 checklist、TASK、release build 与验证结论。

## 关键审计文件与官方121热点

### 现有 local1 冻结真值

- 基线文件：[local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
- 六条主线：
  - `local1` 显示链
  - `Responses` 主链重试增强链
  - Provider runtime 热刷新链
  - 跨 Provider 历史发现链
  - `gpt-5.4 priority` 请求层兜底链
  - Windows/TUI 默认日志降噪链
- 归档项：
  - `A1` 首次对话统一首段清单普通文本化展示
  - `A2` `force_gpt54_priority_fallback` 顶层开关

### 官方121直接相关热点

- [codex_message_processor.rs](I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs)
  - compare 中实改 `+1020/-308`
  - 与 `thread/list`、线程发现链、app-server 线程列举口径直接相关
- 官方 release 明确相关的变更项：
  - `#17414`：Windows `resume --last` / `thread/list` cwd/session 匹配修复
  - `#17659`, `#17824`：引入 `codex-thread-store` 并把 local thread listing 迁到新接口
  - `#17381`, `#17486`, `#17521`, `#17557`：Guardian timeout 可见历史项与语义区分，靠近 TUI 历史/消息展示面
  - `#17550`, `#17336`：TUI prompt history 与 slash command recall 改进，靠近输入/历史体验面

### 本轮明确排除但必须写清的官方121新增能力

- marketplace add 与 marketplace source 支持
- memory mode / reset / deletion / cleanup
- MCP Apps tool calls、parallel-call opt-in、sandbox-state metadata
- realtime output modality、transcript completion、turn item injection
- secure devcontainer / bubblewrap / sandbox 文档
- external-agent migration、filesystem metadata、websocket token-hash API

### 本轮需持续盯住的本地文件路径

- 说明：下列路径分成两类。`codex_message_processor.rs` 是我已直接核实到的官方 `120 -> 121` compare 实改热点；其余路径主要是 local1 冻结语义的本地审计路径，不等于官方 compare 的穷尽改动清单。
- [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
- [codex_message_processor.rs](I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs)
- [lib.rs](I:/vscodeProject/codex/codex-rs/exec/src/lib.rs)
- [lib.rs](I:/vscodeProject/codex/codex-rs/tui/src/lib.rs)
- [resume_picker.rs](I:/vscodeProject/codex/codex-rs/tui/src/resume_picker.rs)
- [app_server_session.rs](I:/vscodeProject/codex/codex-rs/tui/src/app_server_session.rs)
- [client.rs](I:/vscodeProject/codex/codex-rs/core/src/client.rs)
- [provider.rs](I:/vscodeProject/codex/codex-rs/codex-api/src/provider.rs)
- [windows_app_server_refresh_tray.py](I:/vscodeProject/codex/scripts/windows_app_server_refresh_tray.py)

## Checklist

| ID | 条目 | 本轮判定 | 121 影响判断 | TASK 处理口径 |
|---|---|---|---|---|
| F1 | `local1` 显示版本号 | 保持不变 | 官方 121 未把该本地版本后缀能力纳入官方语义；当前任务只需继续把它视为 local1 私有显示链。 | 本轮同步 checklist 时保留现有定义，不扩大到官方 release/version announcement 逻辑。 |
| F2 | 卡片与状态区融入 `local1` | 保持不变 | 官方 121 有 TUI 新功能，但没有把 local1 版本展示并入官方状态区语义。 | 本轮同步 checklist 时只复核现有 `CODEX_CLI_DISPLAY_VERSION` 口径是否仍统一，不新增功能定义。 |
| F3 | 历史单元与升级提示融入 `local1` | 仅文案刷新 | 官方 121 引入 Guardian timeout 可见历史项与 TUI 历史改动，容易让“历史可见文本变化”与 local1 版本提示混淆。 | 本轮同步 checklist 时补一句：官方 121 的历史项新增不改变 local1 版本文本链定义。 |
| F4 | `local1` 的测试与快照基线 | 保持不变 | 官方 121 未改变“local1 必须有快照/断言守护”的要求。 | 后续继续把快照和断言当作该私有能力的守护面，不把 121 的无关 UI 改动写成新增要求。 |
| F5 | 请求重试范围增强 | 保持不变 | 官方 121 发布摘要未直接声明覆盖 local1 的 `/responses` 全 HTTP 普通重试私有口径。 | 本轮同步 checklist 时继续按现有 F5 口径静态复核，不把 121 的其他网络或 sandbox 修复混写进此项。 |
| F6 | 流式重试与 UI 提示联动 | 仅文案刷新 | 官方 121 增加 realtime/transcript/app-server 事件与 Guardian 历史可见项，容易污染“重试中间态只走状态区、不入历史”的边界描述。 | 本轮同步 checklist 时明确：121 的新事件面不自动改变 local1 对 retry UI/history 边界的定义。 |
| F7 | 单次重试等待时间上限为 `10s` | 保持不变 | 官方 121 发布信息未提供反向证据说明该 local1 cap 已被官方吸收或废止。 | 本轮同步 checklist 时继续按 `10s` cap 做静态核对，不新增实现指令。 |
| F8 | 重试次数保持“大次数或等效无界”目标 | 保持不变 | 官方 121 无明确 release note 声明要替换现有 local1 retry budget 语义。 | 本轮同步 checklist 时保留原语义，不把 121 的其他鲁棒性修复误写成 budget 变更。 |
| F9 | 重试配置入口尽量统一 | 保持不变 | 官方 121 无直接文本表明统一 retry 分类入口语义被官方取代。 | 后续继续把统一入口作为 local1 私有要求审查。 |
| F10 | 基于 user `config.toml` 的 Provider 切换与可选 refresh | 仅文案刷新 | 官方 121 大量新增 marketplace、external-agent migration、app-server API，语义上接近 provider/runtime，但不等于 config-first tray 逻辑。 | 本轮同步 checklist 时把以下硬边界继续钉死：`source provider` 只读显式 `[model_providers.*]`；`target provider` 固定顶层 `model_provider`；apply 只复制 `base_url` 与 `experimental_bearer_token`；target/source 缺字段、语法无效或结构不支持时不落盘；无 live instance 也允许只写配置并提示“未刷新任何实例”；官方 121 的 marketplace/external-agent/app-server 能力不得改写上述口径。 |
| F11 | 历史默认不按 Provider 分割 | 需要新增121风险提示 | 官方 121 明确修复 Windows `thread/list` / `resume --last` cwd/session 匹配，并把 local thread listing 迁到新接口；这最容易冲击 F11 的文案口径。 | 必须新增专项风险提示，单独复核“保留 cwd / `--all` / `show_all` 语义”和“默认不按 provider 过滤”是否仍被正确区分；同时继续钉死 CLI 与 embedded TUI 继续旧线程仍使用当前 provider、remote TUI 不自动切到历史线程 provider。 |
| F12 | `Responses` stream / websocket 的 `401` 直接普通 retry 链 | 保持不变 | 官方 121 发布信息没有把该 local1 `401` 私有处理口径吸纳为通用官方策略。 | 后续继续把它当 local1 私有回归项，不把其他认证或网络修复混写进来。 |
| F13 | `gpt-5.4` 默认 priority 请求层兜底与顶层关闭开关 | 保持不变 | 官方 121 虽有与模型、rate-limit、plan decoding 相关条目，但没有声明替代 local1 `force_gpt54_priority_fallback` 顶层开关。 | 本轮同步 checklist 时继续按当前 checklist 定义审查，并把以下硬边界继续写死：仅顶层 `config.toml` 字段生效；省略或显式 `true` 等价；显式 `false` 时同时关闭 `priority` 与 `Fast` 透传但保留 `Flex`；任何 `[profiles.*].force_gpt54_priority_fallback` 都不得生效。 |
| F14 | Windows app / app-server 默认日志降噪 | 保持不变 | 官方 121 引入大量 app-server 新能力，但 release note 未声明默认日志过滤重新扩噪。 | 后续继续保留“默认 warn、显式 `RUST_LOG` 可覆盖”的本地口径。 |
| F15 | TUI 默认日志降噪 | 保持不变 | 官方 121 有 TUI 新功能与 CLI update announcement，但不等于恢复高噪音日志默认值。 | 本轮同步 checklist 时继续按现有 local1 降噪口径审查，不把 TUI 新功能写成日志策略变化。 |
| A1 | 首次对话统一首段清单普通文本化展示 | 仅文案刷新 | 官方 121 新增 TUI 历史、Guardian 可见项、realtime transcript 事件，容易让“主消息首段文本链”与其他可见输出链混淆。 | 本轮同步 checklist 时把以下硬边界继续钉死：仅 brand-new / `Clear` 新线程；仅单个纯文本 `你好` 触发；`resume` / `continue` / `fork` / 历史线程 / MCP 非 `你好` / subagent 会话都不触发；同线程只出现一次；`AgentMessageDelta` / `ItemCompleted(ThreadItem::AgentMessage)` / legacy `AgentMessage` 三条消费链前缀必须一致；121 的新消息/历史/事件面不得改写上述定义。 |
| A2 | `force_gpt54_priority_fallback` 顶层开关 | 保持不变 | 官方 121 未声明 profile 级覆盖或默认关闭该 local1 顶层开关。 | 本轮同步 checklist 时继续按顶层-only 精确口径复核：仅顶层 `config.toml` 可写；默认等价 `true`；显式 `false` 时同时关闭 `priority` 与 `Fast` 透传但保留 `Flex`；任何 `[profiles.*]` 同名字段都不得覆盖。 |

### F11 专项风险复核

- 官方 121 明确写了 Windows `resume --last` / `thread/list` cwd/session 匹配修复（`#17414`），这会改变“历史能否被找到”的行为感知，但不等于取消 local1 既有的 cwd / `--all` / `show_all` 语义。
- 官方 121 还引入了 `codex-thread-store` 与 local thread listing 重构（`#17659`, `#17824`），说明本地线程列举链路发生了结构迁移，不能只看 release note 文案，必须把 [codex_message_processor.rs](I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) 当成高风险审计点。
- `#17414` 只修 Windows 上 `resume --last` / `thread/list` 的 cwd/session matching，不改变 provider 规则；本轮同步 checklist 时不得把该修复误写成“resume 跟随历史 provider”或“remote TUI 也切换 provider”。
- 本轮同步 checklist 时，F11 必须同时保留两层边界：
  - 默认不按 provider 过滤
  - 仍保留 cwd / `--all` / `show_all` 语义
- 本轮同步 checklist 时，必须继续钉死两条 provider 边界：
  - CLI 与 embedded TUI 继续旧线程时仍使用当前 provider
  - remote TUI 仍不自动切到历史线程记录的 provider
- 本轮同步 checklist 时，F11 仍需保留 `thread/list` 返回项保留 `model_provider` 字段这一验收口径，避免 thread listing 重构后 provider 身份信息被遗漏。

## Acceptance

- [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 已完成首轮 `121` 同步回写，且新增 `121 同步基线（2026-04-17）` 与 `2026-04-17 同步判定矩阵`，没有丢失既有 `F1-F15`、`A1/A2` 主表。
- 本文明确引用 `2026-04-15` 的官方 `rust-v0.121.0`，并明确排除 `2026-04-16` 的 `rust-v0.122.0-alpha.3` 及其后的 `main`。
- 本文对 `F1-F15` 与归档 `A1/A2` 全量覆盖，没有把“功能不变”偷换成“无需逐项复核”。
- 本文把官方 121 的变化拆成“与 local1 定制直接相关的热点”和“明确不纳入本轮的新增能力”两类。
- 本文已把 `F10`、`F11`、`F13`、`F14`、`F15`、`A1`、`A2` 的高漂移硬边界重新写死，不要求执行者回读旧 checklist 才能知道关键约束。
- 本文明确写出：当前会话没有通过已注册技能列表挂载 `$codexflow-temp-output-writer`，本轮只是沿用其输出目录规则落盘。
- 本文已写死最小 release 构建命令、release-only smoke 范围、三份 evidence 日志以及两份 reviewer 审计 md 的固定路径。
- 本文已把官方 `121` 真值与当前仓库静态真值拆开定义，避免执行者误把 repo truth 排除在外。
- 本文末尾保留 `Subagent严格复核附录` 与 `主Agent审核处理结果` 两个章节，且它们是全文最后两个章节。
- 本文明确后续还需执行第一轮 reviewer 严格复核、主 agent 审核处理、最小 release 编译、local 清单全量验证、第二轮 reviewer 不可测项代码复核；其中只允许最小 release 编译，不允许 debug build。
- 本文不包含任何与本任务无关的代码实现扩展指令、分支/worktree 操作指令或回滚指令。

## Notes

- 本轮没有 public API、interface、schema、type 变更计划；这是 checklist/task 同步、必要时最小行为修复、以及 release-only 验证任务，不是功能扩展任务。
- 本轮主回写对象是 [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)；本 TASK 同时承担执行依据、过程记录和审计轨迹。
- 当前工作区已有大量未提交改动；执行时必须基于现状合并，不回滚、不重置、不覆盖无关用户修改。
- 当前会话没有通过系统技能列表启用 `$codexflow-temp-output-writer`；本 TASK 仅依据用户提供的 skill 文本沿用其输出目录约定，不等待实际 skill 自动化能力。
- `本轮需持续盯住的本地文件路径` 不是官方 compare 全量改动清单，而是“官方 121 直接热点 + local1 冻结语义观察面”的组合列表。
- 若 release-only 验证暴露真实实现漂移，本轮允许做恢复既有 local1 行为所需的最小代码修复，但不得扩成新的私有功能，不得把 `121` 无关能力顺手并入。
- 文本文件必须保持 UTF-8（无 BOM），最终需要复读确认无乱码。

## Build Recipe

- 最小 release 构建的唯一主命令固定为：

```powershell
Set-Location "I:\vscodeProject\codex\codex-rs"
$env:CARGO_TARGET_DIR = "I:\vscodeProject\codex\codex-rs\target\release\2026-04-17_local1_checklist"
cargo build -p codex-cli --bin codex --release
```

- 若命中 Windows 跨盘 `v8` symlink 权限问题，只允许对同一条命令补充：

```powershell
$env:CARGO_HOME = "I:\cargo-home-local1"
```

- 上述 `CARGO_HOME` 只属于 same-drive workaround，不改变主命令体，不得因此切换到 debug build、`cargo test` 或 `cargo run`。
- release 产物固定记录：
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-17_local1_checklist\release\codex.exe`
  - `I:\vscodeProject\codex\codex-rs\target\release\2026-04-17_local1_checklist\release\codex.pdb`

## Validation Plan

- 固定 release-only smoke 命令：
  - `codex.exe --version`
  - `codex.exe --help`
  - `codex.exe app-server --help`
  - `codex.exe resume --help`
  - `codex.exe fork --help`
- 条目最终状态只允许使用：
  - `Passed Runtime`
  - `Passed Mixed`
  - `Passed Static`
  - `Blocked`
  - `Failed`
- `F1`：release runtime + 静态 spot check。
- `F2-F4`：静态审计 + 第一轮 reviewer / 第二轮 reviewer 代码复核；不引入 debug/test build。
- `F5-F9`、`F12`、`F13`、`A2`：静态审计 + reviewer 代码复核；若 release 证据自然覆盖，再升级为 `Passed Mixed`。
- `F10`：静态审计 + reviewer 代码复核；`app-server --help` 只作辅助，不把 help 面当 refresh 语义主证据。
- `F11`：release smoke 证明 CLI/help 面未丢 `cwd` / `--all` 相关能力，配合静态审计与 reviewer 代码复核收口。
- `F14-F15`：静态审计为主，必要时辅以 release smoke。
- `A1`：默认静态审计 + reviewer 代码复核；只有在不引入 debug/test build 的前提下拿到自然路径 release 证据，才允许升为 runtime pass。
- 第一轮 reviewer 的输出路径固定为：
  - [local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md)
- 第二轮 reviewer 的输出路径固定为：
  - [local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md)

## Raw Evidence

- `release_build.log`
  路径：[release_build.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_build.log)
- `release_runtime_smoke.log`
  路径：[release_runtime_smoke.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_runtime_smoke.log)
- `targeted_validation.log`
  路径：[targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)
- `第二轮 reviewer 报告`
  路径：[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md)
- 三份日志都必须带固定字段：
  - `started_at`
  - `finished_at`
  - `cwd`
  - `command`
  - `key_env`
  - `exit_code`
  - `stdout_excerpt`
  - `stderr_excerpt`
  - `related_items`
  - `evidence_paths`
- `targeted_validation.log` 额外必须写：
  - `validation_case`
  - `status`
  - `source_files_checked`
  - 若为 `Blocked` 或 `Passed Static`，需写清阻塞原因或静态核对结论
- 如需临时快照或中间审计文件，统一放到：
  - `I:\vscodeProject\codex\tmp\agent-snapshots`
  - 使用后清理，不放在项目根目录

## 最终证据矩阵

- 本节是 `F1-F15`、`A1`、`A2` 的唯一 item-by-item 收口位置。
- 最终状态只允许使用：`Passed Runtime`、`Passed Mixed`、`Passed Static`、`Blocked`、`Failed`。
- 固定字段为：`ID | 最终状态 | 证据类型 | 证据路径 | reviewer结论 | 主Agent处理结果 | 备注`

| ID | 最终状态 | 证据类型 | 证据路径 | reviewer结论 | 主Agent处理结果 | 备注 |
|---|---|---|---|---|---|---|
| `F1` | `Passed Mixed` | `release runtime + static spot check` | [release_build.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_build.log)；[release_runtime_smoke.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_runtime_smoke.log)；[targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log) | 第二轮 reviewer 建议 `Passed Mixed` | `采纳`；维持通过 | 版本链已从 `0.120.0-local1` 修正为 `0.121.0-local1` |
| `F2` | `Passed Static` | `static code audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | TUI 版本展示链未漂移 |
| `F3` | `Passed Static` | `static code audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | 历史单元与升级提示仍引用 local1 显示常量 |
| `F4` | `Passed Static` | `static tests/snapshots audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | 已按 F4 例外边界纳入快照与断言守护 |
| `F5` | `Passed Static` | `static retry-classifier audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `/responses` 普通重试口径保留，非 `/responses` whitelist 保留 |
| `F6` | `Passed Static` | `static telemetry/UI audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | retry 中间态只更新状态，不入历史 |
| `F7` | `Passed Static` | `static retry-delay audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | 单次等待上限仍为 `10s` |
| `F8` | `Passed Static` | `static retry-budget audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | bounded/unbounded 预算语义未被 121 改写 |
| `F9` | `Passed Static` | `static unified-entry audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | retry 分类与 route/path 透传仍沿统一主链 |
| `F10` | `Passed Static` | `static config-first audit + auxiliary help` | [release_runtime_smoke.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_runtime_smoke.log)；[targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `app-server --help` 仅作辅助，不作为 refresh 语义主证据 |
| `F11` | `Blocked` | `release help + static audit + reviewer` | [release_runtime_smoke.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/release_runtime_smoke.log)；[targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Blocked` | `采纳`；本轮不修正 | `thread/list` / rollout summary 对缺失 `model_provider` 的历史记录会静默回填当前 fallback provider，当前约束下无法无争议证明真实 provenance |
| `F12` | `Passed Static` | `static 401 retry audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `/responses` 401 仍走统一普通 retry classifier |
| `F13` | `Passed Static` | `static config/service-tier audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | 顶层 `force_gpt54_priority_fallback` 与 `Fast/Flex` 语义保持一致 |
| `F14` | `Passed Static` | `static app-server log-filter audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | 默认日志过滤仍为 `warn` |
| `F15` | `Passed Static` | `static TUI log-filter audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `codex-tui.log` 与文档口径一致 |
| `A1` | `Passed Static` | `static first-turn checklist audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `brand-new / Clear + 纯文本 你好` 与三条消费链共用前缀逻辑仍成立 |
| `A2` | `Passed Static` | `static top-level toggle audit + reviewer` | [targeted_validation.log](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/targeted_validation.log)；[local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md) | 第二轮 reviewer 建议 `Passed Static` | `采纳`；维持通过 | `[profiles.*]` 仍无覆盖入口 |

## 用户/玩家视角直观变化清单

- 本次修改无用户/玩家可直接感知的直观变化。

## Subagent严格复核附录
- 第一轮 reviewer 已输出正式复核报告：
  - [local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_严格复核_2026-04-17.md)
- reviewer 共给出 `5` 条 findings，按严重度排序如下：
  - `高`：两份文档把“允许使用的真值来源”写成只认三类页面/文档来源，但正文实际又依赖当前仓库源码与 GitHub compare 机器可读真值，导致官方 `121` 真值与 repo 静态真值边界自相矛盾。
  - `中`：checklist 的 `F11` 主表“当前代码迹象”仍把“provider 默认过滤”写成当前现状，但当前 repo truth 已变成默认跨 provider discover，仅保留 fork/current-provider fallback 与 embedded/remote 注入差异。
  - `中`：checklist 默认审查边界排除了 tests/snapshots/fixtures，但 `F4` 又把快照与断言守护定义成核心基线，导致 `F4` 默认流程无法闭环。
  - `中`：TASK 定义了状态枚举、两轮 reviewer 与三份日志，却没有为 `F1-F15/A1/A2` 的最终 item-by-item 状态提供固定归宿，缺少最终证据矩阵。
  - `低`：TASK 仍残留少量旧阶段措辞，主要出现在执行记录与附录说明，容易在检索时干扰阶段判断。
- reviewer 原报告中的每条 finding 都已包含：`严重度`、`问题描述`、`证据`、`为什么是问题`、`修改建议`、`建议落到章节`；本节只保留主 TASK 所需的审计摘要，完整证据以外部 reviewer 报告为准。
- 第二轮 reviewer 已输出正式不可测项代码复核报告：
  - [local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_不可测项代码复核_2026-04-17.md)
- 第二轮 reviewer 共给出 `1` 条 finding：
  - `中`：`F11` 的 `thread/list` / rollout summary 链路在历史元数据缺失 `model_provider` 时会静默回填当前 fallback provider，只能证明“字段存在”，不能证明“历史 provider 身份被真实保留”。
- 第二轮 reviewer 的条目级状态建议收口为：
  - `F1`：`Passed Mixed`
  - `F2-F10`、`F12-F15`、`A1`、`A2`：`Passed Static`
  - `F11`：`Blocked`

## 主Agent审核处理结果
- Finding 1：`采纳`
  - 理由：用户当前执行口径明确要求同时对照“官方 `121` 真值”和“当前仓库真值”；reviewer 指出的矛盾成立。正文已把“官方 `121` 边界真值”和“repo 侧静态真值”拆开定义，并删除了会破坏边界的一行 API 时间戳表述。
  - 回写章节：
    - [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
      `121 同步基线（2026-04-17）`
    - [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
      `当前推断范围`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `Context`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `Upstream Baseline`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `审查边界`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `Acceptance`
- Finding 2：`采纳`
  - 理由：reviewer 对 `F11` repo truth 的引用准确，原“当前代码迹象”确实落后于当前工作区现状。已改写为“默认跨 provider discover + fork/current-provider fallback + embedded/remote 注入差异 + `thread/list` 保留 `model_provider` + 官方 121 风险热点”这组现状描述。
  - 回写章节：
    - [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
      `F11 | 历史默认不按 Provider 分割`
- Finding 3：`采纳`
  - 理由：`F4` 的静态真值本来就依赖 tests/snapshots；如果默认边界把它们排除，`F4` 无法成立。已在 checklist 边界中加入例外，并在 `F4` 的验收口径里明确本条不受默认 tests/snapshot 排除规则约束。
  - 回写章节：
    - [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
      `审查边界`
    - [local1-custom-feature-checklist-2026-03-28.md](I:/vscodeProject/codex/docs/local1-custom-feature-checklist-2026-03-28.md)
      `F4 | local1 的测试与快照基线`
- Finding 4：`采纳`
  - 理由：TASK 需要一个固定的 item-by-item 收口位置，否则三份日志、两轮 reviewer 和最终状态会分散。按 reviewer 建议的两种方案中，我采用“在 TASK 正文内新增固定章节”的方式，避免再引入一个额外产物，同时不破坏文末最后两个章节约束。
  - 回写章节：
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `最终证据矩阵`
- Finding 5：`采纳`
  - 理由：旧阶段措辞不再适合执行版 TASK。已把执行记录中的历史描述改写成不带“只写 TASK 文档”“文档冻结版”标签的过去式表述，并清理了附录中的同类旧词。
  - 回写章节：
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `执行记录`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `Subagent严格复核附录`
- 第一轮 reviewer 结论收口：
  - `5/5` 条 findings 已处理，其中 `5` 条采纳、`0` 条不采纳。
  - 第一轮 reviewer 已全部闭环到正文。
- 第二轮 reviewer Finding 1：`采纳`
  - 理由：reviewer 指出的 `F11` provenance 缺口成立。当前实现不仅在 [codex_message_processor.rs](I:/vscodeProject/codex/codex-rs/app-server/src/codex_message_processor.rs) 的 `thread/list` / rollout summary 路径上对缺失 `model_provider` 使用 `fallback_provider`，在 [extract.rs](I:/vscodeProject/codex/codex-rs/state/src/extract.rs) 的 state metadata 提取链上也会把缺失 provider 默认落成 `default_provider`。在“不新增 public API / interface / schema / type、不引入 `unknown` 伪值、不扩写迁移任务”的本轮边界下，无法用一个低风险补丁无争议恢复“真实 provider provenance”。
  - 修正决定：`不修正`
  - 回写章节：
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `执行状态`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `执行记录`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `最终证据矩阵`
    - [local1清单同步到rust-v0.121.0_TASK_2026-04-17.md](I:/vscodeProject/codex/.codexflow/临时/local1清单同步121_2026-04-17/local1清单同步到rust-v0.121.0_TASK_2026-04-17.md)
      `Subagent严格复核附录`
- 第二轮 reviewer 结论收口：
  - `1/1` 条 findings 已处理，其中 `1` 条采纳、`0` 条不采纳。
  - 本轮最终未新增代码修复；`F11` 维持 `Blocked`，其余条目按 `Passed Mixed` / `Passed Static` 收口。
