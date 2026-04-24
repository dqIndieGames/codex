# 跨 Provider 历史默认全量读取\_TASK_2026-04-02

## Context

- 当前历史发现链路默认会按当前 `model_provider` 收窄结果，主要体现在服务端 `thread/list` 默认过滤、本地 CLI `resume --last` 查询、本地 TUI 最近会话查询和本地 TUI resume picker 默认只看当前 provider。
- 当前继续旧线程时，CLI 与 embedded TUI 会显式把当前 `config.model_provider_id` 带进 `thread/resume`；remote TUI 当前不会从客户端注入 `model_provider`，而是沿用远端 app server 的既有语义。本任务不改 remote TUI 这条口径。
- `--fork --last` 与 fork picker 当前复用了部分最近会话/历史 picker helper。本任务默认不改 fork 流程的 provider 过滤语义；若实现复用同一 helper，必须拆分 resume 与 fork 的默认 provider 口径，禁止顺手把 fork 流程也改成跨 provider。
- 本次除了代码任务外，还需要把该能力补进 [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 的长期本地定制清单。

## Goal

- 默认历史发现链路不再按当前 provider 分割，覆盖服务端 `thread/list` 默认值、本地 CLI `resume --last`、本地 TUI 最近会话查询、本地 TUI resume picker。
- CLI 与 embedded TUI 继续旧线程时仍使用当前 provider，不自动切换到历史线程记录的 provider；remote TUI 维持现状，不从客户端注入 provider。
- fork 流程保持当前 provider 过滤语义不变，不能被本次改动连带放开。
- 将该行为写入 `local1` 长期定制清单，作为后续同步官方版本时的回归核对项。

## 执行状态

- [x] 已补 task 执行骨架与静态验证约束
- [x] 已完成服务端 `thread/list` 默认 provider 过滤修改
- [x] 已完成 CLI `resume --last` 默认 provider 过滤修改
- [x] 已完成 TUI resume 默认 provider 过滤修改
- [x] 已完成 fork/remote 口径保护
- [x] 已完成测试源码同步改写（未执行）
- [x] 已完成 `local1` checklist 文档回写
- [x] 已完成 subagent 严格复核附录写入
- [x] 已完成主 agent 二次审核与 task 复写

## 执行记录

- 初始约束：全程禁止编译、构建、测试、格式化触发式命令；仅允许静态源码核对、`git diff`、`git diff --check`、编码检查。
- 2026-04-02 22:36:15：已在 `codex-rs/app-server/src/codex_message_processor.rs` 把 `thread/list` 的 `modelProviders = None` 默认值从“当前 provider”改为“全 provider”；显式 provider 列表仍过滤，显式空数组仍走全量。
- 2026-04-02 22:36:15：已在 `codex-rs/exec/src/lib.rs` 把 `resume --last` 的历史查询默认 provider 过滤改为不注入；同时保留 `thread_resume_params_from_config()` 继续显式带当前 `config.model_provider_id`。
- 2026-04-02 22:36:15：已在 `codex-rs/tui/src/lib.rs` 拆分最近会话查询的 provider 默认值：embedded `resume --last` 走全 provider，embedded `fork --last` 保持当前 provider，remote 继续不从客户端注入 provider。
- 2026-04-02 22:36:15：已在 `codex-rs/tui/src/resume_picker.rs` 拆分 picker 默认 provider 过滤：本地 resume picker 走全 provider，本地 fork picker 保持当前 provider，remote picker 继续全量；同时清理过期注释。
- 2026-04-02 22:36:15：已补静态测试源码：服务端补 `None / [] / [provider]` 语义覆盖；CLI 补历史查询默认值与 resume 注入保留测试；TUI 补 embedded resume、embedded fork、remote 的 provider 差异测试；未执行任何测试。
- 2026-04-02 22:36:15：已审计 `exec`、`tui`、`debug-client`、`app-server-test-client` 的 `thread/list` 调用方；`debug-client` 与 `app-server-test-client` 已使用 `model_providers: None`，本次无需额外代码修改。
- 2026-04-02 22:36:15：已同步更新 `codex/docs/local1-custom-feature-checklist-2026-03-28.md`，新增 `F11` 与对应必查清单项。
- 2026-04-02 22:36:15：静态核查已执行 `rg`、只读源码复查、`git diff --check`；未执行任何编译、构建、测试、格式化命令，原因是本 task 明确禁止所有编译类验证。
- 2026-04-02 22:36:15：根据 reviewer 附录，已修正文档口径，明确“默认不按 provider 过滤”不等于“取消 cwd / `--all` / `show_all` 语义”；相关边界已同步回写到 task 与 `local1` 文档。
- 2026-04-02 22:36:15：根据 reviewer 附录，已在 `codex-rs/tui/src/resume_picker.rs` 把本地 rollout picker 的 `ProviderFilter::Any` 分支改为“不过滤 provider，但 fallback 仍显式使用当前 provider”，并补静态测试源码覆盖该 fallback 口径。
- 2026-04-02 22:36:15：根据 reviewer 附录，已把 `codex-rs/tui/src/app_server_session.rs` 从“关键修改文件”改为“关键审计文件”，明确该文件本次未改代码，仅用于确认 remote 语义保持不变。

## 关键修改与审计文件

- `codex-rs/app-server/src/codex_message_processor.rs`
- `codex-rs/app-server/tests/suite/v2/thread_list.rs`
- `codex-rs/exec/src/lib.rs`
- `codex-rs/tui/src/lib.rs`
- `codex-rs/tui/src/resume_picker.rs`
- `codex-rs/tui/src/app_server_session.rs`（未改代码，仅审计 remote 生命周期参数语义保持不变）
- `codex/docs/local1-custom-feature-checklist-2026-03-28.md`

## Checklist

- [x] 调整 `codex-rs/app-server/src/codex_message_processor.rs` 中 `thread/list` 的默认 provider 过滤逻辑：
      未传 `modelProviders` 时不再自动回退到当前 `config.model_provider_id`；显式传入 provider 列表时继续按传入值过滤；显式传空数组时继续表示“所有 provider”。
- [x] 保持 `thread/list` 返回结构不变：
      继续返回线程自身的 `model_provider` 字段，供调用方按需展示或记录来源。
- [x] 调整 `codex-rs/exec/src/lib.rs` 中 `resume --last` 的历史查询逻辑：
      最近会话查询不再自动注入当前 provider 过滤；通过线程 ID 或线程名恢复的现有逻辑保持不变，不改当前按 `thread.name` 客户端扫描匹配的逻辑。
- [x] 保持 CLI 恢复旧线程时的 provider 口径不变：
      `thread_resume_params_from_config()` 仍显式带上当前 `config.model_provider_id`，不自动切换到历史线程记录的 provider。
- [x] 调整 `codex-rs/tui/src/lib.rs` 中最近会话查询参数：
      本地 `resume --last` 默认不再传 `model_providers = Some(vec![config.model_provider_id.clone()])`；本地 `fork --last` 维持当前 provider 过滤；remote 模式维持现状。
- [x] 调整 `codex-rs/tui/src/resume_picker.rs` 中 provider 默认筛选：
      本地 resume picker 默认从 `ProviderFilter::MatchDefault(config.model_provider_id.to_string())` 改为全量 provider；本地 fork picker 保持当前 provider 过滤；remote picker 继续保持全量语义。
- [x] 保持 remote TUI 生命周期参数口径不变：
      `codex-rs/tui/src/app_server_session.rs` 中 `ThreadParamsMode::Remote` 的 `thread_start_params_from_config()`、`thread_resume_params_from_config()`、`thread_fork_params_from_config()` 继续不从客户端注入 `model_provider`。
- [x] 同步更新注释和测试名：
      删除 `codex-rs/tui/src/resume_picker.rs` 中“只看当前 provider”之类的过期注释；若测试名依赖旧语义，必须一并改成新口径。
- [x] 补齐测试源码改写：
      服务端补或改 `thread_list` 测试，覆盖“未传 provider 默认全量、显式 provider 仍过滤、显式空数组仍全量、返回项 `model_provider` 不丢失”；CLI 补或改 `resume_lookup_model_providers()` 与 `thread_resume_params_from_config()` 相关测试；TUI 补或改 local resume、local fork、remote 三种 provider 口径差异测试。若共享 helper 被拆分，补对应静态单测源码。
- [x] 审核所有 `model_providers: None` 调用方：
      至少复核 `exec`、`tui`、`debug-client`、`app-server-test-client` 是否接受 `thread/list` 的新默认值；若仅需审计、不需改代码，也必须在执行记录中记明。
- [x] 修改 [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md)：
      在“定制功能主清单”新增 `F11`，并在“同步官方后的必查清单”新增对应回归项。新增内容按以下文案写死：

  `F11` 行建议内容：

  `| F11 | 历史默认不按 Provider 分割 | 本地历史列表在未显式传 provider 时默认可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / \`--all\` / \`show_all\` 语义的前提下，默认不再按 provider 过滤；CLI 与 embedded TUI 继续旧线程时仍使用当前 provider，remote TUI 维持现状，不自动切换到历史线程记录的 provider。 | codex-rs/app-server/src/codex_message_processor.rs、codex-rs/exec/src/lib.rs、codex-rs/tui/src/lib.rs、codex-rs/tui/src/resume_picker.rs、codex-rs/tui/src/app_server_session.rs 当前存在 provider 默认过滤、resume provider 注入差异或共享 helper 口径分流。 | 历史发现链路默认不再按 provider 收窄，但仍保留现有 cwd / \`--all\` / \`show_all\` 语义；continue/resume 仍按本文约定发起；\`thread/list\` 返回项仍保留 \`model_provider\` 字段。 |`

  “同步官方后的必查清单”新增一条：

  `- [ ] 历史列表在未显式传 provider 时默认仍可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / \`--all\` / \`show_all\` 语义前提下默认仍不按 provider 过滤；CLI 与 embedded TUI 继续旧线程仍默认使用当前 provider；remote TUI 仍不从客户端注入 provider。`

## Acceptance

- `thread/list` 在未传 `modelProviders` 时，默认可返回不同 provider 下的线程。
- 本地 CLI `resume --last` 在保留现有 cwd / `--all` 语义前提下，默认按全 provider 历史选择最近线程，而不是只看当前 provider。
- 本地 TUI `resume --last` 和本地 resume picker 在保留现有 cwd / `show_all` 语义前提下，默认可发现不同 provider 下的线程。
- 本地 `fork --last` 和本地 fork picker 不因本次改动被连带放开到全 provider。
- CLI 与 embedded TUI 继续旧线程时仍按当前 provider 发起，不自动切换到历史线程记录的 provider。
- remote TUI 生命周期参数仍不从客户端注入 provider。
- `codex/docs/local1-custom-feature-checklist-2026-03-28.md` 已新增 `F11`，且必查清单新增对应回归项。

## Notes

- 本任务不改 SQLite 结构，不改 `threads.model_provider` 持久化字段，只改默认查询和本地历史发现行为。
- 本任务不改 CLI / TUI 现有 cwd 过滤默认值；`--all` / `show_all` 仍继续控制是否放开 cwd 过滤。
- 本任务不实现“跨 provider 恢复时自动切换到历史线程 provider”。
- 本任务不承诺在 picker UI 中直接显示 provider；若未新增该显示能力，文档和验收只能声称 `thread/list` 返回项保留 `model_provider` 字段。
- 全程禁止编译验证，仅允许静态核查。

## Subagent严格复核附录

**按严重度排序的问题清单**

1. 严重度：中
   问题描述：task 和 `local1` 清单把“跨 provider 默认全量”写成了“默认展示/看到所有 provider 下的线程”，但当前实现仍然保留了原有的 cwd 过滤，只有在 `--all` / `show_all` 时才会放开 cwd。对应文案见 [cross-provider-history-default-task-2026-04-02.md](/E:/vscodeProject/codex_github/codex/docs/cross-provider-history-default-task-2026-04-02.md#L78)、[cross-provider-history-default-task-2026-04-02.md](/E:/vscodeProject/codex_github/codex/docs/cross-provider-history-default-task-2026-04-02.md#L82)、[local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md#L36)、[local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md#L49)。而代码里 CLI 仍然在默认路径下按 cwd 过滤，见 [lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/exec/src/lib.rs#L1160) 和 [lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/exec/src/lib.rs#L1183)；TUI `resume --last` 仍然构造 `filter_cwd`，见 [lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/lib.rs#L1191) 和 [lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/lib.rs#L1194)；resume picker 默认也仍然按 cwd 过滤，见 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L239) 、 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L245) 和 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L867)。
   为什么是问题：这会把“去掉 provider 默认过滤”误写成“去掉全部默认过滤”。后续按文档回归时，维护者可能会把“仍然受 cwd 限制”的现状误判为 bug，或者错误地继续放开 cwd 过滤，造成超出本次需求范围的行为变更。
   修改建议：把所有相关文案改成“在保留现有 cwd / `--all` / `show_all` 语义的前提下，默认不再按 provider 过滤”或等价表述。`F11` 行和必查清单都应补上这个边界。
   建议落到 task 的哪个章节：`Checklist` 中 `local1` 文案项、`Acceptance`、`Notes`、`执行记录`。

2. 严重度：低（风险）
   问题描述：本次把本地 resume picker 默认值改成了 `ProviderFilter::Any`，见 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L297) 到 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L308)。但本地 rollout loader 在 `ProviderFilter::Any` 分支下会把 `default_provider.as_deref().unwrap_or_default()` 传给 `RolloutRecorder::list_threads()`，见 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L334) 和 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L341)。`RolloutRecorder::list_threads()` 及其后续 cwd/metadata 回退链路仍然接收这个 `default_provider`，见 [recorder.rs](/E:/vscodeProject/codex_github/codex/codex-rs/rollout/src/recorder.rs#L165)、[recorder.rs](/E:/vscodeProject/codex_github/codex/codex-rs/rollout/src/recorder.rs#L172)、[recorder.rs](/E:/vscodeProject/codex_github/codex/codex-rs/rollout/src/recorder.rs#L1022) 和 [recorder.rs](/E:/vscodeProject/codex_github/codex/codex-rs/rollout/src/recorder.rs#L1069)。当前新增测试只验证了 helper 选择 `ProviderFilter::Any`，没有覆盖“本地 rollout picker + Any + legacy rollout / metadata fallback”这条新路径，见 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L1891) 到 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs#L1938)。
   为什么是问题：这是本次改动首次让本地 rollout picker 真实走到 `Any` 分支。静态上不能证明空字符串 fallback 一定安全，尤其是老 rollout 缺失 `model_provider`、需要 metadata 回退或 cwd 二次解析时，可能出现边界行为偏差。当前我把它判断为“风险”，不是已证实缺陷。
   修改建议：二选一即可。
   其一，把 rollout loader 在 `ProviderFilter::Any` 时仍传 `config.model_provider_id` 作为 fallback provider，只是不传过滤条件。
   其二，至少补一个静态测试源码，明确覆盖“本地 rollout picker 的 `Any` 分支对缺失 provider 的老 rollout 仍然安全”。
   建议验证点：包含缺失 `session_meta.model_provider` 的历史 rollout、开启 cwd 过滤、关闭 `show_all` 的本地 resume picker。
   建议落到 task 的哪个章节：`Checklist` 的“补齐测试源码改写”、`Notes`、`执行记录`。

3. 严重度：低
   问题描述：task 的“关键修改文件”把 [cross-provider-history-default-task-2026-04-02.md](/E:/vscodeProject/codex_github/codex/docs/cross-provider-history-default-task-2026-04-02.md#L41) 中的 [app_server_session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/app_server_session.rs#L857) 列成了“修改文件”，但本次我看到的是“作为不变语义的审计对象”，没有对应源码 diff。
   为什么是问题：这会让后续读 task 的人误以为 remote 生命周期参数实现也有实际改动，增加复盘成本，也会弱化“这里只做语义保护、未改代码”的边界。
   修改建议：把该文件从“关键修改文件”改成“关键审计文件”或在同一行明确标注“未改代码，仅复核 remote 语义保持不变”。
   建议落到 task 的哪个章节：`关键修改文件`、`执行记录`。

**剩余风险或验证缺口**

- 未执行任何编译、测试、构建、格式化。这是按任务约束刻意保留的验证缺口，不代表通过了编译或测试。
- 除上面列出的文档精度问题和 rollout picker `Any` 分支覆盖风险外，我没有再发现更高严重度的已证实缺陷。
- 服务端 `thread/list` 的三种 provider 语义、CLI `resume --last` 默认不再注入 provider、本地 TUI `resume --last` / 本地 resume picker 放开 provider、fork/remote 维持原语义，这几条从当前静态代码看是对齐的。

## 主Agent审核处理结果

- 问题 1：采纳。原因：reviewer 指出的是文档边界不精确，不涉及既有需求变更；当前实现确实仍保留 cwd / `--all` / `show_all` 语义。对应改动：已重写 task 的 `执行记录`、`关键修改与审计文件`、`Checklist` 中 `local1` 文案项、`Acceptance`、`Notes`，并同步重写 [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 的 `F11` 与必查清单文案。
- 问题 2：采纳。原因：这是 reviewer 提出的真实边界风险；本地 rollout picker 的 `ProviderFilter::Any` 新路径不应把 fallback provider 降成空字符串。对应改动：已在 [resume_picker.rs](/E:/vscodeProject/codex_github/codex/codex-rs/tui/src/resume_picker.rs) 新增 `rollout_provider_filter_and_fallback()`，使 `Any` 分支保持“无 provider 过滤 + 当前 provider fallback”；并补静态测试源码覆盖 `Any` fallback 与 `MatchDefault` fallback 两条路径。
- 问题 3：采纳。原因：`app_server_session.rs` 本次确实只是审计对象，不应被写成实际修改文件。对应改动：task 已将该文件改为“未改代码，仅审计 remote 生命周期参数语义保持不变”，并在执行记录中补记该结论。
