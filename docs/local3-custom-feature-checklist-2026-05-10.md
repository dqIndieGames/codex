# local3 定制功能与回归验收清单

本清单用于合并官方上游版本后，核对 local3 定制功能是否保留，并指导后续实现和验收。

- **功能摘要**：说明用户能感知到的结果。
- **详细规则**：明确实现边界和验收标准。
- **历史经验**：保留故障原因、定位入口和回归陷阱。
- **配置快照**：记录核对时的本机设置，不替代长期规则。

历史经验中的“当时已验证”只代表当时记录，不代表当前版本已经通过验收。

## 一、用户可见功能

程序会提取本节的编号行，生成首次“你好”清单。保留连续的 1–21 项，每项正文保持一个物理行；其他章节不使用 `数字. ` 列表。

1. **local3 版本身份。** CLI、TUI、doctor、历史记录和其他展示本地构建身份的位置显示 `<Codex 版本>-local3`；发往服务端的版本信息按第 21 项处理。

2. **首次“你好”清单。** 全新线程或 Clear 后的新线程，首个普通用户输入恰好为纯文本 `你好` 时，在首个 assistant 主消息开头显示完整功能清单，每个新线程只显示一次。

3. **主链自动重试。** 会终止对话、上下文压缩或 realtime 模型链路的远端请求错误均进入自动重试，每次失败后固定等待 `10s`；单次超时只结束当前尝试，下一次重新获得完整时间额度。

4. **重试可见且不污染历史。** 用户能看到持续累计的重试次数和安全诊断信息；中间失败只作临时状态展示，不写入历史、fork、replay 或普通错误日志。

5. **跨 provider 继续历史会话。** 历史入口默认可发现不同 provider 的旧会话；继续旧线程时使用当前顶层 provider，无法换绑时明确提示仍在使用旧 provider。

6. **全局优先服务层。** 默认使用 priority；顶层显式关闭后恢复官方 Fast / Flex / None 映射，profile 内同名设置不生效。

7. **默认日志降噪。** Windows App、app-server 和 TUI 默认减少日志；analytics、feedback、log_db 默认关闭，仍允许显式开启。

8. **批量优化与历史安全。** 默认开启会话记录批量写盘和 app-server 高频通知合并，保持回答、用量、改动、计划及命令状态及时更新，崩溃恢复可靠，并保留关闭开关。

9. **Provider 刷新覆盖全部入口。** 地址、token、优先服务层和 fast mode 有效配置变化后，刷新 app-server、已打开会话、exec、subagent 和 agent_jobs 等运行入口。

10. **Provider 刷新无需等待报错。** 配置变化后，下一次请求使用新配置；地址或凭据变化会中断旧连接及其等待、重试，并重新建立连接，同地址换 token 也生效。

11. **连接恢复保持原协议。** 选用 WS 的模型请求失败后继续通过 WS 重试，不自动切 HTTP；连续重试每满 3 次，按实际地址执行适用的中转粘连恢复，保留累计序号、必要续链和已完成工具结果。

12. **保存与刷新结果分开反馈。** Provider 字段保存成功但没有运行实例可刷新时，仍视为保存成功，并明确提示“未刷新任何实例”。

13. **app-server stderr 默认安静。** 后台日志和 WebSocket 启动诊断默认不输出；设置 `[logging] app_server_stderr = true` 后恢复诊断，真正启动失败和配置错误仍显示必要信息。

14. **node_repl 使用当前 CLI。** 启动 `node_repl` MCP 时，将 `CODEX_CLI_PATH` 指向当前 `Config.codex_self_exe`，避免工具子进程误用旧版 Codex；其他 MCP 环境不被改写。

15. **退出时安全释放引用。** app-server 退出时调用已有 runtime 引用清理，不新增空闲超时，不全局扫描或结束进程，不因 UI 断开而杀掉仍加载的线程。

16. **Provider token 隔离。** 非空 `experimental_bearer_token` 优先用于该 provider 的请求，不继承全局登录态；没有该 token 时继续使用原 AuthManager 行为。

17. **Windows 云端构建与发布。** 本地不编译；先下载 GitHub build artifact 验证真实 exe，再由 GitHub promotion workflow 发布 Release，禁止本地上传资产。

18. **Context window 图片梯子。** 普通模型请求连续溢出时，按四档逐步降低旧图成本；变化以轻量记录持久化，resume / fork 可重建，不修改磁盘原图，四档用尽后仍继续自动重试。

19. **命名账号只隔离登录凭据。** `--account <账号名>` 为每个账号使用独立认证文件，配置、会话、日志、skills 和 MCP 继续共享；支持多个命名账号同时运行。

20. **每次 401 重读所选认证文件。** 实际凭据来自文件时，每次 HTTP 401，包括 WebSocket 握手失败，都重新读取所选账号的 `auth.json`；无效文件不覆盖缓存，非文件凭据不受影响。

21. **服务端 User-Agent 使用裸版本。** 发往服务端的 HTTP `User-Agent` 使用当前 Cargo 包版本，不追加 `-local3`；本机展示版本和 app-server initialize 返回本机的 `user_agent` 仍带后缀。

## 二、版本身份与首次清单

### 2.1 版本按用途区分

| 用途 | 版本格式 |
|---|---|
| 本机或客户端展示 local3 构建身份 | `<当前包版本>-local3` |
| 发往服务端的 HTTP `User-Agent` 版本段 | 当前 Cargo 裸包版本 |
| `/models` 的 `client_version` | 同一裸包版本 |
| 更新比较、Python wheel、配置锁、OpenTelemetry `service_version`、OAuth / device-code 协议参数 | 裸 semver |

展示面必须覆盖：

- CLI、TUI、状态卡、标题区、历史单元和升级提示。
- `codex --version`、`codex doctor --json`、doctor runtime details、`codex-app-server --version`。
- app-server initialize 返回本机的 `user_agent`。
- daemon / remote-control JSON、device-code 登录欢迎文案。
- 线程历史元数据和 rollout 会话记录元数据。

`cli_version`、`client_version`、`app_server_version` 按实际用途选择版本，不能仅凭字段名统一替换。

发往服务端的 HTTP `User-Agent`：

- 从当前 Cargo 包版本生成，不写死历史版本号。
- WebSocket 握手、登录、ChatGPT 辅助接口和云任务统一执行。
- `originator`、操作系统、架构、终端标识和官方原有后缀保持原样。

**历史经验｜2026-05-31**

- daemon 会解析 initialize 的 `user_agent`，再用于 doctor 的 app-server 版本展示。这里使用裸 `CARGO_PKG_VERSION`，会造成前台显示 local3、后台却像官方版本。
- Windows smoke 必须明确检查 `-local3`。只匹配裸版本号，无法发现后缀丢失。
- Python wheel 版本不能取 `GITHUB_REF_NAME`。从 `main` 手动触发时会得到不符合 PEP 440 的版本，应读取 Cargo 裸版本。

### 2.2 首次“你好”的触发范围

只有同时满足以下条件才触发：

- brand-new thread，或 Clear 后的新线程。
- 首个普通用户输入。
- 输入恰好是纯文本 `你好`。

清单出现在首个 assistant 主消息开头，每个新线程一次。

以下情况不触发：

- 同线程再次输入 `你好`。
- resume、continue、fork、历史线程重开或子会话。
- 其他文本、多段输入、富文本或带附件输入。

### 2.3 清单格式约束

当前解析器逐行提取 `N. `，并要求编号连续、有序。

- 第一节保留连续的 1–21 项。
- 每项内容放在一个物理行。
- 其他章节使用标题、无序列表或表格，避免被误提取。
- 图片梯子必须保留为独立第 18 项，不能只藏在 retry 子条中。

## 三、重试、超时与连接恢复

### 3.1 重试范围和固定等待

这里的 sampling 指普通模型请求；compact 指压缩长会话上下文；retry budget 指允许的重试次数。

强制重试范围包括：

- 普通 sampling。
- Responses HTTP request retry。
- SSE / WebSocket 流式重连。
- local compact、旧 remote compact `/responses/compact`、remote compaction v2。
- realtime WebSocket connect、WebRTC sideband join。
- TUI、exec、subagent、agent_jobs 中对应的模型请求入口。

远端主链错误不能因 `is_retryable=false`、状态码、错误码或协议映射被排除。范围包括：

- HTTP 503 / 502 / 504 / 429 / 402。
- 网络失败、临时鉴权错误、未完成断流、握手失败和请求超时。
- `Selected model is at capacity. Please try a different model.`
- context window、usage / quota、policy 等服务端错误。

`/models` 刷新和 MCP OpenAI 文件上传 / 下载 URL 生成属于旁路请求，不纳入本清单强制 retry / sticky-break 范围；涉及鉴权和用户错误提示时，仍遵守对应规则。

**所有主链自动重试在当前尝试失败后，固定等待 `10s`。**

- 不使用指数退避、jitter、增长型 `base_delay` 或仅设上限的 cap-only。
- 服务端 `Retry-After` 不改变这 `10s`。
- 正常请求不额外等待。
- 默认允许无界或非常大的重试次数。
- 显式 bounded 或 `stream_max_retries = 0` 表示用户主动限制或禁用重试，不能替代默认体验验收。
- 显式次数限制只控制重试预算，不触发 WS → HTTP。

### 3.2 流式超时读取配置

模型首事件等待和后续流式空闲，均使用 provider 的 `stream_idle_timeout_ms`。

| 项目 | 规则 |
|---|---|
| 显式配置 | 按配置值执行 |
| 未配置 | 默认 `300000` 毫秒，即 300 秒 |
| 首事件等待 | 使用该配置 |
| 后续流式空闲 | 使用同一配置；有模型进展后重新计时 |
| 持续输出 | 不按生成总时长中断 |
| Responses HTTP 等待响应头 | 不设置 local3 固定等待看门狗 |

300 秒是默认值，不是强制上限。不能再叠加 60 秒或 390 秒硬上限截短流式配置。

首次请求和每次重试均获得完整额度：

- 前次耗时、重试等待、provider refresh 和粘连恢复不扣减下一次额度。
- 超时只取消当前尝试，显示临时状态，等待 `10s` 后重试。
- RST / 读失败进入自动重试。
- 用户取消和 provider refresh 仍能中断等待。
- 提示显示实际生效时长，不写死“一分钟”或“6.5 分钟”。
- 超时和恢复不清零用户可见序号。

### 3.3 建连超时单独处理

建连与流式等待是不同阶段，不能混成一个固定超时。

| 路径 | 超时规则 |
|---|---|
| Responses WebSocket 建连 | 读取 `websocket_connect_timeout_ms`；默认 `15000` 毫秒，上限 `390000` 毫秒 |
| realtime WebSocket 建连 | 保留现有 `390s` 规则 |
| WebRTC HTTP 请求 | 使用 provider 的 `first_model_event_timeout`；当前其来源是 `stream_idle_timeout_ms` |
| WebRTC sideband WebSocket | 按 realtime 连接规则处理 |

compact 按实际使用的请求方式和阶段验收。使用 Responses 流式请求时，按流式配置执行，不能把所有 compact 一概写成固定 390 秒。

### 3.4 WS 失败后仍使用 WS

适用于选用 Responses WebSocket 的 sampling、local compact 和 remote compaction v2。

- 握手失败、断流、超时或连续失败后，继续通过 WS 重试。
- 允许关闭失效连接并重新建立 WS。
- 连续失败次数、有限预算耗尽或服务端拒绝 WS，都不能触发自动切 HTTP。
- 本会话后续请求继续使用 WS。
- 已完成工具结果必须保留，不能因连接恢复而重复执行。
- 必要的 `previous_response_id` 续链语义保持完整。
- 用户取消立即生效。
- 重连过程只展示临时状态，累计序号不归零。

官方和中转均执行“WS 保持 WS”。原本使用 HTTP 的独立请求继续按其 HTTP 规则执行。

关闭连接的诊断可保留协议 close code；原始 reason 不直接显示或落盘，避免泄漏服务端回传内容。

**历史说明｜2026-09-29**

旧策略曾规定“连续 3 次 WS 失败后切 HTTP”。本稿取消该策略，保留失效连接释放、状态恢复、工具续链、计数和历史隔离要求。

### 3.5 中转粘连恢复：每连续 3 次检查一次

sticky-break 指清除 Codex 可控的中转粘连信号；recovery generation 是内部恢复轮次标记。

在第 3、6、9、12 次等连续失败处理阶段，先检查**完成认证和路由解析后的实际请求 URL / base_url**。

| 实际目的地 | 处理 |
|---|---|
| ChatGPT 官方 Codex 后端，例如 `https://chatgpt.com/backend-api/codex` 及其子路径 | 跳过中转粘连转换 |
| 外部中转或 relay provider | 执行适用的 sticky-break |

未显式填写 `base_url` 的官方 provider，也必须按解析后的实际地址判断。

中转 Responses 的可全量重放请求，在第 3 次失败处理阶段：

- 从默认 thread id 派生带 recovery generation 的新 `prompt_cache_key`。
- 清空 Codex 缓存的 `x-codex-turn-state`。
- 重置 WebSocket session，并仍通过 WS 建立新连接。
- 让紧随其后的请求使用新 generation，不携带旧 turn-state 和旧 `previous_response_id`。

边界：

- 不改变真实 thread id、session id 或用户提示词。
- Codex 只能清除自己控制的信号，不能保证中转一定换号。
- `function_call_output` 等必要续链请求仍须重试并参与计数，但必须保留必要续链 ID，或采用不破坏续链的恢复方式。
- 官方地址不为 sticky-break 旋转 cache key、清空 turn-state、强制重置 session 或丢弃续链 ID。
- 官方连接失效时仍可正常重建 WS，这与中转粘连转换分开处理。

### 3.6 compact 和 realtime 的独立入口

**compact：长会话的上下文压缩**

local compact、旧 `/responses/compact`、remote compaction v2 分别验收：

- 默认模式必须重试。
- 每连续 3 次失败按实际 URL 判断是否执行中转粘连恢复。
- 不能因 compact 专属预算或默认配置，在第 3 次之前提前终态。
- 显式 bounded / zero 仍按用户设置处理。
- 使用 WS 的压缩请求保持 WS。
- 旧 `/responses/compact` 的 request retry 必须展示临时 `503 retry N` 或 `Reconnecting... N`。
- 只证明第四次请求 cache key 变化，不足以证明前三次提示和历史隔离正确。

文中的 **remote compaction v2 是压缩实现版本，不是 WS 协议版本**。

**realtime：实时语音等交互连接**

它与普通文字对话的 Responses WS 是不同入口。

分别检查：

- ordinary WebSocket connect：直接建立 realtime WS。
- WebRTC sideband join：为 WebRTC 实时连接建立配套控制通道。

要求：

- 使用 provider retry 配置和固定 `10s` 等待。
- 支持 provider refresh 中断旧连接。
- 第 3、6、9 次连接 / 握手失败时，递增内部恢复轮次，丢弃半开连接，重建握手 request、header 和 TLS connector。
- 用户看到 `Reconnecting realtime... N` 和恢复阶段提示。
- 临时提示不写历史。
- 真实 thread id、用户输入和用户显式 realtime session id 不变。

realtime 没有 Responses 的 `prompt_cache_key` / `previous_response_id` 字段，不能照搬这些字段的恢复方式去修改用户 session id。

### 3.7 提示、日志和历史隔离

重试中间态只承担：

- 必要的请求与等待。
- 一次轻量临时状态通知。
- TUI 同一状态栏覆盖刷新。
- metrics / counter 计数。

不得进入 rollout、history、fork、replay 或模型上下文。

不能继续通过普通持久化 `EventMsg::StreamError` / `EventMsg::Warning` 链路保存中间失败，再把“界面能看到提示”当作验收通过。

默认不逐次写普通 `warn!` / `error!` 或 app-server stderr。底层不能在上层决定继续重试前抢先记录错误。

允许最终失败、显式 debug / trace 和低频汇总诊断，内容必须脱敏。

用户提示：

- 默认无界或非常大次数模式显示 `503 retry N (auto retry)`、`Reconnecting... N (auto retry)`。
- 显式有限预算可显示当前次数 / 上限。
- 不显示 `18446744073709551615`、`(unbounded)` 或 `(10 min limit)`。
- 标题说明状态和次数。
- 详情说明状态含义、正在重试及安全诊断信息。
- `http 429`、`http 503` 等 telemetry 短串不能代替用户详情；可使用 `HTTP 503 Service Unavailable, retrying`。

安全诊断字段限于：

- HTTP 状态码和标准 reason。
- 去除 query / userinfo 的 endpoint。
- request id、cf-ray。
- 经脱敏的 auth error / auth error code。

不原样展示 HTTP response body。

**显示经验｜2026-05-30、2026-05-31**

HTTP request retry 和 stream / WS reconnect 是不同链路。调整其中一种提示，不能顺手隐藏另一种；app-server、TUI、Windows App 都须继续展示必要重试状态。

### 3.8 计数和回归经验

**计数分离｜2026-06-28**

- 用户可见累计序号与内部 recovery 计数分离。
- sticky-break、超时、重建 HTTP client、重置 WS session 和 provider refresh，均不得让可见序号回到 0 或 1。
- HTTP telemetry 需要累计已被 route recovery 消费的 retry offset。
- 只减少下一轮 `max_attempts` 不够；新 client 的 `on_request_retry(1, ...)` 仍会造成显示回绕。
- stream / WS 的内部 `retries` 用于恢复周期；独立 display retry 用于展示。
- 计数变化不能改变固定 `10s` 等待，也不能触发切换 HTTP。

**旧等待口径残留｜2026-07-03**

当时曾因旧实现、测试名、断言和文档互相强化，留下 `8s` 或 cap-only 规则。该经验继续保留，但本稿目标已改为固定 `10s`。

调整等待规则时检查：

- `E:\vscodeProject\codex_github\codex\codex-rs\core\src\util.rs`
- `E:\vscodeProject\codex_github\codex\codex-rs\core\src\util_tests.rs`
- `E:\vscodeProject\codex_github\codex\codex-rs\codex-client\src\retry.rs`，包括测试区。

排查仍被当作现行 retry 规则的：

- `5s`、`8s/eight seconds`。
- `<=5s`、`最高 5s`，以及仅设最大值的等价表达。
- 退避、jitter、增长型 `base_delay`。
- “连续失败后切 HTTP”的实现、测试和文档。

历史数字可保留，但必须标明已废止，不能继续作为通过标准。

**主链遗漏｜2026-06-14、2026-07-03**

- capacity 错误须以 typed `ServerOverloaded` 进入 retry / recovery；只改 UI 或 `is_retryable()` 不够。
- sampling、local compact、remote compaction v2 都可能把错误转为终态 `ErrorEvent`，须逐条覆盖。
- compact 必须同时证明“会重试”和“每 3 次按实际 URL 判断恢复”。
- Responses 通过不能代替 realtime、WebRTC sideband 和旧 remote compact 验收。

## 四、Provider 刷新、鉴权与服务层

### 4.1 刷新字段和入口

刷新字段：

- `base_url`
- `experimental_bearer_token`
- `force_service_tier_priority`
- fast mode 相关有效配置

刷新入口：

- 所有 app-server 和已加载线程。
- 已打开的 Codex 窗口、TUI / console 会话。
- `codex exec`。
- 已打开及后续新开的 subagent。
- agent_jobs 批量子任务。
- Windows tray 从 source provider 向 target provider 应用配置。

刷新不依赖报错或 retry。配置变化后应尽快更新运行时，下一次请求必须使用新配置。

地址或凭据变化时：

- 使缓存和活动 WS 失效。
- 中断旧连接的等待和重试。
- 使用新配置重新握手。
- 同地址只换 token 也执行上述流程。
- 保留已完成工具结果，不重复执行。
- 新配置仍选用 WS 时，重建后继续使用 WS。

只保存配置、清 plugin / skill cache 或清共享缓存，不代表 loaded thread 已刷新。

### 4.2 控制面、范围和反馈

Windows tray 优先调用 `apply_provider_runtime_from_effective_provider`，由实际运行的 app-server 完成：

- 读取 effective config。
- 写入配置。
- reload user config。
- 刷新 loaded threads。

只有**所有 live instance 都明确不支持**该控制操作时，才回退到 Python 修改 `config.toml`，再调用 `refresh_all_loaded_threads`。

保留全量刷新，同时支持 `console` 和 `appServer` scope：

- `appServer` 能刷新 Windows App 的 app-server thread。
- `console` 不误刷 app-server thread。

结果分别报告：

- 配置是否保存成功。
- 是否刷新到运行实例。

没有 live instance 时，反馈“配置已保存，但未刷新任何实例”。

**历史经验｜2026-05-30、2026-06-03**

IFEO、wrapper、runtime selector 或 Windows App 可能重定向实际 exe。绕过运行中的 app-server 直接改配置，容易出现“文件已改，当前会话仍使用旧 URL / token”。

动态验证须覆盖 HTTP 503 / 429 / 402、无界 503、网络失败、SSE 断流 / 空闲、WS 503 / 426 / 401。刷新后旧 endpoint / token 的请求不能继续增长。

### 4.3 Provider token 隔离

非空 `experimental_bearer_token` 是 provider 自带的静态 bearer token，优先于 `env_key` 和 AuthManager。

覆盖聊天、compact、HTTP / WS、realtime、`/models` 及 refresh 后下一次请求：

- 使用 `Authorization: Bearer <experimental_bearer_token>`。
- 不继承 AuthManager 的 auth mode、账号 ID、ChatGPT routing、FedRAMP 或 attestation。
- 空字符串不生成 `Bearer `。
- 无 token provider 保留原 AuthManager 行为。

必须支持“有 provider token”与“无 provider token”双向刷新：

- 无 → 有：进入 provider token 隔离模式。
- 有 → 无：清除旧 provider token 状态，恢复 AuthManager / `auth.json`。
- `openai_http ↔ yunyi` 只是代表样例，不能写死为特例。

**实现经验｜2026-07-01**

- 只改 `resolve_provider_auth()` 的 Authorization 不够。
- 还须处理 `ConfiguredModelProvider::auth()`、`OpenAiModelsEndpoint::auth()`、`supports_attestation()` 等全局认证读取点，防止 `api_provider()`、默认地址和 `/models` 间接受污染。
- 推荐有非空 provider token 时，在读取点将当前 provider auth 视为 `None`，再由 provider 配置生成 `BearerAuthProvider`。
- 不要全局丢弃 AuthManager 引用，否则移除 token 后无法恢复原认证。
- 即使全局保存了 ChatGPT token、API key、account id 或 FedRAMP 状态，隔离请求也不能夹带其派生 header。

### 4.4 全局优先服务层

| 顶层设置 | 行为 |
|---|---|
| 未配置或开启 | 统一使用 priority |
| 显式关闭 | Fast → priority；Flex → flex；None → unset |

profile 内同名设置不生效。Provider refresh 必须同步相关有效配置。

## 五、历史会话与账号认证

### 5.1 换 provider 后继续旧会话

例如：昨天使用中转 A，今天切换到官方 provider，再打开昨天的会话。用户应能找到旧会话，并通过当前 provider 继续工作。

历史列表、recent sessions、resume picker、resume last 和 `codex://threads/{id}` 默认不按 provider 过滤。

继续旧线程时，不能被以下旧状态锁住：

- `session_meta.model_provider`。
- loaded `config_snapshot.model_provider_id`。
- `SessionThreadConfig.model_provider`。
- `thread/read` 回退或已加载线程快照。

无法换绑时，明确提示仍使用旧 provider。

### 5.2 resume 与 fork 分开验收

- **resume**：继续原会话，使用当前顶层 provider。
- **fork**：从旧会话派生新会话；本地 fork picker / fork last 仍按当前 provider 过滤。

跨 provider 继续旧线程，不扩大为跨 provider 派生新线程。

**恢复经验｜2026-06-14**

- app-server resume 的显式 request provider 必须覆盖历史 provider。
- TUI 同一线程的 active provider 不同时，先 shutdown，再 cold resume / rebind。
- 不能只验证列表能选中，必须验证下一次请求实际使用的 provider。

### 5.3 命名账号

| 账号 | 认证文件 |
|---|---|
| 默认账号 | `$CODEX_HOME/auth.json` |
| `--account <账号名>` | `$CODEX_HOME/accounts/<账号名>/auth.json` |

只有认证文件隔离。配置、会话、日志、skills 和 MCP 继续共享，不复制、不分叉。

`login`、`login status`、token 刷新、`logout`、TUI 和本地 embedded app-server 必须使用同一所选账号。

命名账号要求：

- 不覆盖默认账号或其他账号的认证文件。
- 可并发运行多个命名账号。
- 不隐式复用仅持有单一 AuthManager 的共享 daemon。
- 明确拒绝共享 provider 配置的 app-server proxy、daemon lifecycle 和 Amazon Bedrock 登录，不能静默回落默认账号或留下共享配置。

账号名允许中文；拒绝路径分隔符、路径穿越、Windows 非法字符、保留名、前后空格及超过 64 个字符的名称。

### 5.4 每次 401 重读认证文件

实际凭据来自所选账号认证文件时，每次 HTTP 401，包括 WS 握手失败，都重新读取该文件，不受 managed 恢复状态机次数限制。

覆盖：

- 文件中的 API key、ChatGPT 等凭据。
- File 存储。
- Auto 存储实际回退到文件的情况。
- 默认和命名账号。
- HTTP 流式 / 普通响应、Responses / realtime WS 握手。

以下情况保留原缓存：

- 文件缺失或损坏。
- 认证类型或账号身份不匹配。

以下凭据不能被文件覆盖：

- provider token。
- 环境变量。
- 外部认证。
- 内存凭据。
- 实际来自 keyring 的凭据。

重读不改变 `10s` 等待、累计次数、retry budget 和每 3 次恢复规则，也不能串用其他账号。

## 六、Context window 图片梯子

### 6.1 触发与档位

仅作用于普通 sampling 的连续 context window 溢出。

第 1、2 次原样重试；通常在第 3、6、9、12 次分别升档：

| 档位 | 保留范围 | 更早图片的处理 |
|---|---|---|
| 第一档 | 最后 5 张保持当前状态 | `original → high` |
| 第二档 | 最后 1 张保持当前状态 | `original → high` |
| 第三档 | 最后 5 张保留真图 | 替换为 1×1 占位图 |
| 第四档 | 最后 1 张保留真图 | 替换为 1×1 占位图 |

保留范围不会把此前已降档或占位的图片恢复为原图。

某档没有变化时，在同一次触发中继续升档，不再额外等待 3 次失败。

- 档位按回合记账，同回合多次 sampling 共享进度。
- 新回合从第一档重新开始。
- 四档用尽后仍自动重试，不把 context window 改成终态失败。
- 显示临时 `Context overflow image ladder step N`。
- 不修改磁盘截图文件。

图片梯子只改图，不丢弃 `previous_response_id`，不重置 WS；连接和粘连恢复仍由第三节管理。

### 6.2 持久化与重放

同时修改内存历史，并追加 `RolloutItem::ImagesShrunk`，只记录 `tier` 和 `changed`，不写全量历史快照。

resume / fork 重放时：

- 对当时已重建出的历史重新执行同一档。
- “保留最后 N 张”按存活历史重新计算，不按过期位置回放。
- 每档幂等，跳过已经降档或占位的图片。
- 不调用 `image_preparation::prepare_response_items`。
- 只调整 `detail` 或替换为已知占位 URL，不重新编码。

`prepare_response_items` 是发送前步骤，可能把无法处理的图片改成文字，在重建阶段调用会污染历史。

ImagesShrunk 不是 compact 基线：

- 不推进 compact 窗口。
- 不显示 compact UI。
- 不参与 fork 边界或回合计数。
- 不截断重放范围。
- 单条记录保持 1 KB 量级。

paginated 会话通过 fork 派生新线程，原线程历史保持不变；legacy rollout rollback 只保留兼容验证，不作为当前主路径验收。

### 6.3 必须保留的历史经验

**清单与验收｜2026-08-15**

- 图片梯子必须是独立编号功能。
- 第 3 项负责“溢出仍可重试”，第 18 项负责“如何瘦身旧图”，不各自维护重复规则。
- 验收看下一包是否变瘦，以及是否出现升档提示，不能只匹配 1×1 常量。
- 发送前 prepare 可能把 1×1 转为 omit 文案，这仍可说明请求变瘦，但不能把 omit 写成目标规格。

**错误落盘方式｜2026-08-16**

旧版曾把瘦身历史写为：

`CompactedItem { message: "", replacement_history: Some(全量历史) }`

不能恢复这种做法：

- `Compacted` 同时承担历史基线、compact 窗口节点和回合边界。
- 重建时遇到带 `replacement_history` 的记录，会将 `rollout_suffix` 截到该记录之后，即 `rollout_suffix = &rollout_items[index + 1..]`，导致后续 resume / fork 历史残缺。
- 实际变化只有几百字节，却每档写出含原图 base64 的几十 MB 全量快照；`load_history` 又会全量读入内存。

真正的 remote compact 也可能是“空 message + 非空 `replacement_history` + 有 `window_number` / `window_id`”，与旧图片梯子记录同构。不能通过“忽略空 message 的 Compacted”清理旧数据，否则会误伤真 compact。

原记录说明：发布 `0.144.3-local3-image-ladder` 后，已检查该版本期间的会话，未发现此类记录，因此当时无需迁移。该结论不代替其他版本的数据核查。

**新增 rollout 变体的检查点**

原修复涉及约 15 处穷尽 `match`，包括：

- rollout 的 policy、metadata、list、search、recorder、persistence_metrics。
- state 的 extract、runtime::threads。
- thread-store、app-server-protocol、memories。
- core 的 spawn 和重放。

rollout policy 必须允许持久化，否则新记录会被静默丢弃。

## 七、日志、批量优化与运行时清理

### 7.1 默认日志降噪

Windows App、app-server、TUI 默认使用较安静的日志设置；显式配置仍可开启详细诊断。

analytics、feedback、log_db 默认关闭，可配置开启。

app-server stderr 默认不输出：

- 普通后台诊断日志。
- `codex app-server (WebSockets)`。
- `listening on`、`readyz`、`healthz`。

设置 `[logging] app_server_stderr = true` 后恢复诊断。真正启动失败或配置错误仍须向用户显示必要错误。

**经验｜2026-06-02：** 不能只检查 tracing layer，启动 banner 也是 stderr 来源。

### 7.2 批量优化

包含两种优化：

- **会话记录批量写盘**：合并多次小写入，减少磁盘操作。
- **app-server 高频通知合并**：减少连续小更新，降低客户端负担。

默认可以开启，但以下结果须与关闭优化时保持用户可感知等价：

| 检查项 | 要求 |
|---|---|
| 回答输出 | 及时显示，不能长时间积压 |
| token usage | 用量及时、准确更新 |
| diff / plan | 文件改动和任务计划及时更新 |
| 命令完成 | 命令完成状态及时回传 |
| 崩溃恢复 | 会话恢复可靠性不因批量保存而退化 |

必须保留显式关闭开关。

### 7.3 node_repl 继承当前 CLI

`node_repl` 是指定的 MCP 工具服务。它需要调用 Codex 时，应使用主程序当前运行的 local3 exe。

仅对 server 名为 `node_repl` 的 MCP 注入：

`CODEX_CLI_PATH = Config.codex_self_exe`

目的：避免主程序已经更新，工具却调用自动安装目录里的旧 exe，导致刷新、诊断和 app-server 行为不一致。

不能全局改写其他 MCP server 的环境变量，包括其他本地 stdio MCP。

### 7.4 app-server 退出清理

主 app-server shutdown 调用已有 `clear_runtime_references()`，释放外部 auth、apps runtime、skills watcher 等引用。

禁止：

- 新增 idle timeout。
- 全局扫描或 kill `node_repl.exe`。
- 因 UI 订阅断开而结束仍加载的线程。

## 八、Windows 构建与交付

### 8.1 固定流程

- **静态复核：** 核对本次修改和相关验收范围。
- **GitHub build / test：** 本地不编译；只构建 `x86_64-pc-windows-msvc` 的 `codex.exe`，上传独立 artifact，不直接发布 Release 或 prerelease。
- **下载 artifact 验证：** 使用该 run 下载的真实 exe，执行 `--version`、`--help` 和相关行为验证。
- **GitHub promotion：** 验证通过后，由 workflow 从已验证 artifact 创建或更新正式 Release 并附加资产，禁止本地上传。
- **重新下载 Release 核验：** 比对 GitHub asset digest，再用 Release 下载的 exe 完成最终 smoke 和相关回归。

build、下载验证或 promotion 任一环节失败，修复后从 GitHub build 重新开始。

所有 GitHub CLI 查询与触发显式指定：

`--repo dqIndieGames/codex`

### 8.2 验证来源

- 本地旧 exe、其他 run 的 artifact、源码静态检查和本地编译产物，不能代替本轮 artifact 验证。
- 发布后的最终检查必须使用重新下载的 Release asset，不能继续拿 Actions artifact 代替。
- Actions artifact 不会自动出现在 Releases 页面，必须经过独立 promotion。
- 禁止本地编译时，编译及相关测试证据来自 GitHub Actions。

### 8.3 历史经验

- **2026-06-02：** 必须下载云端产物验证真实 exe，不能拿源码或本地旧文件代替。
- **2026-06-03，合入 `rust-v0.136.0`：** 只更新版本号不够，须逐项核对清单、版本身份、历史发现、日志、node_repl 和 runtime 清理。
- **2026-06-14：** 明确 build 与 promotion 分离；构建成功不等于可以发布。
- **2026-08-16：** 原记录指出，2026-08-07 的 compile-only 调整曾从 `local2-minimal-windows-release.yml` 删除回归和 smoke，导致 rollout / resume 未经验证即可发版。后来恢复测试并补充图片梯子、重放回归；不能再次用“能编译”代替这些检查。

## 九、合并上游后的验收矩阵

本表列出检查入口和用户结果；具体规则见对应正文。

| 主题 | 必验场景与关键证据 |
|---|---|
| 版本身份 | CLI、doctor 及 runtime details、TUI、app-server initialize、daemon / remote-control、登录欢迎文案、历史、rollout 和升级提示；展示版本与协议裸版本分别验证 |
| 服务端 User-Agent | WS 握手、登录、ChatGPT 辅助接口和云任务；版本来自当前包版本，无 `-local3`，其余字段保持官方行为 |
| 首次清单 | 新线程、Clear、重复输入、恢复旧线程、fork、子会话、多段 / 富文本 / 附件；只在规定入口触发，包含第 18 项 |
| 固定 10 秒 | HTTP request、stream / WS、各 compact、realtime、WebRTC、`Retry-After`；验证每次失败后的等待，不只检查最大值 |
| 流式配置超时 | 首次与重试；默认值和自定义值；首事件及后续空闲均按配置；无额外固定上限，持续输出不按总时长中断 |
| 响应头与建连 | HTTP 响应头等待无 local3 固定看门狗；WS 建连与流式等待分开；检查取消、refresh、断流和各自连接超时 |
| WS 保持协议 | 普通回答、local compact、remote compaction v2；官方 / 中转；握手失败、断流、超时、连续失败及预算耗尽后不自动切 HTTP |
| WS 恢复结果 | 新连接仍为 WS；后续轮次保持协议；工具结果和续链完整，已完成工具不重做；取消有效，计数不归零 |
| WS 真实界面 | 使用下载的 GitHub artifact exe；截图并读图确认重连提示，结合请求证据确认仍使用 WS |
| 中转粘连恢复 | 503 / 502 / 504、断流、握手、capacity；第 3 / 6 / 9 / 12 次；默认官方地址、中转、全量重放与工具续链 |
| 上下文压缩 | local、旧 `/responses/compact`、remote compaction v2 分别验证；默认重试、按 URL 恢复、bounded / zero；旧入口前三次提示及历史隔离 |
| 实时语音连接 | realtime 直连 WS 与 WebRTC sideband 分别检查；重试、每 3 次内部恢复轮次、临时提示、配置刷新中断及 session id 不变 |
| 重试显示与历史隔离 | HTTP 和 stream / WS 均显示连续 1..6；不暴露哨兵值；重连和 refresh 不回绕；无逐次普通错误日志，不进历史 / fork / replay |
| 旁路边界 | `/models`、文件 URL 生成不强制套主链 retry；鉴权隔离和安全错误提示仍检查 |
| Provider 刷新 | app-server、TUI / console、exec、当前及新 subagent、agent_jobs、tray、无实例；无报错刷新、同地址换 token、换地址、重试中刷新及各 scope |
| Provider token | 有 / 无 token 双向切换、空值、与 `env_key` 同时配置、全局登录态存在；覆盖聊天、压缩、realtime、HTTP / WS、`/models` 和刷新后请求 |
| 服务层 | 顶层未配、开启、关闭；Fast / Flex / None；profile 不覆盖，刷新后生效 |
| 历史会话恢复 | 历史列表、最近会话、resume 选择器 / 最近会话、deep link、已加载线程；下一请求使用当前 provider；fork 选择范围仍按当前 provider 过滤 |
| 多账号 | 默认及两个命名账号、中文名、非法名、并发、API key / ChatGPT、登录 / 状态 / 刷新 / 登出、TUI / embedded app-server、共享 daemon 和拒绝入口 |
| 401 文件重读 | 连续至少 4 次 401；跨第 3 次恢复仍有效；HTTP、Responses / realtime 握手；File / Auto、命名账号、坏文件、身份不匹配及非文件凭据 |
| 图片梯子 | 第 1 / 2 次原样、第 3 / 6 / 9 / 12 次升档、无变化连升、同回合共享及新回合复位；四档用尽仍重试，下一包确实变瘦 |
| 图片历史 | ImagesShrunk 大小、resume / fork 重建、原线程不变、paginated 主路径、legacy 兼容、无 compact 副作用、磁盘原图不变 |
| 默认日志 | Windows App、app-server、TUI；analytics / feedback / log_db 默认关闭及显式开启 |
| stderr | 默认静音与显式开启；tracing、WS banner、启动失败及配置错误 |
| node_repl | 调用当前 local3 exe；其他 MCP 环境不被改写 |
| 批量优化 | 开启 / 关闭时，回答、用量、文件改动、计划和命令完成状态及时一致，崩溃恢复可靠 |
| 退出清理 | 已有引用释放；无新增空闲超时、全局 kill 或误伤仍加载线程 |
| Windows 交付 | 本轮 build → artifact 真 exe 验证 → GitHub promotion → Release 重新下载核验；版本、重试序号及相关行为通过 |

## 十、配置与 WS 版本核对快照

**核对日期：2026-10-06。**本节记录当时配置，不作为所有安装环境的固定要求。

配置文件：

`C:\Users\Administrator\.codex\config.toml`

相关配置节选：

```toml
model = "gpt-6-astra"
model_provider = "openai_http"
model_reasoning_effort = "xhigh"
service_tier = "default"
force_service_tier_priority = false

[features]
fast_mode = false

[model_providers.openai_http]
name = "OpenAI WebSocket"
base_url = "https://chatgpt.com/backend-api/codex"
wire_api = "responses"
supports_websockets = true
request_max_retries = 100000
stream_max_retries = 100000
requires_openai_auth = true
```

解读：

- `openai_http` 是 provider 标识名，不能根据名字判断传输方式。
- `wire_api = "responses"` 指 Responses API。
- `supports_websockets = true` 表示启用 WS。
- 当前 provider 未显式配置 `stream_idle_timeout_ms`，按当前实现使用默认 300 秒。
- 当前 provider 未显式配置 `websocket_connect_timeout_ms`，按当前实现使用默认 15 秒。
- 顶层显式关闭 priority 强制映射及 fast mode，与第四节允许显式关闭的规则一致。

当前代码的 WS 握手使用：

```http
OpenAI-Beta: responses_websockets=2026-02-06
```

代码将其标为 Responses WebSockets V2。

- v1 是早期实验版本；旧 `responses_websockets` 和 `responses_websockets_v2` 开关均已标为移除。
- 当前通过 provider 能力选择 WS，不通过旧开关二选一。
- v2 使用 `response.create`；满足续接条件时，通过 `previous_response_id` 和新增输入继续请求。
- WS v1/v2 是应用层协议版本，与模型版本、HTTP/1 / HTTP/2、remote compaction v2 分开理解。
- 当前官方 [WebSocket 模式说明](https://developers.openai.com/api/docs/guides/websocket-mode)描述了复用连接及增量续接方式；它不构成旧 v1 全部字段差异的说明。

**配置启用 WS，不等于能证明某条既有连接此刻仍在 WS。**核对时现行代码仍有自动切 HTTP 的路径；本稿要求后续取消该行为。固定 10 秒和禁止自动降级均须通过实际实现与验收后，才能标记完成。
