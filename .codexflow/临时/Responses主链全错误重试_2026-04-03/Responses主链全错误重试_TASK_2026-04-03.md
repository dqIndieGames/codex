# Responses主链全错误重试_TASK_2026-04-03

## Context

- 当前 `Responses` 主链的错误重试口径存在分裂：请求层主要按 `402`、`429`、`5xx`、传输层错误做白名单重试，高层 stream/turn 层又可能把更多 `UnexpectedStatus` 视为可重试。
- 当前很多 retry 可见信息只有 `429 retry`、`Reconnecting... 1` 这类摘要文案，无法让使用者看到完整错误详情。
- 当前 release 下首个 websocket retry 默认可能被隐藏，不满足“每次重试都要显示出来”的要求。
- 当前 retry 中间错误不能污染历史记录；本次必须继续保留“状态区可见、历史区干净”的体验约束。
- 当前 `401 Unauthorized` 已有现成 unauthorized recovery 链路；本次不能把它粗暴降级成普通重试，必须保留“先恢复认证，再进入统一重试分类”的顺序。
- 本次文档只覆盖 `Responses` 主链，不扩到工具执行、本地配置、sandbox、非 `/responses` 端点，也不顺手修改现有重试预算、bounded/unbounded 语义或单次 `10s` 退避上限。

## Goal

- 冻结一份可直接执行的实现任务清单，使 `Responses` 主链上的所有远端错误、传输错误、流式错误统一参与自动重试；每次重试都完整显示错误详情；这些中间错误不写入历史记录；`401` 仍先走现有认证恢复；现有预算与退避上限保持不变。

## 冻结决策

- `Responses` 主链上的所有远端错误、传输错误、流式错误都纳入统一重试分类；这里的“所有错误”不包含用户中断、turn abort、工具执行失败、sandbox、env/config、本地 prompt 构建失败等非主链错误。
- 每一次遇到 `401 Unauthorized` 都必须先走现有 unauthorized recovery，而不是直接走普通全错误重试。
- `401` 的 recovery follow-up 请求继续沿用现有 auth recovery follow-up 语义处理，不额外消耗本次统一重试改造里的普通请求/流式 retry 记数，也不额外发普通 `will_retry = true` 的通用 retry 提示。
- 如果 auth recovery 本身失败，继续沿现有 auth recovery failure / 上抛语义处理，不新增双重提示，不把 recovery 失败伪装成普通 `401 retry` 文案。
- 只有在 unauthorized recovery 完成后，后续仍落回 `Responses` 主链错误时，才继续进入统一重试分类。

## Retry详情展示契约

- 中间 retry 错误的权威详情字段固定为服务端预格式化的完整错误字符串；默认复用当前最终错误格式化真值，等价于 `CodexErr::to_string()`。
- app-server 或等价中间层必须把该完整字符串原样透传到 retry 通知链，优先通过 `additional_details` 或等价单一字段承载；客户端不得自行重新拼接第二套 retry 专用摘要格式。
- retry 详情的唯一用户可见出口固定为状态区、重试提示区及其绑定的详细错误展示区；允许沿用当前状态详情 UI，但不允许新增历史消息、历史错误单元或其他长期留存出口。
- 最终失败时继续走现有非 retry 终态错误路径；前序 retry 详情不回灌历史，但终态错误仍应沿用同一完整错误格式化真值进入最终失败展示。

## Checklist

- [x] 盘点 `Responses` 主链当前真值入口，明确请求层、stream/turn 层、错误映射层、app-server 通知层、TUI 展示层分别在哪里决定“是否重试”和“向用户显示什么错误信息”。
- [x] 明确定义 in-scope 与 out-of-scope 错误集合：本次只纳入 `Responses` 主链上的 HTTP 非 `2xx`、WebSocket wrapped error、SSE/stream disconnect、idle timeout、传输层错误；明确排除用户中断、turn abort、工具执行失败、sandbox、env/config、本地 prompt 构建失败等非主链错误。
- [x] 固化 `401 Unauthorized` 顺序：先执行现有 unauthorized recovery；只有 recovery 之后仍然失败，才进入统一重试分类；不得把 `401` 直接改成普通无限重试。
- [x] 统一请求层与高层 stream/turn 层的重试分类真值，去掉“低层不重试 `403`、高层又可能重试 `403`”这类分裂口径；本次要让 `Responses` 主链的 `400`、`403`、`404` 等远端错误也进入统一自动重试链。
- [x] 让每次 retry 事件都带完整错误详情，默认复用现有最终错误格式化真值，等价于当前 `CodexErr::to_string()` 能提供的完整信息集合；显示内容至少覆盖状态码、错误体、url、request id、trace 或 cf-ray、auth error、auth error code 等当前可提取字段，不再让 `429 retry` 之类摘要文案成为唯一可见内容。
- [x] 打通状态型 retry 路径的详细信息透传，让 HTTP retry、WebSocket retry、stream reconnect、idle timeout 等中间重试都能把完整错误详情显示到状态区或等价详细错误展示区；同时移除 release 下首个 websocket retry 被隐藏的行为。
- [x] 保持 retry 中间错误只走 `will_retry = true` 通知链，只更新状态区、重试提示区、详细错误展示区；不得写入历史消息，不得新增历史错误单元，不得把 replay 的 retry 错误回灌到历史区。
- [x] 补验证与测试，至少覆盖：HTTP `400`、`403`、`404`、`429` retry；`401` 认证恢复优先级；wrapped websocket error retry；release 下首个 retry 可见；app-server `will_retry = true` 时详情不丢失；TUI 状态区显示完整详情但历史区不落错误项；bounded 模式下最终失败路径与 retry exhaustion 行为未被破坏。
- [x] 按“归档更新细则”直接编辑 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md` 的现有 `F5-F9` 与“同步官方后的必查清单”，只补现有条目，不新增新的 `F` 项，也不新增独立归档章节。
- [x] 在 TASK 初稿完成后，启动同配置 reviewer subagent 做批判审核，单独产出 `Responses主链全错误重试_批判审核_2026-04-03.md`；审核完成后，主文档维护者必须逐条处理审核意见，并把采纳后的修改整合回本 TASK 文档。若某条审核意见不采纳，必须在 `Notes` 或文末补充说明中写明理由。

## 归档更新细则

- 归档目标文件固定为 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md`。
- 归档时仅修改 `F5-F9` 这 5 行以及“同步官方后的必查清单”，`F1-F4`、`F10-F11`、其他章节结构与标题保持不动。
- `F5` 的 `功能项` 保持为 `请求重试范围增强`；必须同时更新 `明确定义`、`当前代码迹象`、`后续验收口径` 三列，不得只改单列。更新后口径必须明确：`Responses` 主链默认不再依赖 `402`、`429`、`5xx` 白名单；在保留 `401` 认证恢复优先级前提下，其余主链远端错误统一视为可重试；`当前代码迹象` 要写到统一分类真值入口；`后续验收口径` 要体现 `400`、`403`、`404` 不再被回退成白名单外错误。
- `F6` 的 `功能项` 保持为 `流式重试与 UI 提示联动`；必须同时更新 `明确定义`、`当前代码迹象`、`后续验收口径` 三列。更新后口径必须明确：每次 retry 都显示完整错误详情；retry 详情只更新状态区和状态详情展示，不进入历史记录；`当前代码迹象` 要覆盖 app-server 透传与 TUI 状态展示链；`后续验收口径` 要体现首个 websocket retry 可见且历史区仍干净。
- `F7` 的 `功能项` 保持为 `单次重试等待时间上限为 10s`；必须同时更新 `明确定义` 与 `后续验收口径`，并在 `当前代码迹象` 里注明 `10s` cap 继续适用于这次扩大的全错误重试口径，而不是只适用于旧白名单状态。
- `F8` 的 `功能项` 保持为 `重试次数保持“大次数或等效无界”目标`；必须同时更新 `明确定义`、`当前代码迹象`、`后续验收口径` 三列。更新后口径必须明确：本次变化只扩展错误分类，不改变默认预算模式；`401` recovery follow-up 不额外改变普通 retry budget 语义；`后续验收口径` 必须区分统一错误分类与 bounded exhaustion 的终态表现。
- `F9` 的 `功能项` 保持为 `重试配置入口尽量统一`；必须同时更新 `明确定义`、`当前代码迹象`、`后续验收口径` 三列。更新后口径必须明确：请求层分类、高层分类、retry 详情透传、首个 retry 可见性统一收敛到同一真值入口。
- “同步官方后的必查清单”里，原有与旧白名单语义冲突的条目必须直接替换，不是简单并列追加。至少执行以下替换或补充：
  - 把仅检查 `402`、`429`、`5xx`、传输层错误的旧条目，替换为检查 `400`、`403`、`404`、`402`、`429`、`5xx`、传输层错误的统一重试口径，并明确 `401` 仍先走认证恢复。
  - 把“若要支持 `427` 或其他额外状态”的旧白名单延展条目，替换为“新增主链远端错误仍通过统一分类入口处理，不回退到散落白名单判断”。
  - 保留“单次退避上限仍然是 `10s`”和“默认重试预算与端点级例外仍能被清楚区分”这两条，但要让它们明确服务于扩大后的全错误重试口径。
  - 新增“首个 websocket retry 仍可见”。
  - 新增“retry 详情字段不丢失，且不再退化成只有摘要文案”。
  - 新增“retry 错误仍不污染历史区，最终失败才进入终态错误路径”。

## Acceptance

- [x] `Responses` 主链上的 `403 Forbidden` 示例错误会进入自动重试，而不是在请求层直接终止。
- [x] 每次 retry 都能看到完整错误详情，而不是只看到 `429 retry`、`Reconnecting...` 这类摘要。
- [x] retry 中间错误不会进入历史区，只会更新状态区、重试提示区或等价的详细错误展示区。
- [x] `401 Unauthorized` 仍先走现有认证恢复，认证恢复之后仍失败时才进入统一重试分类。
- [x] `401` recovery follow-up 不新增普通 retry budget 消耗，也不新增普通 `will_retry = true` 双重提示；auth recovery 本身失败时继续沿现有 auth recovery failure / 上抛语义处理。
- [x] release 下首个 websocket retry 也对用户可见，且同样带完整错误详情。
- [x] 单次退避上限仍为 `10s`，现有 bounded/unbounded 预算语义不变。
- [x] bounded exhaustion 前的 retry 详情仍不进入历史区；bounded exhaustion 后只最终失败进入终态错误路径，且 `403`、wrapped websocket error 等统一分类错误在 bounded 下也遵守同一分界。
- [x] 本轮对 `local1` 的新增写入仅涉及现有 `F5-F9` 与“同步官方后的必查清单”，不新增新的功能项或独立归档章节；工作树中已存在的 `F10` / Windows build 相关改动不属于本轮回退目标。
- [x] reviewer subagent 的审核意见已经按实际采纳情况整合回本 TASK 文档，最终执行真值以本 TASK 文档为准。

## 实施回写

- 本轮已实际落地代码修改，统一真值入口落在以下链路：
  - 请求层 `/responses` HTTP 分类：`codex-rs/codex-api/src/provider.rs` 的 `responses_http_status_is_retryable(...)` 与 `should_retry_request_error(...)`，由 `codex-rs/codex-api/src/telemetry.rs` 实际消费。
  - 高层错误映射与“是否还能继续 turn retry”边界：`codex-rs/core/src/api_bridge.rs`、`codex-rs/core/src/error.rs`、`codex-rs/core/src/client.rs`。
  - retry 详情透传与首个 websocket retry 可见性：`codex-rs/core/src/client.rs`、`codex-rs/core/src/codex.rs`。
  - app-server `will_retry = true` 透传与 TUI 状态区消费：`codex-rs/app-server/src/bespoke_event_handling.rs`、`codex-rs/tui/src/chatwidget.rs`。
- 实际实现采用“共享主链策略 + 请求层/turn 层来源分离”的方式，而不是简单把所有 HTTP 错误在高层再重试一遍：
  - `/responses` 请求层 HTTP 非 `2xx` 在请求层统一按“除 `401` 外都可重试”处理。
  - 为避免改变既有预算语义，请求层已处理过的 HTTP 错误不会再被高层 turn retry 二次放大。
  - websocket connect、wrapped websocket error、stream disconnect、idle timeout 仍保留在 turn 侧自动重试链中，并统一展示完整详情。
- `401 Unauthorized` 的现有 `handle_unauthorized(...)` 顺序保持不变；本轮没有把 `401` 降级成普通 retry，也没有给 recovery follow-up 增加额外普通 `will_retry = true` 通知。读流阶段若出现 `UnexpectedStatus(401)`，现在会先重建 transport error 并再次走 `handle_unauthorized(...)`，成功后直接重新发起真实请求，不先消耗普通 stream retry budget。
- turn 侧 wrapped websocket HTTP transport 错误现在直接保留为 `UnexpectedStatus(Turn)`，不再提前降成 `UsageLimitReached` / `InvalidRequest` 等终态，因此 wrapped `400`、`402/429 usage_limit_reached` 也会先进入统一 retry 链。
- 请求层 retry 事件现在会携带完整 `details`；`429 usage_limit_reached` 会复用语义化 formatter，plain `429` 才回退到 raw HTTP details，因此 `429 retry`、`402 retry`、`Reconnecting...` 等状态提示不再是唯一可见信息。
- release 下首个 websocket retry 的隐藏逻辑已经移除；首次 retry 现在和后续 retry 一样，都会显示状态摘要与完整错误详情。
- 文本级验证已同步补到现有测试文件，但本轮严格遵守约束，没有执行任何编译、构建、测试或 lint 命令。

## 静态复核结果

- 通过阅读 `codex-rs/core/src/client.rs` 可确认：`/responses` HTTP 请求错误与 websocket/stream 错误已分别映射到 `RequestLayer` / `Turn`，从而消除了“低层不重试 403、高层又重试 403”的分裂口径，同时避免双重计数。
- 通过阅读 `codex-rs/core/src/codex.rs` 与 `codex-rs/core/src/client.rs` 可确认：读流阶段 `401` 不再直接终态失败，而是先走 unauthorized recovery；recovery 成功后会把 `retry_after_unauthorized` 上下文挂到下一次真实请求的 telemetry 上。
- 通过阅读 `codex-rs/core/src/codex.rs`、`codex-rs/app-server/src/bespoke_event_handling.rs`、`codex-rs/tui/src/chatwidget.rs` 可确认：中间 retry 仍走 `will_retry = true` 状态链，`additional_details` 会透传到状态区，且 replay 路径仍跳过这些中间 retry 事件，不污染历史区。
- 通过阅读 `codex-rs/codex-api/src/telemetry.rs` 与 `codex-rs/codex-client/src/retry.rs` 可确认：请求层退避仍复用现有 `backoff(...)`，未改动 `10s` clamp 与现有 request budget 语义。
- 通过阅读 `codex-rs/core/src/api_bridge.rs`、`codex-rs/core/src/error.rs` 可确认：请求层 exhaustion 仍保留 request-layer `RetryLimit` 终态边界；而 turn 侧 wrapped HTTP transport 错误则不再提前降级，从而满足“wrapped websocket error 先重试、exhaustion 后再终态失败”的分界。

## 残余风险

- `read-side 401` 恢复主流程已经补上，但当前重建的 `TransportError::Http` 使用 `headers: None`，因此恢复 telemetry 会丢掉原始 header 里的 `request_id`、`cf-ray`、`auth error`、`auth error code`，这属于 debug 信息完整性风险，不影响主功能优先级。
- 请求层 `400 invalid_request` 等错误现在会复用终态 formatter；若后续坚持“retry details 必须尽量保留全部 transport debug 字段”，这里仍有规格层面的 formatter 完整性风险。
- 本轮新增的覆盖都是文本级 helper / mapper / UI 断言，没有执行动态端到端验证；bounded exhaustion、read-side `401` recovery 和 wrapped websocket error 的最终表现结论仍来自纯阅读式静态复核。

## Notes

- `完整显示` 默认指展示当前系统可生成的完整格式化错误字符串，不额外要求设计新的原始 JSON 展示界面。
- 本轮严格遵守用户硬约束：未执行 `cargo build`、`cargo check`、`cargo test`、`cargo clippy` 或其他等价编译/测试/lint 命令；所有复核均为纯阅读式静态复核。
- `Responses主链全错误重试_批判审核_2026-04-03.md` 仅保留原始批判意见与复核证据，不替代最终 TASK 真值。
- `local1` 当前工作树中存在与本轮无关的 `F10` / Windows build 相关用户改动；本轮既没有覆盖这些改动，也没有把它们当作回退目标。本轮对 `local1` 的新增写入范围仅限 `F5-F9` 与重试相关必查清单。
- 实现后 reviewer 结论已经整合：Finding 1-3 均“原问题成立，但现代码已修复”；Finding 4 “仅文档侧采纳”，用于澄清归档写入边界，而不是要求回退用户已有工作树改动。
