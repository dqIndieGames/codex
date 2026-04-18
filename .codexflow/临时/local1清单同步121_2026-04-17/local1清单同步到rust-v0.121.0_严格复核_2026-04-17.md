# local1 清单同步到 rust-v0.121.0 严格复核报告_2026-04-17

## 复核范围

- 复核对象 1：`I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md`
- 复核对象 2：`I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md`
- 官方 121 真值范围固定为 `2026-04-15` 发布的 `rust-v0.121.0`
  - release：<https://github.com/openai/codex/releases/tag/rust-v0.121.0>
  - compare：<https://github.com/openai/codex/compare/rust-v0.120.0...rust-v0.121.0>
- 明确排除 `2026-04-16` 的 `rust-v0.122.0-alpha.3` 及其后的 `main`
- 复核方式：仅做文档与仓库静态真值复核，不做构建、不做测试、不扩成代码实现审查

## 结论摘要

- 本次共发现 `5` 条问题：`1` 条高严重度，`3` 条中严重度，`1` 条低严重度风险。
- 已确认 `checklist` 不只是补了一个 `121 同步判定矩阵`。`F1-F15` 与归档 `A1/A2` 主表、`同步官方后的必查清单`、`2026-04-10 归档补充` 都已经做了执行版 `121` 同步扩写，不属于“只加矩阵未同步正文”。
- 已确认 `TASK` 末尾仍保留 `Subagent严格复核附录` 与 `主Agent审核处理结果` 两个章节，且它们确实是全文最后两个章节。
- 已做 repo truth spot-check，当前未发现 `F10`、`F13/A2`、`F14`、`F15`、`A1` 与当前仓库真值直接冲突；本报告的主要问题集中在真值来源边界、自身闭环、以及 `F11` 的当前现状描述。

## 证据边界

- 本报告同时引用两类证据：
  - 官方 `rust-v0.121.0` release/compare 页面，作为官方 `121` 真值
  - 当前仓库源码/脚本/文档静态内容，作为“当前仓库真值”
- 未执行 release smoke、最小构建或任何运行时验证，因此所有结论都属于“静态复核结论”，不冒充 runtime pass。

## Findings

### Finding 1

- 严重度：高
- 问题描述：两份文档把“允许使用的真值来源”写成只认三类页面/文档来源，但正文实际又大量依赖当前仓库源码与 GitHub API 时间戳；这与本次任务要求核对“当前仓库真值或官方 121 真值”不闭合，也让文档自身的证据边界自相矛盾。
- 证据：
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md:11-17` 明写“允许使用的真值来源只有三类”：当前 checklist、官方 release 页面、官方 compare 页面。
  - 同一份 checklist 的 `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md:21-24` 与 `:58-74` 又直接基于“当前仓库代码”和多个本地源码路径写“当前推断范围”“当前代码迹象”。
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:20` 写入了“对应 API 返回时间为 `2026-04-15T20:45:18Z`”，这已经超出它自己在 `:25-28` 所限定的三类来源。
  - 同一份 TASK 的 `:58-61`、`:68-116` 又实际使用了官方热点与本地代码路径作为判断依据。
- 为什么是问题：
  - 如果后续执行者严格照文档执行，会得到一个自相矛盾的流程：文档口头上禁止把“当前仓库源码”当真值，但正文又不断以源码现状下判断。
  - 这会直接削弱 `121` 边界闭合度。尤其在 `F10/F11/A1/F14/F15` 这类“官方 121 没完全写、但当前 repo 已落地”的条目上，执行者可能错误忽略 repo truth，只剩文字对齐。
  - TASK 还额外引入了 GitHub API 时间戳，进一步破坏了它自己写下的“只认三类来源”规则。
- 修改建议：
  - 把“官方 121 真值”和“当前仓库真值”明确拆成两层，并正式把“当前仓库相关源码/脚本/文档静态核对”加入允许证据源。
  - 如果坚持官方侧只认 release/compare 页面，就删除 TASK 里的 API 时间戳表述，不要把 API 返回值混入基线定义。
  - 在两份文档里统一声明：官方边界只认 `2026-04-15` 的 `rust-v0.121.0` release/compare，repo 侧则允许使用当前仓库静态真值做对照。
- 建议落到章节：
  - checklist：`121 同步基线（2026-04-17）`
  - checklist：`当前推断范围`
  - TASK：`Context`
  - TASK：`Upstream Baseline`
  - TASK：`审查边界`

### Finding 2

- 严重度：中
- 问题描述：`checklist` 的 `F11` 主表“当前代码迹象”仍把“provider 默认过滤”写成当前现状，但当前仓库真值已经不是这个状态；默认 resume/history discover 已经放开为 all providers，保留下来的只是 fork/current-provider fallback 与 embedded/remote 注入差异。
- 证据：
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md:70` 写的是“当前存在 provider 默认过滤、resume provider 注入差异或共享 helper 口径分流”。
  - 当前仓库里，`I:\vscodeProject\codex\codex-rs\exec\src\lib.rs:1240-1245` 的 `resume_lookup_model_providers(...)` 直接返回 `None`。
  - `I:\vscodeProject\codex\codex-rs\exec\src\lib.rs:1864-1891` 的测试 `resume_lookup_model_providers_omit_default_provider_filters` 明确断言默认 provider filter 为 `None`。
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs:297-309` 明确写出：local `Resume => ProviderFilter::Any`，只有 `Fork => MatchDefault(...)`。
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs:1902-1949` 的测试再次确认：local resume 是 `Any`，remote sessions 也是 `Any`。
  - `I:\vscodeProject\codex\codex-rs\tui\src\app_server_session.rs:136-138`、`:912-914`、`:1192-1197` 显示 embedded 会注入当前 provider/cwd，remote start/resume/fork 则不注入 provider/cwd。
- 为什么是问题：
  - `F11` 是本轮用户点名的高漂移热点之一，主表如果把“当前状态”写错，会直接误导后续 reviewer 和主 agent 对真实风险面的判断。
  - 现在真正需要钉死的是“默认历史发现已不按 provider 过滤，但 continue/fork fallback 与 embedded/remote 参数注入仍有差异”，而不是再把“provider 默认过滤仍存在”当作现状。
  - 这类错误会导致后续把真实回归解释成“官方 121 带来的 provider 默认过滤残留”，从而偏离真正的 repo truth。
- 修改建议：
  - 把 `F11` 的“当前代码迹象”改成与现状一致的描述：
    - 默认 resume / history discover 不再按 provider 过滤
    - fork 或继续旧线程的 fallback 仍保留当前 provider 语义
    - embedded 会注入 provider/cwd，remote 不注入
    - `thread/list` 返回项仍保留 `model_provider`
  - `TASK` 里的 `F11 专项风险复核` 当前方向基本正确，建议把 checklist 主表与该专项段落对齐。
- 建议落到章节：
  - checklist：`F11 | 历史默认不按 Provider 分割`
  - checklist：`同步官方后的必查清单` 中 `F11` 相关条目

### Finding 3

- 严重度：中
- 问题描述：`checklist` 的默认审查边界把 tests/snapshots/fixtures 排除在外，但 `F4` 恰好把“快照与断言守护”定义成 local1 的核心基线，导致 `F4` 在默认流程下无法真正闭环。
- 证据：
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md:29-31` 写明默认不审查测试文件、snapshot、fixture，也不执行测试。
  - 同一份 checklist 的 `:63` 把 `F4` 明确定义为“所有和 local1 版本展示直接相关的 UI 出口，都要有快照或断言保护”。
  - `I:\vscodeProject\codex\docs\local1-custom-feature-checklist-2026-03-28.md:78-80` 的必查清单又要求“与 local1 相关的快照和断言没有被官方更新回滚掉”。
- 为什么是问题：
  - `F4` 的通过条件本身就建立在 tests/snapshots 上，但文档默认流程又把这些证据面排除了。
  - 这会让文档表面上覆盖了 `F1-F15/A1/A2`，实际上却允许执行者在完全不看快照/断言的情况下把 `F4` 结掉。
  - 对“121 执行版同步”来说，这属于边界不闭合，不是单纯措辞问题。
- 修改建议：
  - 在 `审查边界` 中为 local1 回归清单加一个明确例外：凡是 `F4` 或任何以测试/快照为真值面的条目，静态审查必须包含相关 tests/snapshots/fixtures。
  - 如果不想全局放开，就在 `F4` 或 `同步官方后的必查清单` 中明确写“本条不受默认 tests/snapshot 排除规则约束”。
  - 同时继续保持“不执行测试”的边界，但允许静态复读相关测试与快照文件。
- 建议落到章节：
  - checklist：`审查边界`
  - checklist：`F4 | local1 的测试与快照基线`
  - checklist：`同步官方后的必查清单`

### Finding 4

- 严重度：中
- 问题描述：`TASK` 已经定义了条目状态枚举、两轮 reviewer、三份 evidence 日志和 release-only 验证要求，但没有给“最终证据矩阵”定义任何固定章节、固定路径或固定格式；审计闭环在“最终每条 F/A 项怎么收口”这一步断开。
- 证据：
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:204-209` 定义了最终状态枚举：`Passed Runtime / Passed Mixed / Passed Static / Blocked / Failed`。
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:217-245` 定义了两轮 reviewer 路径与三份日志固定字段。
  - 但全文对“最终证据矩阵”的唯一出现只有 `:271` 这一句：“若涉及条目状态变化，需同步写入最终证据矩阵”。
  - 当前文档章节在 `:254-271` 结束于 `Subagent严格复核附录` 与 `主Agent审核处理结果`，没有任何矩阵章节或独立文件路径。
- 为什么是问题：
  - 当前文档把“怎么收集证据”写清了，却没有把“F1-F15/A1/A2 最终逐项状态落到哪里”写清。
  - 一旦开始走 reviewer、主 agent 采纳/不采纳、release-only 验证，就会出现日志齐全但最终 item-by-item 状态无统一归宿的问题。
  - 这直接削弱了用户点名要求的“审计轨迹、日志产物、两轮 reviewer、release-only 验证要求是否完整闭环”。
- 修改建议：
  - 不要把新章节追加到文末，否则会破坏“最后两个章节必须是附录和主Agent处理结果”的要求。
  - 更稳妥的做法有两种，任选其一：
    - 在 `Raw Evidence` 中固定新增一个独立产物路径，例如 `final_evidence_matrix.md` 或 `final_evidence_matrix.json`
    - 或者在 `Validation Plan` 后、`用户/玩家视角直观变化清单` 前新增 `最终证据矩阵` 章节
  - 至少应固定字段：`ID | 最终状态 | 证据类型 | 证据路径 | reviewer结论 | 主Agent处理结果 | 备注`
- 建议落到章节：
  - TASK：`Validation Plan`
  - TASK：`Raw Evidence`
  - TASK：`主Agent审核处理结果`

### Finding 5

- 严重度：低（风险）
- 问题描述：`TASK` 虽然已经转成执行版，但全文仍残留“只写一份 TASK 文档”“文档冻结版”等旧阶段话术；这些内容主要出现在执行记录和附录约束中，风险不高，但会干扰全文检索和阶段判断。
- 证据：
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:63` 仍保留“不是‘只写一份 TASK 文档’”。
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:64` 仍保留“把本文从‘文档冻结版’改成……”。
  - `I:\vscodeProject\codex\.codexflow\临时\local1清单同步121_2026-04-17\local1清单同步到rust-v0.121.0_TASK_2026-04-17.md:263` 仍保留“不保留旧阶段‘文档冻结版’ finding”。
- 为什么是问题：
  - 用户已明确要求核查是否仍残留这类阶段性话术；当前全文检索仍然会命中这些词。
  - 虽然这些表述大多属于历史记录，不会直接改写执行口径，但在 reviewer 或主 agent 用关键词检索时，容易误判当前阶段是否仍是“文档冻结版”。
- 修改建议：
  - 如果这些内容必须保留历史轨迹，建议集中到单独的“历史纠偏记录”块，并改写成过去式，不再在执行正文里反复出现。
  - 如果不需要保留原措辞，直接替换成不带阶段标签的描述，例如“已完成目标纠偏”“已清理旧阶段表述”。
- 建议落到章节：
  - TASK：`执行记录`
  - TASK：`Subagent严格复核附录`

## 已确认通过项

- `checklist` 对 `F10` 的 config-first tray 边界与当前 repo truth 一致：
  - `scripts/windows_app_server_refresh_tray.py` 仅复制 `base_url` 与 `experimental_bearer_token`
  - `thread_provider_runtime_refresh.rs` 仍覆盖相对 `agents/...` / `./agents/...` 与 `failed_threads=[]`、`total_threads=0` 语义
- `checklist` 与 `TASK` 对 `F13/A2` 的顶层 `force_gpt54_priority_fallback` 边界，与 `codex-rs/core/src/config/mod.rs` 和 `codex-rs/core/src/client.rs` 当前实现一致。
- `checklist` 对 `F14/F15` 的“默认 warn，显式 `RUST_LOG` 可覆盖”边界，与 `codex-rs/app-server/src/lib.rs:92`、`codex-rs/tui/src/lib.rs:75`、`docs/install.md:54-56` 当前真值一致。
- `checklist` 对 `A1` 的“仅 brand-new / Clear + 单个纯文本 `你好` + 非 subagent + 三条消息消费链前缀一致”边界，与 `codex-rs/core/src/codex.rs:2214-2378`、`codex-rs/core/src/stream_events_utils.rs:41-45` 当前静态真值一致。

## 残余风险 / 证据边界

- 本报告没有执行构建、release smoke 或运行时验证，因此没有对 `TASK` 中的 release-only 验证链做真实性核验；这里只复核“文档定义是否闭环”。
- 对 `F10/F11/A1` 的 repo truth 复核基于当前仓库静态代码与现有测试文件文本，不等于重新证明运行时行为已经通过。
- 官方 `121` 侧本报告只认 `2026-04-15` 的 `rust-v0.121.0` release/compare 页面，不采信 `2026-04-16` 的 `rust-v0.122.0-alpha.3` 或之后的 `main`。
