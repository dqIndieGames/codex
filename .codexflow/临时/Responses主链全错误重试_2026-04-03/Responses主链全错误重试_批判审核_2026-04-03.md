# Responses主链全错误重试_批判审核_2026-04-03

## 审核对象

- 主文档：`E:\vscodeProject\codex_github\codex\.codexflow\临时\Responses主链全错误重试_2026-04-03\Responses主链全错误重试_TASK_2026-04-03.md`
- 审核方式：同配置 reviewer subagent 批判审核
- 审核重点：
  - `401 Unauthorized` 恢复优先级与预算语义
  - `每次重试完整显示错误详情，但不进入历史记录` 的实现契约
  - 归档到 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md` 的任务粒度
  - release、bounded、`403` 等高风险路径的测试与验收

## 原始 Findings

### 1. 严重：归档任务还不够可直接执行

- 原 TASK 初稿只给了 `F5-F9` 的一句话改写方向。
- 目标归档文档里的 `F5-F9` 实际是 5 列表格行，不是单句定义。
- 未冻结的关键决策包括：
  - 到底只改 `明确定义`，还是连 `功能项 / 当前代码迹象 / 后续验收口径` 一起改。
  - “同步官方后的必查清单”是追加 4 条，还是替换现有与旧白名单语义冲突的条目。
- 风险：归档后同时保留“旧白名单语义”和“全错误重试语义”，与“只补 F5-F9”的目标冲突。

### 2. 严重：`401 Unauthorized` 的优先级写了方向，但没有冻结预算和事件语义

- 原 TASK 初稿只写了“先 recovery，再统一重试分类”。
- 未冻结的关键决策包括：
  - 每一次遇到 `401` 是否都先走 recovery。
  - recovery 后重新发起请求是否消耗现有 retry budget。
  - recovery 自身失败时是否要发普通 `will_retry = true` 的中间错误通知。
- 风险：实现时改变有效重试次数，或者在 auth recovery 与普通 retry 之间产生双重提示。

### 3. 中等：`完整显示但不进历史` 还没有冻结到可实现级别

- 原 TASK 初稿虽然要求复用 `CodexErr::to_string()`，也要求只更新状态区或等价详细展示区。
- 但仍未明确：
  - 中间 retry 的权威字段到底是什么。
  - “状态区或等价展示区”到底允许哪些 UI 出口。
  - 最终失败时是否必须沿用同一完整字符串进入终态错误路径。
- 风险：某些路径继续退化成 `429 retry` 这类摘要，或者不同路径落到不同 UI 面。

### 4. 中等：bounded exhaustion 的终态行为仍不够可验

- 原 TASK 初稿已经要求验证 bounded 模式下最终失败路径与 exhaustion 行为。
- 但验收条目只写了“预算语义不变”，没有冻结用户可见终态。
- 风险：实现者只验证配置层预算语义，却漏掉 exhaustion 前后 UX 分界。

## 采纳结论

- [x] 采纳 Finding 1：主文档将把归档任务改成逐行、逐列、逐清单项的直接编辑说明。
- [x] 采纳 Finding 2：主文档将新增 `401` 单独决策块，明确 recovery 优先级、预算计数和通知语义。
- [x] 采纳 Finding 3：主文档将新增“retry 详情展示契约”，写死权威字段、唯一展示出口和终态延续规则。
- [x] 采纳 Finding 4：主文档将新增 bounded exhaustion 的终态验收项，明确中间 retry 与最终失败的展示分界。

## 整合说明

- 本审核文档保留 reviewer 的原始批判上下文与采纳结论。
- 最终执行真值以整合后的 `Responses主链全错误重试_TASK_2026-04-03.md` 为准。
- 本次 reviewer 指出的 4 个 findings 全部采纳，不存在未采纳项。

---

## 实现后批判审查（Laplace）

### 审核输入

- 审核对象：
  - `E:\vscodeProject\codex_github\codex\.codexflow\临时\Responses主链全错误重试_2026-04-03\Responses主链全错误重试_TASK_2026-04-03.md`
  - `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md`
  - 本轮主链实现相关代码 diff
- 审核约束：
  - 不运行任何编译、构建、测试、lint 或等价命令
  - 只允许阅读代码、diff、测试文本和文档

### 原始 Findings

#### 1. 严重：read-side `401` 仍会绕过 unauthorized recovery

- 当前只有初始 HTTP 请求和 websocket connect 的 `401` 会走 `handle_unauthorized(...)`。
- 一旦 `401` 出现在 websocket wrapped error / SSE 读流错误，`map_response_stream(...)` 会直接把它映射成 turn 侧错误；随后高层因为 `401` 不可重试而直接终态失败。
- 风险：
  - 读流阶段的 `401` 不会先做 auth recovery。
  - 无法保住“recovery follow-up 不计普通 budget / 不发普通 `will_retry = true`”的既有语义。

#### 2. 严重：wrapped websocket semantic HTTP error 仍会提前降成终态

- `responses_websocket` 会把 wrapped `429 usage_limit_reached`、`400 invalid_request_error` 这类错误映射成 `ApiError::Transport(Http)`。
- 但高层旧 mapper 仍可能把它们提前降成 `UsageLimitReached` / `InvalidRequest` 等终态类型，而这些类型在 turn 层不可重试。
- 风险：
  - wrapped websocket `400`、`402/429 usage_limit_reached` 仍然跳过 turn retry。
  - 与“wrapped websocket error 纳入统一自动重试链”的目标冲突。

#### 3. 中等：请求层 `429 usage_limit_reached` retry details 未完全复用终态 formatter

- `format_retry_transport_error_details(...)` 之前只对 `402` 特判复用最终 mapper，其余 HTTP 状态都手工拼成 `UnexpectedResponseError`。
- 风险：
  - 请求层 `429 usage_limit_reached` 虽然不再只有摘要，但可能丢失现有 `UsageLimitReachedError` 可提供的语义化 details。

#### 4. 中等：`local1` 归档边界表象上超出了本轮允许范围

- 审核观察到 `local1` 当前工作树中存在 `F10` 与 Windows build 检查项改动。
- 风险：
  - 如果这些改动被误认为本轮写入，会与“本轮只改 `F5-F9` 与重试相关必查清单”的归档边界冲突。

### Open Questions / Residual Risks

- 还需要确认“真实 `401` recovery 控制流”是否有足够的文本级测试覆盖，尤其是 read-side `401` 之后的 follow-up telemetry 标记。
- 还需要确认 bounded exhaustion 文本级覆盖是否足够清晰，尤其是 wrapped websocket error 在 exhaustion 前后的可见行为分界。

### 待审核处置

- 第二轮审核 subagent 需要基于最新代码和本原始 findings，逐条判断：
  - 是否采纳
  - 是否已经修复
  - 是否属于用户已有工作树改动而非本轮写入
  - 是否需要同步回写 TASK 文档

### 第二轮处置结果（Faraday）

| Finding | 结论 | 原因 | 对应修改 |
|---|---|---|---|
| 1. read-side `401` 绕过 unauthorized recovery | 采纳 | 原 finding 成立；当前代码已修复。读流阶段拿到 `UnexpectedStatus(401)` 后，现在会先走 `recover_stream_unauthorized(...)`，成功则直接回到下一次真实请求，不先消耗普通 retry budget，也不先发普通 `will_retry = true` 提示。 | `codex-rs/core/src/client.rs` 新增 `pending_unauthorized_retry`、`unauthorized_transport_from_codex_error(...)`、`recover_stream_unauthorized(...)`；`codex-rs/core/src/codex.rs` 在普通 retry 判断前先调用 `recover_stream_unauthorized(...)`。 |
| 2. wrapped websocket semantic HTTP error 提前降成终态 | 采纳 | 原 finding 成立；当前代码已修复。turn 侧 wrapped websocket HTTP transport 错误现在直接保留为 `UnexpectedStatus(Turn)`，不再提前降为 `UsageLimitReached` / `InvalidRequest` 等终态，从而重新进入统一 turn retry 链。 | `codex-rs/core/src/api_bridge.rs` 的 `map_responses_stream_api_error(...)` 改为对 turn 侧 HTTP transport 错误直接构造 `UnexpectedStatus(Turn)`；`codex-rs/core/src/api_bridge_tests.rs`、`codex-rs/core/src/error_tests.rs` 增补文本级覆盖。 |
| 3. 请求层 `429 usage_limit_reached` retry details 未复用终态 formatter | 采纳 | 原 finding 成立；当前代码已修复。请求层 retry details 现在先复用 `map_responses_request_api_error(...).to_string()`，只有当映射结果是 `RetryLimit` 时才回退到 raw HTTP detail，因此 `429 usage_limit_reached` 会显示语义化 details，plain `429` 则保留原始 HTTP 详情。 | `codex-rs/core/src/client.rs` 的 `format_retry_transport_error_details(...)` 已按“非 `RetryLimit` 复用 mapped formatter”的规则改写；`codex-rs/core/src/client_tests.rs` 增补 usage-limit/plain-429 文本级断言。 |
| 4. `local1` 归档超出范围，动到了 `F10` / Windows build 项 | 采纳（仅文档侧） | 需要拆开处理。文档边界问题成立：TASK 必须明确“本轮归档写入只涉及 `F5-F9` 与重试相关必查清单”。但不应把 `message_processor.rs` 或 `F10` 现有差异直接当成本轮必须回退的代码目标，因为这些属于当前工作树里已存在的无关用户改动，本轮不能擅自覆盖或回退。 | 回写 TASK 文档，明确“本轮对 `local1` 的新增写入范围”与“工作树中已有的 `F10` / Windows build 相关改动并非本轮回退目标”的区别；不回退用户已有改动。 |

### Remaining Risks

- `read-side 401` 的恢复主流程已经补上，但当前 `unauthorized_transport_from_codex_error(...)` 重建的是 `headers: None` 的 `TransportError::Http`。这不影响“先 recovery 再继续”的核心行为，但会丢失原始 `request_id`、`cf-ray`、`auth error`、`auth error code` 这类 header debug 上下文，属于 telemetry / debug 完整性风险。
- 请求层 `400 invalid_request` 一类错误现在会复用终态 formatter；如果项目坚持“retry details 必须尽量保留全部 transport debug 字段”，那这一点仍有规格层面的完整性风险，因为当前 `InvalidRequest` formatter 本身不天然携带 URL / request id。
- 本轮没有执行任何测试、构建或 lint；新增覆盖仍是文本级 helper / mapper / UI 断言，不是动态端到端验证。

### 文档回写要求

- TASK 文档需要把 reviewer 相关 Checklist / Acceptance 标记为已完成，并补写“Finding 1-3 已修复、Finding 4 仅文档侧采纳”的处置说明。
- TASK 文档需要把“归档仅更新 `F5-F9` 与必查清单”改写成“本轮新增写入仅涉及 `F5-F9` 与重试相关必查清单”，避免与工作树中已有的 `F10` / Windows build 改动混淆。
- TASK 或批判文档里都要明确写明：本轮所有结论均来自纯阅读式静态复核，没有执行编译、测试或 lint。
