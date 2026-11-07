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

3. **主链自动重试。** 除交回官方上下文处理的超限错误外，会终止对话、上下文压缩或 realtime 模型链路的远端请求错误均进入自动重试，保留默认无限重试；有效 `Retry-After` 优先，没有有效服务端等待建议时等待 `10s`，具体按 3.1 执行；单次超时只结束当前尝试，下一次重新获得完整时间额度。

4. **重试可见且不污染历史。** 用户能看到持续累计的重试次数和安全诊断信息；中间失败只作临时状态展示，不写入历史、fork、replay 或普通错误日志。

5. **跨 provider 继续历史会话。** 历史入口默认可发现不同 provider 的旧会话；继续旧线程时使用当前顶层 provider，无法换绑时明确提示仍在使用旧 provider。

6. **全局优先服务层。** 默认使用 priority；顶层显式关闭后恢复官方 Fast / Flex / None 映射，profile 内同名设置不生效。

7. **默认日志降噪。** Windows App、app-server 和 TUI 默认减少日志；analytics、feedback、log_db 默认关闭，仍允许显式开启。

8. **批量优化与历史安全。** 默认开启会话记录批量写盘和 app-server 高频通知合并，保持回答、用量、改动、计划及命令状态及时更新，崩溃恢复可靠，并保留关闭开关。

9. **Provider 刷新覆盖全部入口。** 地址、token、优先服务层和 fast mode 有效配置变化后，刷新 app-server、已打开会话、exec、subagent 和 agent_jobs 等运行入口。

10. **Provider 刷新无需等待报错。** 配置变化后，下一次请求使用新配置；地址或凭据变化会中断旧连接及其等待、重试，并重新建立连接，同地址换 token 也生效。

11. **连接恢复与 HTTP 保底。** Responses WS 遇到 1009 时直接转 HTTP；其他失败连续 3 次转 HTTP；普通对话与压缩规则相同，转 HTTP 后本会话（含此后新开的子代理）一直使用 HTTP，新开会话或重启后恢复 WS；保留累计序号、必要续链和已完成工具结果。

12. **保存与刷新结果分开反馈。** Provider 字段保存成功但没有运行实例可刷新时，仍视为保存成功，并明确提示“未刷新任何实例”。

13. **app-server stderr 默认安静。** 后台日志和 WebSocket 启动诊断默认不输出；设置 `[logging] app_server_stderr = true` 后恢复诊断，真正启动失败和配置错误仍显示必要信息。

14. **node_repl 使用当前 CLI。** 启动 `node_repl` MCP 时，将 `CODEX_CLI_PATH` 指向当前 `Config.codex_self_exe`，避免工具子进程误用旧版 Codex；其他 MCP 环境不被改写。

15. **退出时安全释放引用。** app-server 退出时调用已有 runtime 引用清理，不新增空闲超时，不全局扫描或结束进程，不因 UI 断开而杀掉仍加载的线程。

16. **Provider token 隔离。** 非空 `experimental_bearer_token` 优先用于该 provider 的请求，不继承全局登录态；没有该 token 时继续使用原 AuthManager 行为。

17. **Windows 云端构建与发布。** 本地不编译；先下载 GitHub build artifact 验证真实 exe，再由 GitHub promotion workflow 发布 Release，禁止本地上传资产。

18. **六档渐进图片处理。** 普通模型请求及 HTTP / WS 压缩共用六档策略；请求过大、明确的上传超时及所有 `response.failed` 连续三次触发，先重编码和缩图，再保留最近 5 张及必需图片，最后淘汰其他旧图；必需图片禁止占位，最高档仍能处理新增旧图；保留官方上下文流程、原图、轻量持久化及重试预算，具体按第六节执行。

19. **多账号隔离与标题标识。** `--account <账号名>` 为每个账号使用独立认证文件；官方账号的终端标题显示 `[序号]`，第三方 provider 显示其标识的首字符，英文字母转小写，例如 `yunyi → [y]`；切换 provider 后同步更新，具体按 5.3 执行。配置、会话、日志、skills 和 MCP 继续共享，支持多账号同时运行。

20. **每次 401 重读所选认证文件。** 实际凭据来自文件时，每次 HTTP 401，包括 WebSocket 握手失败，都重新读取所选账号的 `auth.json`；无效文件不覆盖缓存，非文件凭据不受影响。

21. **服务端版本与多账号请求一致。** 发往服务端的本程序版本使用当前 Cargo 裸版本，User-Agent 主段、附加版本段和遥测均不夹带 `-local3`；相同条件下，默认与命名账号除认证身份及必要动态值外，请求保持一致，详见 5.3。本机展示及 initialize 返回本机的展示版本保留后缀。

## 二、版本身份与首次清单

### 2.1 版本按用途区分

| 用途 | 版本格式 |
|---|---|
| 本机或客户端展示 local3 构建身份 | `<当前包版本>-local3` |
| 发往服务端的本程序版本，包括完整 User-Agent 内的各版本段及遥测 `client_version` | 当前 Cargo 裸包版本 |
| `/models` 的 `client_version` | 同一裸包版本 |
| 更新比较、Python wheel、配置锁、OpenTelemetry `service_version`、OAuth / device-code 协议参数 | 裸 semver |

展示面必须覆盖：

- CLI、TUI、状态卡、标题区、历史单元和升级提示。
- `codex --version`、`codex doctor --json`、doctor runtime details、`codex-app-server --version`。
- app-server initialize 返回本机的 `user_agent`。
- daemon / remote-control JSON 中仅用于本机展示构建身份的字段，以及 device-code 登录欢迎文案。发往服务端的注册请求不属于展示用途。
- 线程历史元数据和 rollout 会话记录元数据。

`cli_version`、`client_version`、`app_server_version` 按实际用途选择版本，不能仅凭字段名统一替换。

**远程控制注册版本修正：**`enroll_remote_control_server` 构造的 `EnrollRemoteServerRequest.app_server_version` 必须使用当前 Cargo 裸包版本，不得使用 `CODEX_DISPLAY_VERSION` 或携带 `-local3`，不得写死版本号。本机展示及 initialize 返回本机的展示版本继续保留后缀。

**待修复记录：**本次源码核对发现，该注册字段仍使用 `CODEX_DISPLAY_VERSION`；补充本清单不代表代码已经修复，须完成实现与实际请求验收后更新状态。

发往服务端的 HTTP `User-Agent`：

- 从当前 Cargo 包版本生成，不写死历史版本号。
- WebSocket 握手、登录、ChatGPT 辅助接口和云任务统一执行。
- `originator`、操作系统、架构、终端标识和第三方客户端信息保留真实含义；本程序生成的附加版本段同样使用裸版本，不能只处理 User-Agent 第一段。
- 默认、内嵌和后台服务共用对外版本生成规则；展示版本不得经初始化参数或全局 UA 后缀重新进入出站请求。
- 客户端身份和终端信息来源保持一致，不因账号选择读取另一个客户端遗留的全局状态。

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

### 3.1 重试范围与服务端等待建议

这里的 sampling 指普通模型请求；compact 指压缩长会话上下文；retry budget 指允许的重试次数。

强制重试范围包括：

- 普通 sampling。
- Responses HTTP request retry。
- SSE / WebSocket 流式重连。
- 当前版本的 local compact、remote compaction v2；旧版本的 `/responses/compact` 迁移说明见 3.6。
- realtime WebSocket connect、WebRTC sideband join。
- TUI、exec、subagent、agent_jobs 中对应的模型请求入口。

除下述上下文超限的官方处理分支外，远端主链错误不能因 `is_retryable=false`、状态码、错误码或协议映射被排除。范围包括：

- HTTP 503 / 502 / 504 / 429 / 402。
- 网络失败、临时鉴权错误、未完成断流、握手失败和请求超时。
- `Selected model is at capacity. Please try a different model.`
- HTTP `429`、`turn_rate_limit_exceeded`，以及 `Codex is at capacity, please try again later`。不能因新错误码未被专门识别而退出主链重试。
- usage / quota、policy 等服务端错误。

`context_length_exceeded` 及明确的上下文 token 超限交回对应官方版本的上下文处理流程，保留自动压缩、超限错误返回及各 compact 恢复行为，不得被通用无限重试循环截住。独立返回的上下文超限不单独触发图片处理；通过 `response.failed` 返回时，该事件仍按 6.1 计入图片处理次数，但不得为了凑够三次或执行图片降档而阻止官方返回、递归启动摘要或增加重试。首次失败即按官方流程结束当前逻辑请求时，计数也随该请求结束，不跨新请求累计。

`/models` 刷新和 MCP OpenAI 文件上传 / 下载 URL 生成属于旁路请求，不纳入本清单强制 retry / sticky-break 范围；涉及鉴权和用户错误提示时，仍遵守对应规则。

**默认无限重试保持不变；等待多久与重试多少次分别处理。所有主链入口统一使用以下等待规则。**

**等待值优先级**

- 优先使用本次失败携带的有效 `Retry-After`。
- 缺失或无效时，沿用已支持的限流错误文案等待解析；`turn_rate_limit_exceeded` 纳入限流错误识别。
- 均无有效服务端等待建议时，固定等待 `10s`。
- 不额外叠加指数退避、jitter 或增长型 `base_delay`。
- 不把服务端等待值截短为 `10s`，也不在服务端等待之外再加一次 `10s`。

**不同传输方式的读取位置**

| 错误形式 | 等待建议读取位置 |
|---|---|
| HTTP 错误响应、WS 握手失败 | 实际 HTTP 响应头 `Retry-After` |
| SSE／WS 的 `response.failed` | 依次读取 `response.error.headers`、`response.headers`、事件顶层 `headers` |
| SSE／WS 的 `type: "error"` | 依次读取 `error.headers`、事件顶层 `headers` |

- 同一事件存在多个位置时，按表中顺序取第一个有效值；字段缺失或无效时继续检查下一位置。
- Header 名称不区分大小写。支持非负整数秒和 HTTP-date；空值、负数、不可解析或无法表示的值视为无效。
- SSE 和 WS 必须保留相同错误的错误码、等待建议及重试语义，不能只支持其中一种封装。
- 识别到本次请求失败后，结束当前失败尝试并进入重试等待，不继续等待无关的流结束或空闲超时。

**实际等待与恢复**

- 收到有效等待建议时记录截止时间；后续错误转换、连接恢复和协议切换沿用同一截止时间。
- 发送下一次请求前等待剩余时间。同一次失败只等待一次，不重复计时，不因换连接、WS → HTTP 或中转粘连恢复提前发送。
- `Retry-After: 10` 等待至对应的 10 秒截止时间；`Retry-After: 30` 等待至对应的 30 秒截止时间。
- 有效值为 `0` 或 HTTP-date 已到期时，无需额外等待；没有有效建议时才使用 `10s`。
- 每次新的失败重新采用该次响应的等待建议，成功后不把旧等待建议带入下一次请求。
- 用户取消和 provider refresh 仍按现有规则中断旧请求及等待。等待不扣减下一次尝试的超时额度。
- 等待值不改变默认无限重试、显式有限／零预算、累计序号和每 3 次恢复规则。

**重试预算与边界**

- 正常请求不额外等待。
- 默认允许无界或非常大的重试次数。
- 显式 bounded 模式下，各重试分支必须遵守有效次数上限；`ConnectionFailed` 和 `unbounded_connection_retries` 不得绕过该上限。`stream_max_retries = 0` 禁用对应流式重试。默认无限重试与显式有限／零预算分别验收。
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
- 超时只取消当前尝试并显示临时状态，按 3.1 等待后重试；没有有效服务端等待建议时等待 `10s`。
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

### 3.4 Responses WS 失败转 HTTP 保底

本节适用于选用 Responses WebSocket 的普通 sampling 和各压缩入口（local compact、remote compaction v2），两者使用同一套规则；官方和中转均执行。realtime 和 WebRTC 按 3.6 的 realtime 规则处理，不转 HTTP；原本使用 HTTP 的入口继续使用 HTTP。

| 情况 | 处理 |
|---|---|
| 正常 | 使用 WS；满足续接条件时只发送新增内容 |
| WS 返回 `1009` | 本次不处理图片、不计入图片处理次数、不在 WS 上重试，下一次尝试直接转 HTTP；后续 HTTP 失败按 6.1 判断是否计入图片处理次数 |
| 其他任何失败，例如握手失败、断流、超时、HTTP 5xx | 继续用 WS 重试；连续失败 3 次后，下一次尝试转 HTTP |
| 已转 HTTP | 本会话后续全部请求，包括普通 sampling、压缩，以及此后新开的子代理、agent_jobs 子任务和审核会话，均使用 HTTP，不再切回 WS |
| 新开会话，或重启 codex 后重新进入（含 resume） | 按 provider 配置重新选择协议 |

补充规则：

- 连续失败次数在同一请求的重试过程中累计，WS 上每次失败加 1，包括首次尝试失败；请求成功后，下一次请求重新计数。
- 每次失败按 3.1 等待；同一次失败只等待一次，不因图片处理、连接恢复和协议切换叠加或跳过等待。1009 直接转 HTTP 表示不再尝试 WS，下一次 HTTP 请求仍须遵守本次等待规则。
- 转 HTTP 前后均保持：默认无限重试、累计序号不归零、已完成工具结果不重复执行。显式有限预算继续生效；预算耗尽按 3.1 结束，不触发转 HTTP，转 HTTP 也不重置或增加预算。
- 转 HTTP 后发送完整历史，不沿用旧 WS 连接的 `previous_response_id`；必要的续链语义保持完整。
- HTTP 锁定只保存在当前进程的会话状态中，不写入会话记录，不修改 provider 配置；provider 刷新（含换地址、换 token）和中转粘连恢复都不解除锁定。
- 用户取消立即生效，取消不触发转 HTTP。
- 重连、图片升档和协议切换只展示临时状态。
- WS 的 `response.failed` 同时参与 WS 失败计数和 6.1 的图片失败计数。两者同次达到三次时，下一次 HTTP 请求使用处理后的图片；各计数独立，同一失败不重复等待。1009 本次不是图片触发事件，其对图片连续计数的影响按 6.1 执行。

关闭连接的诊断可保留协议 close code；原始 reason 不直接显示或落盘，避免泄漏服务端回传内容。

**历史说明｜2026-09-29、2026-10-06、2026-10-14**

以下保留当时的故障与实现记录；旧图片触发范围不作为现行验收标准，现行范围以第六节为准。

- 2026-09-29（`94a51d4dcd`）加入“WS 连续失败 3 次转 HTTP”；普通请求和压缩共用该入口，转 HTTP 后锁定整个会话。0.159.1-local3、0.159.3-local3 为此行为。
- 2026-10-06 合并 0.160.1（`e01f4d5bc0`）时移除该逻辑，普通 sampling 改为始终保持 WS；0.161.0-local3 再加回仅限当次压缩的降级，并增加首次 1009 先处理图片。
- 2026-10-14：用户截图显示普通对话因 WS `1009` 连续重连 364 次，约 2 小时 10 分钟。`1009` 表示单条 WS 消息超过服务端上限（社区实测约 16 MiB，非官方公布，见 [openai/codex#38846](https://github.com/openai/codex/issues/38846)）；断线、resume 或上次请求失败后需要整段重发历史，截图 base64 使其超限，原样重发必然再次失败。因此恢复会话级 HTTP 保底：1009 直接转 HTTP，其他失败连续 3 次转 HTTP，普通请求与压缩规则统一；图片处理只在转 HTTP 后仍超限时按计数触发。
- 2026-10-14 实现（提交 `7bc9eeaa1c`）：`codex-rs/core/src/client.rs` 增加会话级 HTTP 锁定，`codex-rs/core/src/responses_retry.rs` 在 1009 或连续 3 次 WS 失败时转 HTTP，`codex-rs/core/src/session/turn.rs` 把普通请求的 413 计入升档。当时按第八节用 run `38497878721` 的 exe 本地验收通过，发布为 `rust-v0.162.1-local3-ws-http-fallback-20261014`；后续版本仍需重新验收。

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
- 按当前传输方式恢复连接；仍使用 WS 时重置 WebSocket session，满足 3.4 条件时改用 HTTP，已锁定 HTTP 的会话保持 HTTP。
- 让紧随其后的请求使用新 generation，不携带旧 turn-state 和旧 `previous_response_id`。

边界：

- 不改变真实 thread id、session id 或用户提示词。
- Codex 只能清除自己控制的信号，不能保证中转一定换号。
- `function_call_output` 等必要续链请求仍须重试并参与计数，但必须保留必要续链 ID，或采用不破坏续链的恢复方式。
- 官方地址不为 sticky-break 旋转 cache key、清空 turn-state、强制重置 session 或丢弃续链 ID。
- 官方连接失效时，在满足 3.4 转 HTTP 条件前仍可正常重建 WS，这与中转粘连转换分开处理。

### 3.6 compact 和 realtime 的独立入口

**compact：长会话的上下文压缩**

官方 0.165.0 已移除旧 `/responses/compact` 实现，远端压缩统一为 remote compaction v2；不恢复已移除接口。当前分别验收 local compact 和 remote compaction v2，重试、图片策略和历史保护要求保持不变。维护更早版本时，旧入口仍须独立验收。

- 除 3.1 规定的官方上下文处理分支外，默认模式必须重试。
- 每连续 3 次失败按实际 URL 判断是否执行中转粘连恢复。
- 不能因 compact 专属预算或默认配置，在第 3 次之前提前终态；官方上下文超限返回、用户取消及显式预算仍优先，不为凑够图片三次触发而强行重试。
- 显式 bounded / zero 仍按用户设置处理。
- 压缩请求重试必须展示临时 `503 retry N` 或 `Reconnecting... N`。
- 只证明第四次请求 cache key 变化，不足以证明前三次提示和历史隔离正确。

#### 压缩共用图片策略

- HTTP 和 WS 压缩均接入第六节的六档图片策略，覆盖自动压缩和手动压缩。
- local compact、remote compaction v2 分别核对：携带历史图片的请求必须使用处理后的图片；使用 WS 的入口执行 3.4 的 HTTP 保底规则。
- 启动上下文压缩本身不触发图片降档；错误按 6.1 统一分类计数，所有 `response.failed` 均纳入。上下文超限保留 3.1 的官方处理分支，WS 1009 本次不计入图片处理，只触发转 HTTP。
- 下一次请求必须使用处理后的历史，不能继续发送压缩开始时保存的旧图片副本。
- 协议切换本身保留图片处理结果、逐图进度、保留策略和图片失败计数，不恢复旧图；触发切换的失败是否增加或清零图片计数，仍按 6.1 判断。

#### 压缩的协议选择

- 压缩与普通 sampling 共用 3.4 的连续失败计数、1009 处理和会话级 HTTP 锁定。
- 压缩开始前本会话已锁定 HTTP 时，压缩直接使用 HTTP；压缩过程中转 HTTP 后，压缩结束后的普通请求和后续压缩继续使用 HTTP。
- 原本使用 HTTP 的压缩继续使用 HTTP，同样执行图片策略。

#### 恢复状态与历史保护

- 用户可见重试序号、WS 连续失败计数、图片连续失败计数分别管理。
- 图片升档和协议切换不清零用户可见序号，不重做已经完成的工具调用。
- 保留必要续链语义；压缩成功后不得用旧连接状态重新带回压缩前的历史。
- 中转粘连恢复继续按实际 URL 判断，但不得把已锁定 HTTP 的会话切回 WS。
- 重试、图片升档和协议切换提示仅作临时展示；实际图片变化仍按第六节持久化。

#### 压缩结果成功后提交

- 自动压缩和手动压缩的输出先暂存，收到 `response.completed` 且结果校验通过后，才提交摘要及压缩后的历史。
- local compact 必须得到本次尝试的非空 assistant 摘要；remote compaction v2 必须得到符合协议的压缩结果，不能借用旧历史中的最后一条回答。
- 断流、错误、取消或预算耗尽时，丢弃本次暂存输出，不写入失败摘要，不推进压缩窗口，不留下半成品压缩记录。
- 每次重试独立收集输出，不混入上次失败尝试的内容。
- 第六节规定的图片处理及 `ImagesShrunk` 记录继续保留，不因压缩失败恢复旧图。
- 验收覆盖：输出摘要后断流、输出后取消、完成但缺少有效摘要、失败后重试成功；检查内存历史、持久化记录及 resume／fork 重建结果。

文中的 **remote compaction v2 是压缩实现版本，不是 WS 协议版本**。

**realtime：实时语音等交互连接**

它与普通文字对话的 Responses WS 是不同入口。

分别检查：

- ordinary WebSocket connect：直接建立 realtime WS。
- WebRTC sideband join：为 WebRTC 实时连接建立配套控制通道。

要求：

- 使用 provider retry 配置，等待规则统一按 3.1 执行；覆盖 realtime WS 建连、WebRTC 请求及 sideband 握手失败。
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
- 经脱敏的模型错误码、标准化错误类别、失败阶段及错误字段是否缺失／解析失败；图片处理的档位、真图和占位图数量、请求总字节数及图片字节数。服务端自由文本不得原样输出，仅提取可安全展示的错误含义。

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
- 展示计数变化不能改变 3.1 的等待规则或触发协议切换；协议切换由 3.4 的连续失败计数和会话级 HTTP 锁定状态控制。

**旧等待口径残留｜2026-07-03**

当时曾因旧实现、测试名、断言和文档互相强化，留下 `8s` 或 cap-only 规则。现行规则统一以 3.1 为准：“无论上游建议多少都固定等待 10 秒”的旧口径已废止；`10s` 仅用于没有有效服务端等待建议的情况。

调整等待规则时检查：

- `E:\vscodeProject\codex_github\codex\codex-rs\core\src\util.rs`
- `E:\vscodeProject\codex_github\codex\codex-rs\core\src\util_tests.rs`
- `E:\vscodeProject\codex_github\codex\codex-rs\codex-client\src\retry.rs`，包括测试区。

排查以下仍被当作现行 retry 规则的硬编码；服务端合法下发的同值等待不属于旧规则残留：

- `5s`、`8s/eight seconds`。
- `<=5s`、`最高 5s`，以及仅设最大值的等价表达。
- 退避、jitter、增长型 `base_delay`。
- 已读取 `Retry-After`，但实际重试仍使用固定 `10s`。
- SSE／WS 只适配一种错误封装，导致另一种封装的等待建议丢失。
- 仍要求“忽略服务端等待建议”的测试名、断言和文档。
- 连接恢复、协议切换或嵌套重试重复等待，或提前绕过等待截止时间。
- WS 满足 3.4 条件却不转 HTTP（尤其是 1009 原样无限重试）、1009 仍先在 WS 上处理图片或重试、未满足条件就提前转 HTTP，以及转 HTTP 后同一会话又切回 WS 的实现、测试和文档。

历史数字可保留，但必须标明已废止，不能继续作为通过标准。

**主链遗漏｜2026-06-14、2026-07-03**

- `server_is_overloaded` 按 `ServerOverloaded` 处理；`turn_rate_limit_exceeded` 按限流错误处理。两者均须进入 retry / recovery，并保留等待建议直至实际等待；不能仅凭 capacity 文案统一分类，也不能只改 UI 或 `is_retryable()`。
- sampling、local compact、remote compaction v2 都可能把错误转为终态 `ErrorEvent`，须逐条覆盖。
- compact 必须同时验证自动重试、按实际 URL 恢复、两种协议的图片策略、1009 直接转 HTTP，以及转 HTTP 后本会话保持 HTTP。
- 普通 Responses 通过不能代替 realtime、WebRTC sideband 和各现存压缩入口验收；维护含旧 remote compact 的版本时，也须单独验证旧入口。

**静态复核｜2026-10-09**

本轮发现以下待修复项，尚未完成动态复现和修复验收：

- Provider refresh 清理共享缓存，但活动请求缺少完整中断路径；同地址换 token 存在复用旧认证 WS 的风险。
- 服务层刷新未同步活动回合使用的设置，后续请求可能继续读取旧值。
- 部分 local compact 流程在完成标记前写入摘要，失败后可能残留无效历史。
- 默认启用的连接失败无限重试分支可绕过显式 bounded 预算。

分别按 4.1、4.4、3.6、3.1 修复和验收。这些发现不等于已证明此次持续断流的唯一原因，也不代表全部由本次升级引入。

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

当前运行会话的 provider 切换生效后，终端标题按 5.3 更新；不能只在程序启动时确定前缀。仅保存配置但尚未刷新运行实例时，不提前显示新 provider 标识。

地址或凭据变化时：

- 同时使共享缓存和当前请求持有的活动 WS 失效；连接复用判断必须识别凭据变化，不能只比较地址。
- 中断旧请求的建连、HTTP 响应头等待、流式读取及重试等待，不等旧请求自行报错或超时。
- 使用新配置重建请求或重新握手；配置刷新本身不额外增加重试等待，远端失败的重试等待按 3.1 执行。
- 同地址只换 token 也执行上述流程。
- 保留已完成工具结果，不重复执行。
- 普通请求和压缩按新配置重建连接；本会话已锁定 HTTP 时继续使用 HTTP，provider 刷新不解除锁定，不能仅因重建连接而重新启用 WS。

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

profile 内同名设置不生效。Provider refresh 必须同步会话配置及活动回合的有效服务层、fast mode 设置；刷新后的下一次 sampling、重试或压缩请求均使用新值，不能继续读取旧回合快照。已发送请求不追溯修改。

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

认证文件及账号绑定的认证、连接状态必须隔离。配置、会话、日志、skills 和 MCP 配置继续共享，不复制、不分叉；共享配置不代表可以串用账号凭据或已认证连接。

`login`、`login status`、token 刷新、`logout`、TUI、exec 和所有获准使用的本地 app-server 路径，必须使用同一所选账号。

命名账号要求：

- 不覆盖默认账号或其他账号的认证文件。
- 可并发运行多个命名账号。
- 不隐式复用仅持有单一 AuthManager 的共享 daemon。
- proxy、daemon lifecycle 尚未支持所选账号安全隔离时，明确拒绝；完成隔离后才能开放，不能静默回落默认账号。命名账号仍拒绝会修改共享 provider 配置的 Amazon Bedrock 登录。

**账号切换不改变其他请求行为**

- 对比前固定程序版本、登录类型、模型、配置、输入、客户端能力和网络环境。
- 账号选择只决定认证上下文，不额外强制切换内嵌／后台运行方式；保留“不隐式复用单一 AuthManager 共享 daemon”的隔离要求。
- HTTP／WS 协议、目标路由、请求体、非认证请求头及完整 User-Agent 保持一致。
- analytics 默认关闭；显式开启时，同条件下客户端版本与运行方式信息保持一致，并如实上报，不能伪造 `rpc_transport` 消除差异。
- MCP 能力按客户端实际能力判断；账号选择不得额外开启或关闭 `userVerification`，也不得绕过设备支持和可信连接检查。
- 默认后台与命名账号使用一致的有效配置、代理和客户端身份来源，不能因旧进程状态产生额外差异。
- 比较时仅按明确字段排除账号凭据、身份标识及必要动态值；保留会话和续链关联检查，不整块忽略 headers 或 metadata。
- 这些要求不代表不同账号的服务端权限、额度或下发策略必然相同。

账号名允许中文；拒绝路径分隔符、路径穿越、Windows 非法字符、保留名、前后空格及超过 64 个字符的名称。

**终端标题账号／Provider 标识**

标题格式统一为 `[标识] 原标题`，前缀与原标题之间保留一个空格。

| 当前生效模式 | 标题前缀 |
|---|---|
| 官方默认账号 | `[1]` |
| 官方命名账号 | 使用该账号已保存的序号，例如 `[2]` |
| 第三方 provider `yunyi` | `[y]` |
| 第三方 provider `wanfeng` | `[w]` |

- 根据当前运行会话实际生效的 provider、路由及认证模式选择标识，不能只看是否传入 `--account`，也不能仅凭 provider 名称判断是否为官方线路。
- 第三方标识取配置键 `model_provider` 的首字符，英文字母转小写，其他字符原样保留；不取展示名称、模型名称或 URL。
- 第三方 provider 即使同时指定了 `--account`，标题仍只显示 provider 标识，例如 `[y]`；账号选择与认证隔离规则继续有效。
- 官方默认账号固定为 `[1]`；命名账号首次分配从 `[2]` 开始，已有序号继续复用，不重新编号。
- 同一官方账号在多窗口、重启及恢复会话后使用相同序号，不同账号不能分到相同序号。
- 第三方前缀不分配账号序号；不同 provider 首字符相同时允许显示相同前缀，不自动追加数字。
- provider 切换实际生效后立即替换前缀；例如 `[2] → [y] → [2]`，不得叠成 `[y][2]`，也不得残留旧标识。
- 标题更新、工作状态变化、清理标题及 resume／fork 后，标识符合当前生效模式，且只出现一次。
- 标识仅用于终端窗口／标签页标题，不写入会话名称、模型上下文或服务端请求；不改变原有认证行为。

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

重读不改变 3.1 的等待规则、累计次数、retry budget 和每 3 次恢复规则，也不能串用其他账号。

## 六、六档渐进图片处理

**状态｜2026-11-07：**本节为已确认、待实现及验收的新策略，替代旧四档规则。更新清单不代表代码或已安装 exe 已实现；候选编码参数尚未通过清晰度验收。

### 6.1 触发、计数与六档策略

普通 sampling、自动及手动 local compact、remote compaction v2 共用本节规则，HTTP 和 WS 均适用。图片处理是发送副本的重编码、缩放和旧图淘汰，不替代官方上下文摘要；图片字节数下降不等于图片 token 下降。

#### 错误触发范围

| 失败事件或错误 | 是否计入图片连续失败次数 | 处理边界 |
|---|---|---|
| HTTP `413` | 是 | 请求体过大，连续三次后执行一轮图片处理 |
| 明确表示请求体、上传内容或图片字节过大的 `Payload Too Large`、`Request Entity Too Large`、`request body too large` 等 | 是 | 依据实际错误字段的含义，不扫描用户正文或被回显的请求内容 |
| 明确的图片分辨率、像素数量或图片输入尺寸超限 | 是 | 纯重编码不能解决尺寸限制时，跳过不适用档位，进入等比例缩小 |
| 已确认发生于请求体上传／发送阶段的超时 | 是 | 必须有阶段证据，不能把所有 timeout 都解释成上传超时 |
| 所有 SSE／WS `response.failed` 事件 | 是，无条件纳入事件计数 | 无论错误对象缺失、解析失败、原因未知，还是已解析出明确的 code/message，均计入；HTTP 200 内的失败事件同样适用 |
| 独立返回、未通过 `response.failed` 封装的 `context_length_exceeded` 或明确的上下文 token 超限 | 否 | 按 3.1 交回官方上下文处理 |
| 未封装为 `response.failed`、也不符合前三类体积／尺寸错误的鉴权、权限、额度、限流、过载、参数、图片解码或策略错误 | 否 | 执行对应现有错误／重试流程；不因笼统的 `too large` 或错误码名称误判 |
| 普通连接失败、DNS／TLS 错误、断线、10053／10054、阶段不明的超时、明确的生成阶段／流式空闲超时 | 否，除非本次另收到 `response.failed` | 网络故障本身不触发图片处理；不得根据等待总时长推断上传超时 |
| WebSocket close code `1009` | 否，本次不处理图片 | 按 3.4 直接转 HTTP；后续请求失败重新分类 |
| 用户取消、provider refresh 的主动中断 | 否 | 执行取消或刷新，不伪造远端失败 |

**`response.failed` 的事件身份必须在错误转换后保留。**不得因为将其解析为鉴权、额度、限流、过载、策略、上下文或其他具体错误，就丢失图片计数资格；也不得把普通 `type: "error"` 事件自动等同于 `response.failed`。

- 具体错误仍用于安全展示、认证恢复、服务端等待及官方上下文处理；图片降档是额外的恢复尝试，不证明图片为根因，也不保证恢复成功。
- `response.failed` 内为上下文超限时，事件仍计入，但官方处理优先：不能截住错误返回、递归启动摘要或强行增加尝试来凑够三次。普通请求和摘要请求均遵守该边界。
- 例如 `response.failed` 携带 `rate_limit_exceeded` 时，计入图片次数，同时保留有效 `Retry-After`；独立 HTTP 429 没有该事件且没有其他触发条件时，不计入。
- 原始响应体、凭据及用户内容不得直接显示或默认逐次落盘；按 3.7 提取安全诊断信息。原因不明时如实显示未知，不能提示为已确认的图片过大。

#### 连续三次与推进规则

- 同一逻辑请求的符合条件失败合并累计，连续三次触发一轮处理；例如 `413 → response.failed → 明确上传超时` 触发一次。同一次失败只计一次，HTTP、事件解析与重试层不得重复累加。
- 每轮处理检查后重新计数；无可处理图片时也不空转编码。请求成功、逻辑请求结束或遇到不属于触发范围的失败，清零图片连续失败计数；不清零用户可见的累计重试序号。
- sampling 与每次 compact 分别维护各自逻辑请求的连续计数，共享同一历史图片的处理结果、逐图进度和保留策略。新请求／新回合不沿用失败计数，也不把已处理图片重置回第一档。
- 正在重试的同一逻辑请求加入新图，不清零其图片失败计数。协议切换、连接重建及 sticky-break 本身不重置计数；触发它们的失败仍按上表分类。1009 本次属于不计入的失败，按连续计数规则清零。
- 一轮处理推进一个有实际缩减效果的档位，更新请求后重试；某档无收益或不适用时，在本轮直接检查下一档，不额外等待三次。不能因跳档越过受保护图片的质量底线。
- 先尊重用户取消、官方上下文返回和显式有限／零重试预算。没有下一次发送机会时，不为降档额外增加尝试或改写图片；同一失败只按 3.1 等待一次，图片处理不叠加或跳过等待。
- WS 连续三次转 HTTP 与图片连续三次触发可以发生在同一恢复轮，按 3.4 配合；图片处理本身不重置 WS、不擅自丢弃必要 `previous_response_id`，连接和粘连恢复仍由第三节管理。

#### 图片分组与保护

| 分组 | 判定与处理 |
|---|---|
| 可降级旧图 | 已完成查看、当前任务不再依赖其细节的历史图片；优先处理，最终可淘汰 |
| 受保护图片 | 最新尚待分析的完整批次、此前尚未完成分析的图片、当前任务明确需要比较或引用细节的参考图；旧图无进一步缩减空间后才处理，禁止占位 |

- 一次用户提交或一次工具返回的多张图构成一个批次，不能静默只保留最后一张。
- 新图出现、请求发送成功或时间经过，都不能单独证明此前图片已完成分析或不再必需；没有完成和解除依赖的依据时继续保护。
- 受保护图片不受“最近五张”的数量上限约束。只对其发送副本尝试第一至第四档，保留整批及当前任务所需的可读性，不允许进入第五、六档的占位淘汰。
- 当前单张大图或整批必需图片本身超限时，同样有第一至第四档路径；达到质量底线仍失败，明确提示拆批或调整线路／服务端限制，不静默删图。

#### 六档内容

| 档位 | 发送副本处理 | 质量与范围要求 |
|---|---|---|
| 第一档：无损重编码 | 保持原始宽高，尝试更紧凑的无损编码 | 保持画面和透明效果；实际发送字节减少才采用 |
| 第二档：同尺寸高质量有损 | 保持宽高；不透明图候选 JPEG `q95` | 尽量保留文字、代码、数字和细线；透明图使用支持透明的编码，不直接转 JPEG 或擅自铺背景 |
| 第三档：中等分辨率 | 等比例缩至最长边不超过 `1280px`，不透明图候选 JPEG `q85` | 不裁切、不拉伸、不放大小图；保留布局、主体及可读的主要文字 |
| 第四档：历史缩略图 | 等比例缩至最长边不超过 `768px`，不透明图候选 JPEG `q75` | 保留整体布局、主体、颜色和历史视觉线索，不保证精确文字／数值可读；受保护图片仍受质量底线约束 |
| 第五档：保留最近五张 | 保留最近五张真图与全部受保护图片的并集；其余满足淘汰条件的旧图替换为 `1×1` | 保留图维持已有处理质量，不恢复原图；实际保留数可超过五张 |
| 第六档：只保留必需图片 | 取消最近五张的额外保留，从最老的可淘汰图片开始替换为 `1×1` | 只保留任务必需图片为最终目标；不淘汰受保护图片，不跳过新增旧图自己的前四档处理 |

**参数状态：**`q95`、`1280px/q85`、`768px/q75` 是初始测试候选，不是已验证的生产值。文字截图、普通画面和透明图片分别做清晰度验收；编码必须为实际发送线路支持的格式。实施时记录所用格式和参数，通过后再更新参数状态，不能仅凭体积收益宣布质量合格。

- 第一至第四档可成批处理符合条件的旧图，优先从较老图片推进；每张图从自己的当前进度前进一个有效档，不因其他图片已到高档而直接跳到占位。
- 每次变换从保留的原始内容生成，不能对已有损结果反复有损压缩；同图、同参数复用固定缓存。实际结果更大或无收益时不采用，并记住该无效尝试。
- 图片本来小于目标尺寸时不放大，仅检查编码收益。图片尺寸错误可跳过只改变编码的无效步骤。
- 第五档按当前仍有真图的图片加入会话顺序取最近五张，再与受保护集合取并集；不为凑够五张恢复占位图。第五档已经使用 `1×1` 淘汰更早图片，它是全面淘汰非必需旧图前的过渡档。
- 示例：真图为 1～10，最近五张为 6～10，第 2 张仍为必需参考图；第五档保留 2、6、7、8、9、10，其余已满足淘汰条件的旧图可占位。
- 第六档在已知请求预算时从老到新处理，达到预算就停；预算未知时每轮先淘汰一张，再发送观察，不一次清空所有非必需真图。
- 已知预算时，任何档位达到预算即停；预算未知时，取得本轮有效结果后发送，仍连续失败三次才继续。不得把某次失败体积直接当成服务端准确限额。

#### 最高档与新增图片

- 分开保存每张图第一至第四档的处理进度、无效档尝试，以及第五／六档的历史保留策略；不能只用一个全局“已到顶”数字判定不再处理。
- 新图先按任务需要保护；后来成为可降级旧图时，从自身进度处理，经过前四档的有效处理或明确无效跳档后，再参与当前保留策略。
- 当前处于第五／六档时，尚未走完前四档的新旧图暂不满足淘汰条件；“保留五张／只留必需图”是这些图片处理完后的目标，不允许为了立即凑数跳过渐进过程。
- 例：A、B 已到第六档，新增 C、D 后仍应保护 C、D；C、D 完成分析且不再必需后，再次三连失败时从自己的第一档开始，不能不处理，也不能直接变成占位图。
- 最高档允许重复应用于新积累的合格旧图。sampling、compact、新回合、协议切换及 provider 刷新均不自动恢复已降级或占位图片，不重复编码已完成的档位。

#### 实际请求与到顶提示

- 不修改磁盘原图；对没有独立磁盘文件的输入，保留现有原始图片内容的可恢复引用，不能只剩下低质量发送副本。
- 图片处理、工具结果合入及最终请求序列化后，核对总字节数、图片字节数、真图数量和占位图数量；不能只改档位或另一份内存历史。普通和摘要请求都必须使用最新处理结果。
- `detail="high"` 不是文件字节压缩的替代品；仅调整 detail、固定保留若干张或看到升档提示，均不能证明最终请求满足预算。
- 无图、无进一步缩减空间或达到受保护图片的质量底线时，不执行空操作，不重复编码或重复追加图片处理记录；明确显示当前处理状态、请求体积、图片数量和“已无法继续压缩”。
- 对仍在重试流程中的请求，六档到顶不停止默认无限重试，也不绕过有限／零预算。`response.failed` 处理到底线仍失败时，显示“图片缩减未解决该失败”，保留实际错误类别；原因不明仍标未知，不能宣称已定位根因。
- 无图请求仍失败时，提示检查其他请求内容或链路；不继续显示正在压图。状态仅按 3.7 临时展示，不写入模型上下文。

**占位图有效性**

- 第五、六档统一使用有效的 `1×1` PNG；必须通过 PNG 校验及完整像素解码，不能只检查 Base64、文件头或尺寸。
- 占位图预先生成并验证，运行时复用；普通请求和各压缩入口使用同一份数据。
- 占位表示移除视觉内容，不是仍可分析原图的压缩；保留相关文字、消息顺序和工具调用关系。不得因占位图损坏而改成 omit 文案。

### 6.2 持久化与重放

实际采用图片变化时同步更新发送使用的历史状态，沿用 `RolloutItem::ImagesShrunk` 的轻量记录职责，并显式区分旧四档记录与新六档记录。新格式须能表达图片稳定标识、逐图处理进度及参数、结果／原始内容引用、保留策略和实际变化；不再要求只保存 `tier`、`changed`，具体 Schema 在实施时确定。

- 不保存全量历史快照，不在每次降档记录中重复嵌入原图或处理图 Base64；复用图片内容存储和缓存引用。
- 记录只包含实际变化及恢复所需状态；无变化不重复追加。图片身份不能只依赖易变的数组位置，必须能对应同一消息或工具结果中的同一图片。
- 新记录按实际处理对象和参数恢复，不能在 resume／fork 时用恢复后的“最近五张”重新决定淘汰对象，误伤后来加入的图片。
- 原有无版本字段的 `tier/changed` 记录按旧四档语义重放，不把旧第 3／4 档解释成新缩放档；旧保留 N 张计算只在其原有历史重放时点执行。旧记录的结果成为新策略的起点，不恢复已淘汰图片。
- 恢复新记录时优先复用已有发送副本；缓存缺失时可从保留原始内容按记录参数重建副本，不覆盖用户图片、不重新决定档位。源内容不可恢复时明确报出缺口，不静默恢复更大原图或丢图。
- 第一至第四档的重放须恢复相同编码／尺寸结果，第五、六档恢复已记录的占位结果；重复重放幂等。恢复图片保护信息，不能仅因重启或 fork 把未完成分析的图片认定为可淘汰。
- 不调用 `image_preparation::prepare_response_items` 做历史重建；该发送前步骤可能把无法处理的图片改成文字，不能用它改变历史。
- 恢复旧会话时，ImagesShrunk 重放使用已验证的占位图；历史快照中已有的旧损坏占位 URL，按精确匹配替换到内存历史。兼容处理不修改原始会话文件、用户图片或其他无法解码图片。
- 普通请求成功、compact 成功／失败／取消、协议及 provider 切换后均保留已采用的图片状态；摘要结果仍按 3.6 成功后提交，图片记录不冒充摘要。

ImagesShrunk 不是 compact 基线：

- 不推进 compact 窗口、不显示 compact UI、不参与 fork 边界或回合计数、不截断重放范围。
- 记录按图片元数据增量增长，不随历史正文或图片字节大小增长；小批次保持 KB 量级，大批次仅增加必要元数据，不用全量快照满足恢复要求。

paginated 会话通过 fork 派生新线程，原线程历史保持不变；legacy rollout rollback 只保留兼容验证，不作为当前主路径验收。

### 6.3 必须保留的历史经验

**清单与验收｜2026-08-15**

- 图片梯子必须是独立编号功能。
- 当时第 3 项负责“溢出仍可重试”，第 18 项负责“如何瘦身旧图”；现行第 3 项按 3.1 保留官方上下文处理边界，第 18 项统一引用 6.1 的六档策略，不各自维护重复规则。
- 验收实际发送请求：保留图片符合当前档位，替换图片可完整解码为 1×1 PNG；请求变小、字符串变化或出现升档提示不能单独判通过。
- 因占位图损坏而转成 omit 文案，判为失败。
- 当时覆盖旧第三、四档、全 high 图片直接跳档、WS 转 HTTP 及旧会话 resume／fork；现行六档验收按 6.4，旧四档只保留历史记录兼容检查。
- 按第八节使用本轮云端构建的 exe，在隔离的故障会话副本上验证压缩完成并继续回答；未通过前不得宣称原断流问题已修复。

**占位图损坏｜2026-10-08**

已确认旧占位 PNG 校验及解压失败。0.161.0 将图片梯子接入压缩后，故障会话替换了 20 张图片，随后持续断流。图片缺陷已确认；其与服务端断流的直接因果，仍需修复前后对照验证。

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

### 6.4 六档策略验收

以下为实施后的通过标准，本次文档更新不代表已执行。按 8.4 选取本次直接受影响的专项，使用本轮云端 artifact 的真实 exe 与实际发送请求验证。

| 验收点 | 通过标准 |
|---|---|
| 错误触发 | 413、明确 too large、尺寸超限、明确上传超时均累计三次；所有 `response.failed` 包括无 error、格式异常、泛化错误和已解析出的鉴权／限流／额度／过载／策略错误都计入，不能因映射后丢失事件身份漏计 |
| 排除与官方上下文边界 | 普通断线、阶段不明超时及独立上下文错误不直接压图；`response.failed` 内上下文超限仍计入，但官方返回与压缩不被截住，不为凑三次额外重试；1009 本次只转 HTTP |
| 计数与预算 | 前两次不降档，第三次处理；混合符合条件错误可累计，不符合条件错误清零；同一次失败多层只计一次；新图不重置同一请求计数；新请求重置计数但保留图片状态；默认无限、有限和零预算分别有效 |
| 六档与收益 | 每档实际请求中的编码、尺寸及保留范围正确；无收益跳档不额外等待，结果不变大；图片完整解码、透明效果保留；全 high 图片仍能进行字节压缩，不以 detail 或提示代替验收 |
| 清晰度 | 打开原图与各候选发送副本对照；文字截图、数值、细线、普通画面和透明素材按任务用途验收，记录实际参数及允许最低档；未经验证不能标记候选参数可正式使用 |
| 最近五张与批次保护 | 第五档保留最近五张真图与全部必需图片的并集，可超过五张；第六档只淘汰非必需旧图，未知预算每轮先一张；多图批次、未完成分析图片及旧参考图不被静默丢弃或占位 |
| 到顶新增图片 | 先到最高档，再加入新批次，解除其任务保护后继续三连失败；新图从自身前四档进度处理，既不永久跳过，也不直接占位；相同输入重复失败不重复编码和写记录 |
| 单图与质量底线 | 单张大图及受保护整批本身过大时可进入第一至第四档；达到底线仍失败则提示拆批或调整线路，不占位；无图片、全占位及无可缩减空间提示准确 |
| 普通与摘要请求 | sampling、自动／手动 local compact、remote compaction v2 的 HTTP／WS 请求使用同一图片结果；覆盖 WS 第三次失败同时转 HTTP 与图片处理、1009、取消、refresh、摘要失败后重试，工具不重做、等待不叠加 |
| 历史恢复 | 旧四档记录按旧语义、新六档按记录对象和参数恢复；覆盖 resume／fork、后续新图、缓存缺失重建、原图保护、保护信息恢复、摘要成功／失败／取消；不截断历史、不生成假 compact 基线、不重复图片 Base64 |
| 真实状态与证据 | 按 8.4 截图并实际读图，核对重试序号、处理状态、协议切换、到顶提示；结合请求字节数、图片解码和恢复结果判断，不能只凭构建或日志通过 |

已知请求预算时在满足预算后停止继续降质；未知预算只能报告恢复尝试结果。若原故障仍未恢复，不得将“图片已压缩”写成“原故障已修复”。

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
- **GitHub build：** 仅构建 `x86_64-pc-windows-msvc` 的 `codex.exe`、打包并上传独立 artifact；不运行测试、格式检查或 lint，不直接发布 Release 或 prerelease。
- **下载 artifact 验证：** 使用该 run 下载的真实 exe，执行 `--version`、`--help` 和相关行为验证。
- **GitHub promotion：** 验证通过后，由 workflow 从已验证 artifact 创建或更新正式 Release 并附加资产，禁止本地上传。
- **重新下载 Release 核验：** 核对 GitHub asset digest，以及 ZIP、exe 的 SHA256；与已验证 artifact 一致时，仅运行 Release exe 的版本和帮助检查。

失败后按原因重试对应阶段。程序或构建输入变化时重新构建；仅测试脚本、环境、下载或发布操作失败时，修复后复用来源明确且哈希一致的本轮产物，重跑受影响步骤。

所有 GitHub CLI 查询与触发显式指定：

`--repo dqIndieGames/codex`

### 8.2 验证来源

- 本地旧 exe、其他 run 的 artifact、源码静态检查和本地编译产物，不能代替本轮 artifact 验证。
- 发布后的最终检查必须使用重新下载的 Release asset，不能继续拿 Actions artifact 代替。
- Actions artifact 不会自动出现在 Releases 页面，必须经过独立 promotion。
- 编译证据来自 GitHub Actions；功能测试证据来自本地运行该 run 下载的真实 exe。

### 8.3 历史经验

- **2026-06-02：** 必须下载云端产物验证真实 exe，不能拿源码或本地旧文件代替。
- **2026-06-03，合入 `rust-v0.136.0`：** 只更新版本号不够，须逐项核对清单、版本身份、历史发现、日志、node_repl 和 runtime 清理。
- **2026-06-14：** 明确 build 与 promotion 分离；构建成功不等于可以发布。
- **2026-08-16：** 原记录指出，2026-08-07 的 compile-only 调整曾从 `local2-minimal-windows-release.yml` 删除回归和 smoke，导致 rollout / resume 未经验证即可发版。后来恢复测试并补充图片梯子、重放回归；不能再次用“能编译”代替这些检查。

### 8.4 简单快速的验证准则

- **纯文档修改：**只检查内容、格式、引用和唯一性，不构建、不跑功能测试。
- **云端构建：**仅准备编译环境、编译 Windows `codex.exe`、打包并上传 artifact；不运行功能测试、回归测试、smoke、格式检查或 lint。
- **本地验证：**只运行本轮下载的 exe，采用“基础检查＋关键功能快测＋受影响专项”；不在本地编译或运行 Rust 测试。
- **控制规模：**基础检查与关键快测以 **5～10 分钟**为安排目标，不含下载、脚本修整和异常排查；到时不能直接判通过。
- **复用验证：**优先使用现有脚本和模拟服务，合并可共用的测试流程；不默认跑完整矩阵，不建设额外测试平台。
- **减少等待：**超时测试优先使用已有的短超时配置；重试等待按 3.1 使用真实时间验证。模拟服务至少覆盖 `Retry-After: 2`、`10`、`30` 和无有效建议时的 `10s`，防止固定等待实现蒙混通过；不为提速修改产品行为。
- **按风险补测：**每个验收点默认一轮；只有新改动、失败或明确未解决风险才追加。逻辑检查本轮 artifact 的黑盒行为、实际请求和持久化结果。
- **真实视觉验收：**涉及终端标题、重试、压缩、完成或取消状态时，运行本轮 artifact 的真实 TUI，截图并实际打开检查目标状态和画面四边；标题验收必须包含真实窗口标题栏或标签页标题。日志、OCR、OSC 输出或快照测试不能替代读图，本机 GUI 验证不得抢占用户当前桌面前台。
- **本轮专项：**覆盖活动请求中刷新、同地址换 token、当前回合服务层切换、压缩失败后的历史保护，以及默认无限、有限和零重试预算；与图片修复共用流程时，同时执行第六节验收。
- **避免重复：**Release 回下载后，ZIP 和 exe 哈希均与已验证 artifact 一致时，仅检查版本和帮助，不重跑整套快测。
- **如实记录：**区分通过、失败和本次未执行；快测通过不能写成全部功能完整实测通过。

## 九、合并上游后的验收矩阵

本表列出完整验收范围。每轮按 8.4 选择基础检查、关键快测和受影响专项，不默认跑完整表；选中项目的通过标准仍以对应正文为准。

| 主题 | 必验场景与关键证据 |
|---|---|
| 版本身份 | CLI、doctor 及 runtime details、TUI、app-server initialize、daemon / remote-control、登录欢迎文案、历史、rollout 和升级提示；展示版本与协议裸版本分别验证 |
| 服务端版本 | 检查完整 User-Agent、本程序附加版本段及遥测版本；覆盖本次受影响的 HTTP／WS、登录、辅助接口和云任务。远程控制注册版本修复时，按 8.4 使用本轮 artifact 的真实 exe，捕获实际注册请求，确认 JSON 中 `app_server_version` 等于该构建的 Cargo 裸版本且无 `-local3`；仅检查 UA 或源码赋值不能判通过 |
| 首次清单 | 新线程、Clear、重复输入、恢复旧线程、fork、子会话、多段 / 富文本 / 附件；只在规定入口触发，包含第 18 项 |
| 重试等待 | HTTP request、SSE／WS、各 compact、realtime、WebRTC 均执行 3.1；覆盖有效秒数、HTTP-date、零值、缺失／无效值和已支持的限流文案等待；核对失败响应与下一次请求的实际时间，不能只验证字段解析 |
| 限流错误封装 | 覆盖 HTTP 429、SSE／WS 的 `response.failed` 和 `error`；验证各 header 位置、读取优先级及 `turn_rate_limit_exceeded`；收到失败事件后及时进入重试处理 |
| 等待与恢复衔接 | 覆盖 WS → HTTP、中转粘连恢复、取消和 provider refresh；同一次失败不重复等待、不绕过应保留的截止时间；默认无限、显式有限／零预算及累计序号保持原规则 |
| 流式配置超时 | 首次与重试；默认值和自定义值；首事件及后续空闲均按配置；无额外固定上限，持续输出不按总时长中断 |
| 响应头与建连 | HTTP 响应头等待无 local3 固定看门狗；WS 建连与流式等待分开；检查取消、refresh、断流和各自连接超时 |
| WS 协议选择 | 普通请求与压缩规则相同：1009 不处理图片，直接转 HTTP；其他失败连续 2 次仍用 WS，第 3 次失败后转 HTTP；官方和中转分别验证；realtime 不转 HTTP，原本使用 HTTP 的入口保持 HTTP |
| WS 恢复结果 | 转 HTTP 后，本会话的普通请求、压缩及此后新开的子代理和审核会话保持 HTTP，不回 WS；新开会话或重启后按配置恢复；provider 刷新不解除锁定；工具不重做、续链完整、取消有效、累计序号不归零 |
| WS 真实界面 | 使用下载的 GitHub artifact exe；截图并读图确认重试、图片升档及协议切换提示，结合请求证据验证 WS → HTTP → 本会话后续请求保持 HTTP、新开会话恢复 WS 的完整流程 |
| 中转粘连恢复 | 503 / 502 / 504、断流、握手、capacity；第 3 / 6 / 9 / 12 次；默认官方地址、中转、全量重放与工具续链 |
| 上下文压缩 | 保留对应官方版本的自动压缩及超限恢复／返回，不被通用无限重试截住；各入口覆盖 HTTP / WS 六档策略、1009 直接转 HTTP、三次失败转 HTTP、混合错误、显式预算、取消和历史隔离；`response.failed` 内超限计数不增加官方尝试次数，不得发送处理前的旧图片副本 |
| 实时语音连接 | realtime 直连 WS 与 WebRTC sideband 分别检查；重试、每 3 次内部恢复轮次、临时提示、配置刷新中断及 session id 不变 |
| 重试显示与历史隔离 | HTTP 和 stream / WS 均显示连续 1..6；不暴露哨兵值；重连和 refresh 不回绕；无逐次普通错误日志，不进历史 / fork / replay |
| 旁路边界 | `/models`、文件 URL 生成不强制套主链 retry；鉴权隔离和安全错误提示仍检查 |
| Provider 刷新 | app-server、TUI / console、exec、当前及新 subagent、agent_jobs、tray、无实例；无报错刷新、同地址换 token、换地址、重试中刷新及各 scope |
| Provider token | 有 / 无 token 双向切换、空值、与 `env_key` 同时配置、全局登录态存在；覆盖聊天、压缩、realtime、HTTP / WS、`/models` 和刷新后请求 |
| 服务层 | 顶层未配、开启、关闭；Fast / Flex / None；profile 不覆盖，刷新后生效 |
| 历史会话恢复 | 历史列表、最近会话、resume 选择器 / 最近会话、deep link、已加载线程；下一请求使用当前 provider；fork 选择范围仍按当前 provider 过滤 |
| 多账号 | 默认及两个命名账号、中文名、非法名、并发、API key / ChatGPT、登录 / 状态 / 刷新 / 登出、TUI / embedded app-server、共享 daemon 和拒绝入口 |
| 多账号请求一致性 | 同条件比较默认与命名账号的实际请求；覆盖真实启动路径、HTTP／WS、认证隔离及非认证字段一致性。遥测、MCP、401、并发和旧后台状态按改动范围补测，执行 8.4 的快速验证规则 |
| 标题账号／Provider 标识 | 官方默认账号显示 `[1]`，命名账号复用已有序号；第三方 `yunyi` 显示 `[y]`、`wanfeng` 显示 `[w]`，同时指定 `--account` 也不叠加数字。验证 provider 切换、状态更新及 resume／fork 后前缀正确且唯一；账号编号相关改动另验多窗口、重启和并发分配。使用真实标题截图并打开检查 |
| 401 文件重读 | 连续至少 4 次 401；跨第 3 次恢复仍有效；HTTP、Responses / realtime 握手；File / Auto、命名账号、坏文件、身份不匹配及非文件凭据 |
| 图片触发与计数 | 按 6.1／6.4 验证 413、明确过大／尺寸限制、上传超时、全部 `response.failed`，不以具体错误码排除失败事件；混合累计、非触发错误清零、多层去重、有限／零预算及官方上下文边界正确；WS 1009 本次不计入 |
| 六档图片处理 | 按 6.4 验证无损、同尺寸有损、中等分辨率、缩略图、最近五张加保护集合、只保留必需图片；无收益跳档、候选参数清晰度、透明图、单图、无图及质量底线均有证据；最高档新增图片仍从自身进度处理，不永久耗尽、不直接占位 |
| 图片历史 | ImagesShrunk 区分旧四档与新六档语义，轻量保存逐图结果和恢复所需状态；resume / fork、缓存重建、后续新图、保护信息及压缩成功／失败／取消后重建一致，不冒充 compact 基线；保留 paginated 主路径、legacy 兼容和原图保护 |
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

**配置启用 WS，不等于能证明某条请求此刻仍在 WS。**普通请求与压缩满足 3.4 条件时转 HTTP，并在本会话内保持 HTTP，新开会话或重启后按配置恢复。3.1 的服务端等待建议与默认等待、图片梯子、协议切换和会话级 HTTP 锁定，均须通过实际实现与验收后才能标记完成。

## 十一、临时排查：回复完成后终端仍忙碌

**用途：**定位后台已有 `task_complete`，但终端仍转圈的问题。本节为临时诊断要求，不属于长期定制功能。

### 记录范围

- **完成事件发出及接收：**记录时间、进程标识、线程 ID、轮次 ID；每个进程首次记录构建版本。
- **界面收尾前后：**记录模型运行、审核、MCP 启动、hook、待恢复转圈及实际忙碌状态。
- **完成后再次变忙：**记录触发事件、事件所属轮次和当前轮次，区分正常新任务与旧事件残留。

### 开销与边界

- 显式开启本专项诊断，不开启全量 debug／trace，也不要求启用 analytics、feedback 或 log_db。
- 仅记录上述关键节点和相关状态变化；不逐帧、不逐 token、不新增后台轮询。
- 使用有界、非阻塞写入；日志异常不得阻塞界面或影响任务完成。
- 同进程、同轮次、同类重复异常每 10 秒最多一条，其余只累计数量。
- 专项日志总量不超过 **5 MiB**，超限轮转覆盖最旧专项日志。
- 不记录对话正文、命令输出、认证信息或模型上下文；不写入会话历史。
- 只增加诊断记录，不改变完成判断、重试或 hook 行为，不按超时强制清除忙碌状态。

### 最小验证与删除条件

- 按 8.4 执行：云端仅构建 Windows exe，本地使用下载产物快测日志关联、容量限制及正常收尾。
- 问题复现和修复验收必须查看真实终端画面，确认停止转圈且下一条消息能正常提交；未复现不得声称已修复。
- **根因修复并验证通过后，删除本专项日志代码、临时开关及仅服务于诊断的测试；保留修复代码和必要回归验证。**
- 本节届时改为简短的原因、修复及验收记录，不再要求后续版本保留这些日志。
