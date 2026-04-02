# local1 定制功能清单（2026-03-28）

## 用途

- 本文用于冻结你希望长期保留在个人分支里的定制功能。
- 这不是官方需求文档，而是你自己的长期维护基线。
- 后续无论同步 `rust-v0.118.0` 还是继续追 `upstream/main`，都应以本文作为合并后的回归核对清单。

## 当前推断范围

- 基于当前仓库代码和前序对话，先把你的个人定制目标明确为两条主线：`local1` 显示链、统一重试增强链。
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
| F1 | `local1` 显示版本号 | 面向用户展示的 CLI/TUI 版本号默认不直接显示官方裸版本，而是显示 `<官方版本>-local1`。 | `codex-rs/tui/src/version.rs` 中存在 `CODEX_CLI_DISPLAY_VERSION`，值为 `concat!(env!("CARGO_PKG_VERSION"), "-local1")`；`codex-rs/cli/src/main.rs` 与 `codex-rs/tui/src/cli.rs` 的版本输出都已切到该常量。 | 合并官方更新后，用户可见版本入口仍统一显示 `-local1` 后缀。 |
| F2 | 卡片与状态区融入 `local1` | 状态卡片、顶部状态区、相关面板里出现的版本号全部走 `CODEX_CLI_DISPLAY_VERSION`，不混用原始包版本。 | `codex-rs/tui/src/status/card.rs`、`codex-rs/tui/src/app.rs`、`codex-rs/tui/src/chatwidget/status_surfaces.rs` 已引用该常量。 | 卡片、状态区、标题区的版本展示口径保持一致，不出现一处 `local1`、一处官方裸版本的分裂。 |
| F3 | 历史单元与升级提示融入 `local1` | 历史消息、升级提示、版本跳转提示等文本，也必须展示 `local1` 版本名。 | `codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/chatwidget.rs`、`codex-rs/tui/src/update_prompt.rs` 中已有 `CODEX_CLI_DISPLAY_VERSION` 参与渲染；相关断言位于 `codex-rs/tui/src/app.rs`、`codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/chatwidget/tests.rs`、`codex-rs/tui/src/update_prompt.rs`。 | 升级提示应表现为 `当前 local1 版本 -> 新版本`，而不是丢失本地定制身份。 |
| F4 | `local1` 的测试与快照基线 | 所有和 `local1` 版本展示直接相关的 UI 出口，都要有快照或断言保护。 | `codex-rs/tui/src/status/snapshots/*`、`codex-rs/tui/src/chatwidget/tests.rs`、`codex-rs/tui/src/history_cell.rs`、`codex-rs/tui/src/app.rs` 已有 `v0.0.0-local1` 或 `CODEX_CLI_DISPLAY_VERSION` 相关校验；`0.118.0` 中已不存在 `tui_app_server` 路径。 | 官方更新合并后，凡是版本展示链被冲掉，都能通过快照或断言第一时间暴露。 |
| F5 | 请求重试范围增强 | 个人定制默认目标不是只保留官方最保守的请求重试，而是显式维护你自己的可重试口径。当前最少应覆盖 `402`、`429`、`5xx`、传输层错误；若你希望把 `427` 之类状态也纳入，则必须在统一口径里明确写死，不允许散落在零星分支判断中。 | `codex-rs/codex-api/src/provider.rs` 已有 `retry_402`、`retry_429`、`retry_5xx`、`retry_transport` 配置入口；`codex-rs/codex-client/src/retry.rs` 中已按请求层策略消费这些开关。 | 合并官方更新后，先核对哪些状态码仍被视为可重试；若新增 `427` 等状态，必须有统一配置或统一判断入口。 |
| F6 | 流式重试与 UI 提示联动 | 流式请求发生短暂失败时，允许自动重试，同时在 UI 中给出明确但不污染历史记录的重试提示。 | `codex-rs/core/src/client.rs` 通过 `RequestRetryNotifier` 把请求层重试事件上报到 `codex-rs/core/src/codex.rs`；`codex-rs/tui/src/chatwidget/tests.rs` 已覆盖 `402`、`429` 重试状态提示。 | 重试提示应更新状态区，不应额外生成错误历史单元；用户能看见正在重连，但历史记录保持干净。 |
| F7 | 单次重试等待时间上限为 `10s` | 无论指数退避还是服务端返回更大的 `Retry-After`，单次等待都不应超过 `10s`。 | `codex-rs/core/src/util.rs` 中存在 `MAX_RETRY_DELAY = 10s`、`clamp_retry_delay`、`retry_delay_for_error`；`codex-rs/codex-client/src/retry.rs` 中请求退避也做了 `10s` clamp。 | 合并官方更新后，`backoff`、`Retry-After` 与 `clamp_retry_delay` 仍应保持 `10s` 上限。 |
| F8 | 重试次数保持“大次数或等效无界”目标 | 你的长期目标是：面向真实使用流程的默认重试预算应当足够大，或者等效无界；如果某些端点出于官方策略仍然是 `max_attempts = 1`，必须把它们视为“显式例外”，不能把例外误当成整体默认行为。 | `codex-rs/core/src/model_provider_info.rs` 已把默认请求重试次数与流式重试预算区分为 bounded/unbounded 两种运行模式；`codex-rs/codex-api/src/provider.rs` 提供 `with_retry_max_attempts(1)`，并由 `codex-rs/codex-api/src/endpoint/models.rs`、`memories.rs`、`realtime_websocket/methods.rs` 在运行时显式收紧这些端点的重试上限。 | 后续要把“整体默认口径”和“端点级例外”分开定义清楚；验收时不能只看单个端点就误判成“全局无限重试”已经成立。 |
| F9 | 重试配置入口尽量统一 | 请求重试、流式重试、状态提示、退避时间、端点例外，尽量通过集中定义维护，减少未来官方更新后难以找回本地定制差异的问题。 | 当前相关真值主要收敛在 `codex-rs/codex-api/src/provider.rs`、`codex-rs/codex-client/src/retry.rs`、`codex-rs/core/src/model_provider_info.rs`、`codex-rs/core/src/util.rs`、`codex-rs/core/src/codex.rs`。 | 后续若继续收敛实现，目标是让“重试策略真值”尽量集中，可快速审计、快速回归。 |
| F10 | 线程 Provider 两字段刷新与 Windows 托盘批量刷新 | 保留 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 两个入口；当前 thread 只允许热刷新 provider 运行态中的 `base_url` 与 `experimental_bearer_token`，空闲线程立即应用，非空闲线程只挂起到下一轮；Windows 额外保留实例注册目录、named pipe 控制面与托盘脚本批量刷新链路。 | `codex-rs/core/src/codex.rs`、`codex-rs/core/src/client.rs`、`codex-rs/core/src/thread_manager.rs` 已接入 session 级待应用刷新与 `ModelClient` 两字段 runtime mutator；`codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/app-server/src/windows_control.rs`、`codex-rs/app-server/README.md` 已暴露 RPC、实例注册和 Windows control pipe；`scripts/windows_app_server_refresh_tray.py` 已固定为系统 Python 直接运行托盘脚本。 | 合并官方更新后，这两个 RPC 仍存在，且仍只刷新 `base_url` 与 `experimental_bearer_token`；活跃线程仍是 queued 到下一轮；Windows 侧仍通过 `$CODEX_HOME/app_servers/*.json` + `\\\\.\\pipe\\codex-app-server-<instance_id>` + 托盘脚本完成实例发现与批量刷新。 |
| F11 | 历史默认不按 Provider 分割 | 本地历史列表在未显式传 provider 时默认可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / `--all` / `show_all` 语义的前提下，默认不再按 provider 过滤；CLI 与 embedded TUI 继续旧线程时仍使用当前 provider，remote TUI 维持现状，不自动切换到历史线程记录的 provider。 | `codex-rs/app-server/src/codex_message_processor.rs`、`codex-rs/exec/src/lib.rs`、`codex-rs/tui/src/lib.rs`、`codex-rs/tui/src/resume_picker.rs`、`codex-rs/tui/src/app_server_session.rs` 当前存在 provider 默认过滤、resume provider 注入差异或共享 helper 口径分流。 | 历史发现链路默认不再按 provider 收窄，但仍保留现有 cwd / `--all` / `show_all` 语义；continue/resume 仍按本文约定发起；`thread/list` 返回项仍保留 `model_provider` 字段。 |

## 同步官方后的必查清单

- [ ] `CODEX_CLI_DISPLAY_VERSION` 仍然存在，且仍带 `-local1` 后缀。
- [ ] 状态卡片、状态区、标题区、历史单元的版本展示仍统一走 `CODEX_CLI_DISPLAY_VERSION`。
- [ ] 与 `local1` 相关的快照和断言没有被官方更新回滚掉。
- [ ] `402`、`429`、`5xx`、传输层错误的重试口径没有被官方代码覆盖掉。
- [ ] 若要支持 `427` 或其他额外状态，统一入口仍然存在，且不是散落补丁。
- [ ] 单次退避上限仍然是 `10s`。
- [ ] “默认重试预算”和“端点级例外”仍然能被清楚区分。
- [ ] 流式重试仍然只更新状态提示，不往历史区写入脏错误记录。
- [ ] `thread/providerRuntime/refresh`、`thread/providerRuntime/refreshAllLoaded`、`$CODEX_HOME/app_servers/*.json` 注册链、Windows named pipe 控制面与 `scripts/windows_app_server_refresh_tray.py` 仍然存在，且 provider runtime 刷新口径仍只覆盖 `base_url` 与 `experimental_bearer_token`。
- [ ] 历史列表在未显式传 provider 时默认仍可读到所有 provider 线程；CLI 与本地 TUI 的最近会话、resume picker 在保留现有 cwd / `--all` / `show_all` 语义前提下默认仍不按 provider 过滤；CLI 与 embedded TUI 继续旧线程仍默认使用当前 provider；remote TUI 仍不从客户端注入 provider。

## 暂不纳入本轮功能定义

- 官方 release 是否已经并入当前分支：这是同步状态问题，不是个人功能定义。
- 远端拓扑、`upstream/origin` 命名、长期分支命名：这是 Git 维护流程问题，不是功能本体。
- 临时调试代码、实验性改动、无关依赖变化：默认不算个人功能，除非后续你明确要求纳入。

## 本文使用方式

- 以后每次你要同步官方更新前，先看本文，确认哪些功能必须保留。
- 每次合并或 rebase 官方更新后，按“同步官方后的必查清单”逐项复核。
- 若后续你新增别的私人功能，直接往本文追加新的 `F10`、`F11` 等条目，不要把功能定义散落到聊天记录里。
