# local1 定制功能清单（2026-03-28）

## 用途

- 本文用于冻结你希望长期保留在个人分支里的定制功能。
- 这不是官方需求文档，而是你自己的长期维护基线。
- 后续无论同步 `rust-v0.118.0` 还是继续追 `upstream/main`，都应以本文作为合并后的回归核对清单。

## 当前推断范围

- 基于当前仓库代码和前序对话，先把你的个人定制目标明确为六条主线：`local1` 显示链、`Responses` 主链重试增强链、Provider runtime 热刷新链、跨 Provider 历史发现链、`gpt-5.4 priority` 请求层兜底链、Windows/TUI 默认日志降噪链。
- 本轮先不把“临时调试代码”“脏工作区噪音”“与定制目标无关的依赖更新”写进功能范围。
- 若后续你还有新的私人功能，再继续追加到本文，不和本轮目标混写。

## 审查边界

- 针对“审核”“代码审查”“合并差异核对”一类任务，默认只做静态代码审查。
- 默认不审查测试文件、snapshot、fixture、测试辅助代码，除非你明确要求把测试也纳入审查范围。
- 默认不做任何编译、构建、测试执行、格式化、lint，除非你明确要求执行。
- 若某次结论仅基于源码审查、未覆盖测试侧，我应在结论中明确说明这一点，不能偷偷扩范围。

## 定制功能主清单

| ID | 功能项 | 明确定义 | 当前代码迹象 | 后续验收口径 |
|---|---|---|---|---|
| F1 | `local1` 显示版本号 | 面向用户展示的 CLI/TUI 版本号默认不直接显示官方裸版本，而是显示 `<官方版本>-local1`。 | `codex-rs/cli/src/main.rs` 仍单独定义 CLI clap 帮助/版本输出用的 `CODEX_CLI_DISPLAY_VERSION`；`codex-rs/tui/src/version.rs` 提供 TUI 共享的 `CODEX_CLI_DISPLAY_VERSION`，并由 `codex-rs/tui/src/cli.rs` 与其他 TUI 展示链消费。 | 合并官方更新后，CLI 主入口帮助/版本输出与 TUI 用户可见版本入口都仍统一显示 `-local1` 后缀；即使实现上仍是两条常量链，也不能发生 CLI/TUI 漂移。 |
| F2 | 卡片与状态区融入 `local1` | 状态卡片、顶部状态区、相关面板里出现的版本号全部走 `CODEX_CLI_DISPLAY_VERSION`，不混用原始包版本。 | `codex-rs/tui/src/status/card.rs`、`codex-rs/tui/src/app.rs`、`codex-rs/tui/src/chatwidget/status_surfaces.rs` 已引用该常量。 | 卡片、状态区、标题区的版本展示口径保持一致，不出现一处 `local1`、一处官方裸版本的分裂。 |
| F3 | 历史单元与升级提示融入 `local1` | 历史消息、升级提示、版本跳转提示等文本，也必须展示 `local1` 版本名。 | `codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/chatwidget.rs`、`codex-rs/tui/src/update_prompt.rs` 中已有 `CODEX_CLI_DISPLAY_VERSION` 参与渲染；相关断言位于 `codex-rs/tui/src/app.rs`、`codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/chatwidget/tests.rs`、`codex-rs/tui/src/update_prompt.rs`。 | 升级提示应表现为 `当前 local1 版本 -> 新版本`，而不是丢失本地定制身份。 |
| F4 | `local1` 的测试与快照基线 | 所有和 `local1` 版本展示直接相关的 UI 出口，都要有快照或断言保护。 | `codex-rs/tui/src/status/snapshots/*`、`codex-rs/tui/src/chatwidget/tests.rs`、`codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/app.rs` 已有 `v0.0.0-local1` 或 `CODEX_CLI_DISPLAY_VERSION` 相关校验；`0.118.0` 中已不存在 `tui_app_server` 路径。 | 官方更新合并后，凡是版本展示链被冲掉，都能通过快照或断言第一时间暴露。 |
| F5 | 请求重试范围增强 | `Responses` 主链默认不再依赖 `402`、`429`、`5xx` 白名单；主链远端 HTTP 错误统一视为可重试，包含 `401`。与此同时，非 `/responses` 端点继续保留旧 whitelist：`402 usage-limit`、`429`、`5xx`、传输层错误。 | `codex-rs/codex-api/src/provider.rs` 中的 `responses_http_status_is_retryable(...)` 与 `should_retry_request_error(...)` 已把 `/responses` 请求层 HTTP 分类收敛为“所有 HTTP 状态统一重试”；同一函数里非 `/responses` 分支仍保留 `402 usage-limit`、`429`、`5xx`、transport retry 的旧判断。`codex-rs/codex-api/src/telemetry.rs`、`codex-rs/core/src/api_bridge.rs`、`codex-rs/core/src/error.rs` 共同把请求层与 turn 层分类绑定到统一主链口径。 | 合并官方更新后，`/responses` 的 `401` 与其他 HTTP 状态都仍应通过统一入口进入普通自动重试，至少要保留一个非 `401` 哨兵状态（如 `409` / `422`）和一个 `401` 哨兵状态用于复核；非 `/responses` 端点则仍保持旧 whitelist，不会被误放宽，也不会被误收窄成“什么都不重试”，且 `402 usage-limit` 与其他 `402` 仍被区分。 |
| F6 | 流式重试与 UI 提示联动 | `Responses` 主链的 retry/reconnect 中间态继续只走状态区与状态详情链，不进入历史记录；同一条主链上的 request / websocket OTEL log-trace 与 sampling reconnect warn 统一 suppress，但 `request_retry_notifier`、`will_retry = true`、`Reconnecting... N`、`additional_details` 和 metrics 继续保留。 | `codex-rs/codex-api/src/telemetry.rs` 已按 `/responses + attempt/是否继续主链` 计算 request 侧 `emit_log_trace`；`codex-rs/codex-api/src/endpoint/session.rs` 已显式透传 `path`；`codex-rs/core/src/client.rs` 已把 `emit_log_trace` 与 `retry_chain_active` 接到 request / websocket connect / websocket request / 失败型 websocket event；`codex-rs/otel/src/events/session_telemetry.rs` 已把 metrics 与 OTEL log-trace 拆开；`codex-rs/core/src/codex.rs` 已去掉 `stream disconnected - retrying sampling request ...` warn，同时保留 UI retry 事件链。 | 合并官方更新后，首个 websocket retry 仍可见，retry 详情字段不丢失，也不再退化成只有摘要文案；历史区仍然保持干净，不新增 retry 中间错误单元；`codex.api_request`、`codex.websocket_connect`、`codex.websocket_request` 与会驱动同一条 retry/reconnect 链继续前进的失败型 `codex.websocket_event` 都不应再为中间态落 OTEL log-trace，sampling reconnect warn 也不应再额外刷屏。 |
| F7 | 单次重试等待时间上限为 `10s` | 无论扩大到多少主链错误分类，单次指数退避或 `Retry-After` 等待都不应超过 `10s`。 | `codex-rs/core/src/util.rs` 中的 `MAX_RETRY_DELAY = 10s`、`clamp_retry_delay`、`retry_delay_for_error` 仍控制流式侧上限；`codex-rs/codex-client/src/retry.rs` 与 `codex-rs/codex-api/src/telemetry.rs` 仍通过 `backoff(...)` 对请求层退避做 `10s` clamp。 | 合并官方更新后，扩大的 `400`、`403`、`404` 等主链自动重试也仍受同一 `10s` cap 约束，不能因为全错误重试而突破上限。 |
| F8 | 重试次数保持“大次数或等效无界”目标 | 本次变化只扩展主链错误分类，不改变默认 request/stream budget 模式；bounded/unbounded 语义继续沿现有实现，`401` 也直接落在同一普通 retry budget 口径内，不再保留单独 recovery follow-up 层。 | `codex-rs/core/src/model_provider_info.rs` 仍区分 bounded/unbounded 运行模式；`codex-rs/codex-api/src/provider.rs` 的端点级 `with_retry_max_attempts(1)` 例外未改；`codex-rs/core/src/client.rs` 与 `codex-rs/core/src/codex.rs` 当前已去掉 `/responses` 主链上 `401` 的认证恢复优先特判，直接沿用统一普通 retry 链。 | 合并官方更新后，要继续把“统一错误分类”与“预算耗尽终态”分开看：bounded exhaustion 前仍是中间 retry，exhaustion 后才进入最终失败路径，不得把分类扩展误判成预算语义变化。 |
| F9 | 重试配置入口尽量统一 | `Responses` 主链的 retry 观测 suppress 真值必须统一收敛到同一主链分类入口：请求层、websocket connect、websocket request、失败型 websocket event、stream reconnect warn 都由同一套 retry/reconnect 分类驱动；route 真值必须从 `codex-rs/codex-api/src/endpoint/session.rs` 显式透传，不允许依赖 URL 猜测、散落的 ad-hoc `if should_retry` 或全局宏改写。 | 当前相关真值已主要收敛在 `codex-rs/codex-api/src/provider.rs`、`codex-rs/codex-api/src/telemetry.rs`、`codex-rs/codex-api/src/endpoint/session.rs`、`codex-rs/core/src/api_bridge.rs`、`codex-rs/core/src/client.rs`、`codex-rs/core/src/codex.rs`；其中请求层通过显式 endpoint 入参进入 `run_with_request_telemetry(...)`，websocket 侧通过 `retry_chain_active` 和同一主链错误分类决定 suppress。 | 合并官方更新后，新增主链远端错误仍应通过统一分类入口和统一详情透传链处理，不回退到散落的状态白名单、首个 retry 特判、额外 UI 摘要逻辑、局部 websocket 白名单，或通过修改 `shared.rs` 全局宏语义来一刀切静音。 |
| F10 | 线程 Provider 两字段刷新、全 retry 边界消费与 Windows 托盘批量刷新/替换 | 保留 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 两个入口；provider runtime 刷新仍只覆盖 `base_url` 与 `experimental_bearer_token` 两字段。空闲线程 refresh 立即应用；活跃线程收到 refresh 后也会立即更新 live runtime，且后续所有自动 retry 边界都会在发起下一次 retry 前吃到最新 refresh，至少覆盖 `turn-level retry`、`401 retry`、`websocket reconnect`、`websocket fallback to HTTP`、`request-layer internal retry`；若当前 attempt 尚未进入下一次 retry，则仍不承诺中途热切。refresh 重读最新 effective config 时，`[agents.*].config_file` 的相对路径仍必须相对于当前 user `config.toml` 解析。Windows 侧继续保留实例注册目录、named pipe 控制面与 tray 批量刷新链路，并新增 tray 退出按钮、provider 下拉以及“把当前 effective config 中所选 source provider 的 `base_url` / `experimental_bearer_token` 复制到当前 user config 中当前 `model_provider` 对应 provider 条目后立即 refresh”的能力；执行时不切换 `model_provider_id`；若目标条目不存在则明确报错不落盘。 | `codex-rs/core/src/codex.rs`、`codex-rs/core/src/client.rs`、`codex-rs/core/src/api_bridge.rs`、`codex-rs/codex-api/src/provider.rs`、`codex-rs/codex-api/src/endpoint/session.rs`、`codex-rs/codex-api/src/endpoint/responses.rs` 当前已把 active refresh 改为直接更新 live runtime，并把 request retry / `401 retry` / websocket reconnect / fallback 前的 provider/auth 取值改成按 attempt 从 live source 重读；`codex-rs/app-server/src/windows_control.rs`、`codex-rs/app-server/src/message_processor.rs`、`codex-rs/app-server/src/lib.rs` 已新增 `list_effective_providers` 与 `apply_provider_runtime_from_effective_provider` 控制面；`scripts/windows_app_server_refresh_tray.py` 已新增退出入口、provider 下拉、catalog 错误态、malformed registration 清理、多实例 catalog 一致性检测，以及 `Applied / Queued / Failed` 与 `config_write_failed / provider_parse_failed / provider_field_missing / request_failed` 分类反馈。 | 合并官方更新后，这两个 RPC 仍存在，且仍只刷新 `base_url` 与 `experimental_bearer_token`；活跃线程收到 refresh 后，后续 `turn-level retry`、`401 retry`、`websocket reconnect`、`websocket fallback to HTTP`、`request-layer internal retry` 都仍会在发起下一次 retry 前吃到最新 refresh；若当前 attempt 未进入下一次 retry，则仍不承诺中途热切。包含 `agents/reviewer.toml` 或 `./agents/reviewer.toml` 的合法配置在 `refresh` / `refreshAllLoaded` 下仍按 user `config.toml` 目录解析，不再把 `failedThreads` 污染为失败；`refreshAllLoaded` 仍把 `failedThreads = []` 视为成功，`totalThreads = 0` 或无 live instance 时 tray / bulk refresh 不误报失败。Windows 侧仍通过 `$CODEX_HOME/app_servers/*.json` + `\\\\.\\pipe\\codex-app-server-<instance_id>` + tray 脚本完成实例发现与批量刷新，并保留 Win32 `ctypes` prototype 绑定、stale registration 清理、一致性检测与最小 `codex-cli --bin codex --release` 构建链；tray 仍保留退出按钮，并仍可从 current effective config 的 provider 下拉复制两字段到当前 user config 中当前 `model_provider` 对应 provider 条目后立即 refresh，`model_provider_id` 不会被误切换；若 source provider 缺字段或目标条目不存在，则明确失败，不静默落盘。 |
| F11 | 历史默认不按 Provider 分割 | 本地历史列表在未显式传 provider 时默认可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / `--all` / `show_all` 语义的前提下，默认不再按 provider 过滤；CLI 与 embedded TUI 继续旧线程时仍使用当前 provider，remote TUI 维持现状，不自动切换到历史线程记录的 provider。 | `codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/exec/src/lib.rs`、`codex-rs/tui/src/lib.rs`、`codex-rs/tui/src/resume_picker.rs`、`codex-rs/tui/src/app_server_session.rs` 当前存在 provider 默认过滤、resume provider 注入差异或共享 helper 口径分流。 | 历史发现链路默认不再按 provider 收窄，但仍保留现有 cwd / `--all` / `show_all` 语义；continue/resume 仍按本文约定发起；`thread/list` 返回项仍保留 `model_provider` 字段。 |
| F12 | `Responses` stream / websocket 的 `401` 直接普通 retry 链 | `Responses` 主链上的 `401 unauthorized` 不再走单独的认证恢复优先分支；无论发生在初始请求层、turn 执行中的 stream / websocket 路径、websocket reconnect 还是 fallback 到 HTTP，`401` 都直接落入与其他 `/responses` HTTP 错误相同的普通 retry 链。当前这条链冻结的是统一普通 retry 控制流，不把 `request_id` / `cf-ray` / auth error headers 级 debug 上下文保真视为本轮既有定制能力。 | `codex-rs/codex-api/src/provider.rs` 已把 `/responses` 的 `401` 纳入统一可重试 HTTP 状态；`codex-rs/core/src/client.rs` 与 `codex-rs/core/src/codex.rs` 当前已去掉 `401` 的 unauthorized recovery 优先特判，使 request-layer、turn-level、websocket reconnect 与 fallback 都直接沿用普通 retry 链。 | 合并官方更新后，若 `/responses` 主链执行中出现 `401`，request-layer、stream / websocket、websocket reconnect 与 fallback 到 HTTP 都仍应直接走普通 retry；不能退化回“`401` 先做 unauthorized recovery”或“`401` 直接终止”。本轮验收默认只检查统一普通 retry 控制流是否仍成立，不把 header 级 debug/telemetry 上下文保真作为当前定制功能的通过条件。 |
| F13 | `gpt-5.4` 默认 priority 请求层兜底与顶层关闭开关 | 当 `/responses` 请求体构造出口检测到 `model_info.slug == "gpt-5.4"` 时，顶层 `config.toml` 中 `force_gpt54_priority_fallback` 省略或显式 `true` 时，outbound 请求继续强制序列化 `service_tier = "priority"`；显式 `false` 时，不再走硬编码 `priority`，同时 `Fast` 也不再透传，`Flex` 继续保留，`None` 继续 unset。该字段只允许出现在顶层 `config.toml`，不支持任何 `[profiles.*]` 版本。 | `codex-rs/core/src/config/mod.rs` 已新增顶层 `force_gpt54_priority_fallback` 并在运行时 `Config` 中落成默认 `true` 的布尔开关；`codex-rs/core/src/client.rs` 当前已把 `gpt-5.4` 的 `/responses` `service_tier` hook 改为读取该开关；`codex-rs/core/config.schema.json` 与 `codex-rs/core/src/client_tests.rs` 已同步新的顶层字段和 hook 语义。 | 同步官方后，`gpt-5.4` 发出的 `/responses` 请求在顶层开关省略或显式 `true` 时仍强制带 `priority`；显式 `force_gpt54_priority_fallback = false` 时，不再强制 `priority`，也不再向请求体透传 `fast`，但 `flex` 继续保留；非 `gpt-5.4` 不被误影响，且任何 `[profiles.*].force_gpt54_priority_fallback` 都不得生效。 |
| F14 | Windows app / app-server 默认日志降噪 | 未设置 `RUST_LOG` 时，Windows app 主链不再默认以高详细度写入 sqlite 日志；显式 `RUST_LOG` 时仍可恢复详细调试日志。该能力只改默认日志过滤，不改变 app-server 协议、线程生命周期或状态持久化语义。 | `codex-rs/app-server/src/lib.rs` 当前把 sqlite log layer 以 `TRACE` 级别默认挂到 tracing registry；Windows app 实际链路会走到该 app-server 路径。 | 同步官方后，Windows app 主链默认不再硬编码 `TRACE` 落盘；显式 `RUST_LOG` 覆盖能力仍在；线程、turn、review、approval、subagent 语义不发生回归。 |
| F15 | TUI 默认日志降噪 | `codex-tui.log` 与 TUI sqlite log layer 在未设置 `RUST_LOG` 时默认降噪；显式 `RUST_LOG` 时仍可恢复详细日志。该能力属于默认观测口径收敛，不删除文件日志或 sqlite 日志能力。 | `codex-rs/tui/src/lib.rs` 当前在 `RUST_LOG` 未设置时默认使用 `codex_core=info,codex_tui=info,codex_rmcp_client=info`，并把文件日志写入 `codex-tui.log`；`codex/docs/install.md` 也使用同一默认口径。 | 同步官方后，`codex-tui.log` 与 TUI sqlite 日志默认不再按旧 `info` 基线持续刷屏；显式 `RUST_LOG` 覆盖能力仍可用。 |

## 同步官方后的必查清单

- [ ] CLI 主入口帮助/版本输出与 TUI 用户可见版本链仍都带 `-local1` 后缀；即使 CLI 与 TUI 仍是两条常量链，也没有发生漂移。
- [ ] 状态卡片、状态区、标题区、历史单元的版本展示仍统一走 `CODEX_CLI_DISPLAY_VERSION`。
- [ ] 与 `local1` 相关的快照和断言没有被官方更新回滚掉。
- [ ] `/responses` 主链上的 HTTP 状态仍通过统一入口进入自动重试，包含 `401`。
- [ ] 至少一个非 `401` `/responses` 状态哨兵样例（如 `409` / `422`）与一个 `401` 哨兵样例仍符合自动重试口径，不会因为文档只举了几个例子而漏掉回归。
- [ ] 非 `/responses` 端点仍保持旧 whitelist：`402 usage-limit`、`429`、`5xx`、传输层错误仍按原口径处理，非 usage-limit 的 `402` 仍不会被误判成可重试。
- [ ] 新增主链远端错误仍通过统一分类入口处理，不回退到散落白名单判断。
- [ ] 单次退避上限仍然是 `10s`。
- [ ] “默认重试预算”和“端点级例外”仍然能被清楚区分。
- [ ] `Responses` retry 链不再产生 `codex.api_request` 中间态 OTEL log/trace。
- [ ] `Responses` websocket connect / reconnect retry 链不再产生 `codex.websocket_connect` 中间态 OTEL log/trace。
- [ ] `Responses` websocket request retry 链不再产生 `codex.websocket_request` 中间态 OTEL log/trace。
- [ ] `Responses` websocket retry / reconnect 链中的失败型 `codex.websocket_event` 不再产生中间态 OTEL log/trace。
- [ ] sampling reconnect `warn!` 不再刷屏。
- [ ] 首个 websocket retry 仍然可见。
- [ ] `Reconnecting... N`、retry 详情字段与 `additional_details` 仍然完整透传，不退化成只有摘要文案。
- [ ] 流式重试与请求重试仍然只更新状态提示，不往历史区写入脏错误记录；最终失败才进入终态错误路径。
- [ ] retry 中间态 metrics 仍然保留，没有因为 suppress log/trace/warn 被一起消掉。
- [ ] 非 `Responses` 端点与非 retry 终态日志没有被误 suppress。
- [ ] turn / websocket 执行过程里出现的 `401` 仍会直接走普通 retry；不会退化成直接终止，也不会退化回 unauthorized recovery 优先分支。
- [ ] 同步官方后，`gpt-5.4` 发出的 `/responses` 请求在顶层 `force_gpt54_priority_fallback` 省略或显式 `true` 时仍强制带 `priority`；显式 `false` 时不再强制 `priority`，也不再向请求体透传 `fast`，但 `flex` 继续保留；非 `gpt-5.4` 不被误影响，且任何 `[profiles.*].force_gpt54_priority_fallback` 都不得生效。
- [ ] Windows app 主链默认不再硬编码 `TRACE` 写入 sqlite 日志；显式 `RUST_LOG` 覆盖能力仍在，且 app-server 协议、线程生命周期、turn/review/approval/subagent 语义未被日志降噪改坏。
- [ ] `codex-tui.log` 与 TUI sqlite 日志默认不再按旧 `info` 基线持续写盘；显式 `RUST_LOG` 覆盖能力仍可用。
- [ ] `thread/providerRuntime/refresh`、`thread/providerRuntime/refreshAllLoaded`、`$CODEX_HOME/app_servers/*.json` 注册链、Windows named pipe 控制面与 `scripts/windows_app_server_refresh_tray.py` 仍然存在，且 provider runtime 刷新口径仍只覆盖 `base_url` 与 `experimental_bearer_token`；含 `[agents.*].config_file = "agents/..."` 或 `"./agents/..."` 的合法配置在 `refresh` / `refreshAllLoaded` 场景下仍按 user `config.toml` 目录解析。
- [ ] 活跃线程收到 refresh 后，后续 `turn-level retry`、`401 retry`、`websocket reconnect`、`websocket fallback to HTTP`、`request-layer internal retry` 都仍会在发起下一次 retry 前吃到最新 refresh；若当前 attempt 未进入下一次 retry，则仍不承诺中途热切。
- [ ] `refreshAllLoaded` 仍把 `failedThreads = []` 视为成功；`totalThreads = 0` 或无 live instance 时 bulk refresh / tray 仍给出非失败反馈，不会误报失败。
- [ ] Windows tray 仍保留 Win32 `ctypes` prototype 绑定、malformed/stale registration 清理、多实例 catalog 一致性检测与退出按钮，并仍可从当前 effective config 的 provider 下拉复制 `base_url` 与 `experimental_bearer_token` 到当前 user config 中当前 `model_provider` 对应 provider 条目后立即 refresh；若 source provider 缺字段、target 条目不存在或 catalog 不一致，则明确失败不落盘，`model_provider_id` 不会被误切换。
- [ ] tray 成功或部分成功反馈仍能区分 `Applied` / `Queued` / `Failed` 或等价状态；provider 列表生成失败、provider 不存在、字段缺失、配置写入失败、refresh 部分失败都有明确反馈；Windows app-server refresh/control 相关源码仍能通过最小 `codex-cli --bin codex --release` 构建链，不丢失类似 `PathBuf` 的基础依赖导入。
- [ ] 历史列表在未显式传 provider 时默认仍可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / `--all` / `show_all` 语义前提下默认仍不按 provider 过滤；CLI 与 embedded TUI 继续旧线程仍默认使用当前 provider；remote TUI 仍不从客户端注入 provider。
- [ ] `thread/list` 返回项仍保留 `model_provider` 字段，避免历史列表虽然跨 provider 可见，但 provider 身份信息被悄悄丢失。

## 暂不纳入本轮功能定义

- 官方 release 是否已经并入当前分支：这是同步状态问题，不是个人功能定义。
- 远端拓扑、`upstream/origin` 命名、长期分支命名：这是 Git 维护流程问题，不是功能本体。
- 临时调试代码、实验性改动、无关依赖变化：默认不算个人功能，除非后续你明确要求纳入。

## 本文使用方式

- 以后每次你要同步官方更新前，先看本文，确认哪些功能必须保留。
- 每次合并或 rebase 官方更新后，按“同步官方后的必查清单”逐项复核。
- 若后续你新增别的私人功能，直接往本文追加新的 `F16`、`F17` 等条目，不要把功能定义散落到聊天记录里。

## 2026-04-10 归档补充：首次对话清单与 gpt-5.4 priority 开关

- 本节用于归档 2026-04-10 新增并冻结的两项 local1 私有能力；它们已纳入后续同步官方与回归核对范围。
- 归档项 A1：首次对话固定清单展示。
  口径：TUI 与 app-server 路径在“新对话的第一条用户消息提交后”立即插入固定 local1 清单，替代随机启动提示；每个新对话只插入一次；同线程后续轮次、resume、continue、历史线程重开均不得重复插入；实现路线固定为内建主逻辑注入，不走 hook、启动 banner、外层包装器或旁路拦截链。该固定清单中还必须新增且只新增 1 条 refresh/retry + Windows tray 联动概述项，文案固定为：`- Provider refresh/retry 与 Windows tray 联动：active thread 在 refresh 后的后续自动 retry 会切到最新 \`base_url\` / \`experimental_bearer_token\`；Windows tray 新增退出入口，并支持从当前 effective config 的 provider 下拉复制两字段到当前 \`model_provider\` 对应 provider 条目后立即 refresh。`
- 归档项 A2：`force_gpt54_priority_fallback` 顶层开关。
  口径：该字段只允许写在顶层 `config.toml`；省略与显式 `true` 等价，默认继续保留 `gpt-5.4` 的 `/responses` `service_tier=priority` 强制兜底；显式写 `force_gpt54_priority_fallback = false` 时，不只是关闭这条 `gpt-5.4 priority` 请求层兜底，还要在当前请求构造 hook 位置一并关闭 `Fast` 透传；`Flex` 继续保留，`None` 继续 unset；任何 `[profiles.*]` 下的同名字段都不支持，也不得覆盖顶层配置。
- 归档要求：本节是这两项能力在基线 checklist 中的正式归档记录；后续若实现、回归或同步官方时出现口径冲突，以本节与对应 TASK 文档的冻结口径为准。
