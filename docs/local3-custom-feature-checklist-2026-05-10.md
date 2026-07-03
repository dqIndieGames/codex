# local3 定制功能清单_2026-05-10
本清单用于在 local3 合并官方上游版本后，按用户可感知的功能结果核对本地定制能力是否仍然保留。

1. local3 版本身份保留为 `<Codex 版本>-local3`，所有用户能看到版本的位置都要显示这个本地构建后缀；原来可能被上游版本覆盖成官方裸版本，修改后 CLI、TUI、状态卡片、标题区、历史单元和升级提示都继续显示 local3 身份，这样用户能确认自己正在使用本地定制版。

2. 首次输入纯文本 `你好` 时显示 local3 功能清单，并且每个新线程只显示 1 次；原来清单可能被做成某个客户端专用提示或重复插入，修改后 brand-new thread 或 Clear 后的新线程中，首个普通用户输入恰好为 `你好` 才会在首个 assistant 主消息第一段显示全量 local3 清单，resume、continue、fork、历史线程重开、子会话和其他输入都不重复触发，这样用户首次检查定制功能时能稳定看到完整清单且不会被反复打扰。

3. 远端模型主链失败后的自动重试覆盖所有会直接终止一次 Codex 对话、compact 或 realtime 模型链路的远端/模型请求错误，不能有错误类型豁免，保持更耐用、更少打断的体验；原来只有部分远端错误会按白名单重试，修改后所有由模型服务主链、Responses HTTP、SSE、WebSocket、compact、realtime/WebSocket 或其他参与一次 assistant turn / compaction 的 Codex 请求链路返回的错误都进入普通自动重试，任何旧 `is_retryable=false`、状态码、错误码、协议层映射或错误分类都不能绕过 retry；`Selected model is at capacity. Please try a different model.`、context window、usage/quota、policy 等服务端返回错误也必须按同一 retry budget 处理；本项强制范围不包含模型目录刷新 `/models` 与 MCP OpenAI 文件上传/下载 URL 生成等旁路 HTTP 辅助请求，这些旁路可以保留自身产品口径，不作为本清单 retry / sticky-break 漏改判定，但如果它们发送 provider token 或展示用户可见错误，仍按对应的鉴权隔离和提示规则验收；所有自动 retry 等待间隔最高 `5s`，默认也按 `5s` 作为硬上限，包括 Responses HTTP request retry、stream/WebSocket retry、compact retry、realtime/WebSocket 连接 retry 和服务端 `Retry-After` 建议等待；禁止留下 `8s` 或更高的旧 cap、测试名、断言或文档口径；中间失败不写入历史，只保留可见重连提示和可诊断信息，这样用户在模型容量、临时鉴权、网络、服务抖动或其他远端异常时更少看到终态失败，也不会被越来越长的本地重试等待卡住。

4. 重试期间的可见提示、日志噪声和统计口径保持平衡；原来中间态可能刷屏或让诊断信息丢失，修改后用户仍能看到首次重连、重试次数、重试详情等提示，但所有 `willRetry=true`、`EventMsg::StreamError`、HTTP request retry、stream/WebSocket reconnect 和 compact retry 的中间失败默认不得以 `warn!` / `error!` 写入普通运行日志或 app-server stderr，也不应生成每次 retry 一条的高噪日志；只有最终失败、用户显式开启 debug/trace 诊断、或低频汇总型诊断才允许落日志，同时 retry metrics/counter 继续保留，这样用户界面和后台日志更安静，排查问题时仍有统计依据。

   - retry 中间态目标清单：retry 中间态就应该只是“内部继续重试 + TUI 上同一个状态栏数字增加”。除了必要的网络重试、等待、一次轻量状态通知、TUI 覆盖刷新和 metrics 计数，不应该再写日志、写历史、进 fork、进 replay 或污染上下文。落地时必须同时去掉 retry 中间态 `warn!` / `error!` 噪声，并把 retry 中间态从普通持久事件链改成真正的 transient status update；不能把普通 `EventMsg::StreamError` / `EventMsg::Warning` 进入 rollout/history/fork/replay 的路径当作完成证据。

5. 历史会话默认跨 provider 可发现，并且继续旧线程时使用当前顶层 provider；原来历史入口可能按 provider 收窄，修改后历史列表、最近会话、resume picker 和 `codex://threads/{id}` deep link 默认都能看到旧会话，并且恢复旧线程时不能因历史 `session_meta.model_provider`、已加载线程快照或 `thread/read` 回退继续粘住旧 provider；若旧线程 provider 与当前顶层 provider 不一致，应重建/换绑到当前 provider，做不到时必须明确提示仍在使用旧 provider，这样用户切换 provider 后仍能找回并继续之前的工作，不会误以为已经走新 provider。

6. 全局优先服务层默认开启，并允许显式恢复官方映射；原来不同配置层级可能让请求服务层表现不一致，修改后顶层未配置或设为开启时统一使用 priority，顶层显式关闭时恢复 Fast -> priority、Flex -> flex、None -> unset 的官方映射，profile 内同名设置不生效，这样用户默认获得更稳定的优先体验，也能按需回到官方行为。

7. Windows app、app server 和 TUI 默认日志降噪，运行时负担默认更轻；原来未显式设置日志时可能产生高噪声记录，修改后 Windows app/app server 和 TUI 默认只保留更安静的日志级别，显式设置后仍可打开详细日志，同时 analytics、feedback、log_db 默认关闭但可配置开启，这样用户日常使用更轻、更安静，需要排查或反馈时还能手动打开。

8. 默认开启不影响使用的批量优化，并保留即时反馈和历史安全；原来 rollout 批量 flush 与 app-server 高频通知合并默认关闭，修改后这些优化可以默认开启，但前提是输出节奏、token usage、diff/plan 更新、命令完成状态和崩溃恢复与未开启优化时保持用户可感知等价，同时必须保留显式关闭开关；这样用户默认获得更低 I/O 和更少客户端负担，但仍能看到及时刷新和可靠历史。

9. Provider refresh 的刷新范围扩大到所有正在使用的 Codex 入口，并覆盖会影响路由和速度的关键 provider 字段；原来 provider runtime 刷新只要求覆盖 `base_url` 与 `experimental_bearer_token` 两个字段，刷新结果可能只影响部分 live instance，修改后 `base_url`、`experimental_bearer_token`、`force_service_tier_priority` 与 fast mode 相关有效配置都必须对所有 app server、已经打开的 Codex 窗口/会话、`codex exec`、已打开和后续新开的 subagent、以及 agent_jobs 批量子任务尽可能更快生效，Windows tray 从 source provider 复制字段到当前 target provider 后也要触发同一刷新口径；这样用户换 URL、token、优先服务层或 fast 开关后，不同入口不会继续拿旧地址、旧 token 或旧速度/服务层策略发请求。

10. Provider refresh 不是只在 retry 时才生效，而是配置变化后面向所有 live runtime 的通用刷新能力；原来 refresh 容易被理解成“请求失败后的补救动作”，修改后只要 provider 有效配置发生变化，就应尽快刷新已加载线程、app-server runtime、console/exec runtime 和正在等待下一次请求的会话，即使当前没有 retry、没有报错、没有正在流式输出，也应让下一次请求使用新 provider 状态；这样用户主动切换 provider 参数后，不必靠失败重试或新开对话才能看到新配置。

11. 所有 Codex retry 入口都要接入 hard route recovery，且不能因为错误类型、compact 类型、retry budget 或续链场景豁免 retry；这里的 retry 入口优先指一次用户/agent turn 的模型请求链路，特别包括普通 sampling、Responses request retry、Responses stream/WebSocket reconnect、local compact、remote compaction v2 和 realtime/WebSocket 连接，compact 不是可忽略的后台清理，而是长线程继续可用的用户可见链路。retry 粘连故障的重置必须按“每连续 3 次 retry 失败就重置一次”执行：第 1、2 次只做普通重试，不重置 WebSocket session、不旋转 `prompt_cache_key`、不主动丢 `previous_response_id`；第 3 次 retry 失败处理阶段必须执行 sticky-break，将可全量重放请求的 `prompt_cache_key` 从默认 thread id 派生为带 recovery generation 的新值，并重置 WebSocket session，使紧随其后的下一次普通全量重放已经使用新 recovery generation 且不携带旧 `previous_response_id`；如果后续继续连续失败，第 6、9、12 次等每个 3 次周期也必须再次执行同样的 sticky-break 重置。HTTP 503/502/504、SSE/WebSocket 未完成断流、WebSocket handshake 失败、local compact、remote compaction v2、`Selected model is at capacity. Please try a different model.` 以及其他会终止 assistant turn / compact 的远端/模型请求错误，在 TUI、`codex exec`、subagent、agent_jobs 等入口都应使用同一恢复口径；必要时继续走 fallback transport，但不修改真实 thread id、session id 或用户提示词；`function_call_output` 等必须续接旧响应链的请求也不能成为 retry 或 sticky-break 计数豁免，实现必须保留 `previous_response_id` 续链语义或采用不破坏续链的 recovery 方式，而不是跳过 retry。compact 路径必须同时满足“会 retry”和“每 3 次 retry 触发 sticky-break”两个条件：local compact 与 remote compaction v2 都不能只做一次失败返回，也不能因为 compact 专属有限 retry budget、fallback transport 或 `stream_max_retries` 默认值在第 3 次之前截断；若确需 bounded 模式或显式 `stream_max_retries = 0`，必须在测试和文档中明确这是主动禁用/限制 retry，而不是 local3 默认体验。这样用户遇到“503 retry N (unbounded) / Reconnecting... N (unbounded) / stream closed before response.completed / Selected model is at capacity / compact retry N”时，每满 3 次 retry 都会有一次换掉有问题中转账号粘连的机会，而不是只能新开对话或派生线程。

   - realtime/WebSocket 专项口径：realtime 没有 `prompt_cache_key` / `previous_response_id` 字段，不能把 Responses 的 cache-key 旋转要求照搬成改用户 session id。普通 realtime websocket connect 与 WebRTC sideband join 都必须使用 provider retry 配置、5 秒上限和 provider runtime refresh 中断能力；第 3、6、9 次连接/握手失败时递增内部 recovery generation，丢弃半开连接并重建握手 request/header/TLS connector，用户看到 `Reconnecting realtime... N` 与 route recovery 阶段提示，真实 thread id、用户输入和用户显式 realtime session id 不变。
   - old remote compact `/responses/compact` 专项口径：它已有 request retry 与第 3 次 route recovery，仍需把 503/429/transport retry 的中间态发成 transient status，让用户看到 `503 retry N` 或 `Reconnecting... N`；这些中间态不得写入 rollout/history/fork/replay，也不得把 `/models` 或 files 旁路顺手纳入主链 retry。

12. 无 live instance 的 provider 字段复制仍视为成功，并给出明确反馈；原来没有可刷新实例时可能让用户误以为字段写入失败，修改后只要 provider 字段写入成功，即使没有任何正在运行的实例，也反馈“未刷新任何实例”，这样用户能区分“配置已保存”和“当前没有可通知的运行入口”。

13. app-server stderr 默认保持安静，只有显式配置才打开后台诊断输出；原来 `warn` 日志或 WebSocket 启动 banner 可能默认写入 stderr，修改后默认不再显示 `codex app-server (WebSockets)`、`listening on`、`readyz`、`healthz` 等后台诊断文字，只有用户在 `config.toml` 配置 `[logging] app_server_stderr = true` 后才恢复这些诊断信息，这样日常使用更安静，排查问题时仍能手动打开。

14. `node_repl` MCP 自动继承当前 local3 CLI 路径；原来用户实际运行 local3 时，`node_repl` 子进程仍可能使用 AppData 自动安装目录里的旧版 `codex.exe`，修改后启动 `[mcp_servers.node_repl]` 时会把 `CODEX_CLI_PATH` 指向当前 `Config.codex_self_exe`，这样 refresh、诊断和 app-server 行为跟当前 local3 版本保持一致。

15. app-server 退出时只补已有 runtime 引用清理，不做激进进程管理；原来 shutdown 路径可能漏释放外部 auth、apps runtime 和 skills watcher 引用，修改后主 app-server 退出时补调已有 `clear_runtime_references()`，但不新增 idle timeout，不全局扫描或 kill `node_repl.exe`，也不因为当前 UI 订阅断开就杀仍加载的线程，这样能减少残留引用，同时避免误伤正在使用的会话。

16. 配置了 `experimental_bearer_token` 的 provider 必须按 provider 自带 token 隔离发请求，不受全局 `AuthManager` 登录态影响；原来请求头虽然优先使用 `experimental_bearer_token`，但 `provider.auth()`、`api_provider()`、`/models` 拉取、auth mode、ChatGPT account header、FedRAMP header 或 attestation 仍可能间接受 `auth.json` / ChatGPT / API key 登录态污染，导致外部 provider 请求被错误路由或错误鉴权。修改后只要当前 provider 有非空 `experimental_bearer_token`，聊天请求、compact 请求、模型列表请求、WebSocket / HTTP 请求和 provider runtime refresh 后的下一次请求都必须使用 `Authorization: Bearer <experimental_bearer_token>`，并且不得从 `AuthManager` 读取或继承 auth mode、账号 ID、FedRAMP、ChatGPT backend routing 或 attestation；没有 `experimental_bearer_token` 的 provider 继续保持原有 AuthManager 行为。这样用户切到自带 bearer token 的外部 provider 时，请求只按该 provider 的 token 和 base_url 发送，不会被本机 Codex 登录态带偏。

## local3 验收矩阵（合并上游后必查）

| 主题 | 必验场景 | 通过口径 |
|---|---|---|
| 版本身份 | `codex --version`、`codex doctor --json`、TUI 状态卡、app-server initialize `user_agent`、daemon/remote-control 输出、线程历史元数据、升级提示、GitHub Release 下载后的 Windows smoke | 用户能看到的 local3 构建身份均显示 `<Codex 版本>-local3`；用于包版本、协议版本、配置锁或更新比较的裸 semver 不误加后缀 |
| 首轮 `你好` 清单 | brand-new thread、Clear 后新线程、同线程第二次 `你好`、resume/continue、fork、历史线程重开、subagent、多段输入、带富文本/附件输入 | 只有 brand-new 或 Clear 后首个普通纯文本恰好为 `你好` 时，在首个 assistant 主消息第一段插入清单；其他入口和重复输入不触发 |
| retry 5 秒上限 | Responses HTTP request retry、stream/WebSocket reconnect、local compact、remote compaction v2、realtime/WebSocket 连接、服务端 `Retry-After` | 自动 retry 等待不超过 `5s`；旧 `8s/eight seconds` 实现、测试名、断言和文档口径不得残留在 retry 语义里 |
| sticky-break 第 3 次 | HTTP 503/502/504、SSE/WebSocket 断流、WebSocket handshake 失败、realtime ordinary websocket connect、WebRTC sideband join、`Selected model is at capacity. Please try a different model.`、ordinary sampling、local compact、remote compaction v2、subagent、agent_jobs、`codex exec` | 第 1、2 次只普通 retry；第 3、6、9、12 次等连续 retry 失败处理阶段触发 sticky-break；Responses 可全量重放请求旋转 `prompt_cache_key` recovery generation 并重置 WebSocket session，可全量重放的下一次请求不携带旧 `previous_response_id`；realtime 路径递增 recovery generation、重建握手/连接状态且不改用户 session id |
| compact 专项 | local compact、old remote compact `/responses/compact`、remote compaction v2 分别构造 retryable 远端错误、capacity 错误、断流错误、默认无界 retry、显式 bounded / `stream_max_retries = 0` | 默认 local3 体验下 compact 必须 retry，并且第 3 次 retry 必须 sticky-break；old remote compact request retry 要有 transient `503 retry N` / `Reconnecting... N` 状态；bounded 或 `stream_max_retries = 0` 只能作为显式限制/禁用 retry 的配置口径，不能替代默认验收 |
| retry 范围排除 | `/models` 模型目录刷新、MCP OpenAI 文件上传/下载 URL 生成 | 这两类旁路 HTTP 辅助请求不作为本清单 retry / sticky-break 漏改 finding；但若涉及 provider token、ChatGPT account header、FedRAMP header、attestation 或用户可见错误，仍按鉴权隔离和提示规则验收 |
| retry 中间态 | HTTP request retry、stream/WebSocket reconnect、compact retry、realtime/WebSocket start retry、WebRTC sideband retry、fallback transport、provider runtime refresh 期间 retry | 用户看到连续 retry 数字和必要诊断；中间失败不进 rollout/history/fork/replay，不刷普通 `warn!` / `error!` 或 app-server stderr；最终失败和显式 debug/trace 诊断仍保留 |
| Provider refresh | app-server 已打开线程、TUI/console、`codex exec`、当前 subagent、后续新开 subagent、agent_jobs、无 live instance、Windows tray provider apply | `base_url`、`experimental_bearer_token`、`force_service_tier_priority`、fast mode 有效配置不靠失败 retry 也能刷新；无 live instance 时反馈“配置已保存但未刷新任何实例” |
| Provider token 隔离 | 带非空 `experimental_bearer_token` 的聊天、compact、realtime/WebSocket、`/models`、provider refresh 后下一次请求；移除 token 后回退 | 有 provider token 时只用 `Authorization: Bearer <experimental_bearer_token>`，不继承 AuthManager 的 auth mode、账号 ID、ChatGPT routing、FedRAMP 或 attestation；无 token provider 保持原 AuthManager 行为 |
| 历史跨 provider | history list、recent sessions、resume picker、resume last、`codex://threads/{id}` deep link、旧 provider 线程继续、fork picker / fork last | 继续旧线程默认使用当前顶层 provider；历史发现不被旧 provider 过滤；fork 边界按当前 provider 保持，不把“继续旧线程”误扩成“跨 provider 派生” |
| app-server stderr | 默认启动、`[logging] app_server_stderr = true`、WebSocket banner、`listening on`、`readyz`、`healthz`、配置错误 | 默认不输出后台诊断噪声；显式开启后恢复诊断；真正启动失败或配置错误仍能给用户看到必要错误 |
| `node_repl` CLI 继承 | `[mcp_servers.node_repl]`、本地 stdio MCP、其他 MCP server、当前 `Config.codex_self_exe` | 仅 `node_repl` 被注入 `CODEX_CLI_PATH=<当前 local3 codex.exe>`；其他 MCP server 不被全局改写 |
| 批量优化与历史安全 | rollout batch flush、app-server 高频通知合并、token usage、diff/plan 更新、命令完成状态、崩溃恢复、显式关闭开关 | 默认优化不改变用户可见输出节奏和历史可靠性；显式关闭后能回到未优化行为 |
| app-server 退出清理 | 主 app-server shutdown、外部 auth、apps runtime、skills watcher、仍加载线程、`node_repl.exe` | 只释放已有 runtime 引用；不新增 idle timeout，不全局扫描或 kill 进程，不因 UI 订阅断开误伤仍在使用的会话 |

## 2026-07-03 retry 5 秒与 sticky-break 漏改反思

- 这次漏点不是 retry 主链完全没做，而是旧文档和旧测试仍把等待上限冻结在 `8s`，导致实现、测试名、断言和 checklist 互相强化旧口径。后续凡调整 retry 等待上限，必须同时扫 `core/src/util.rs`、`codex-client/src/retry.rs`、`core/src/util_tests.rs`、`codex-client/src/retry.rs` 测试区、文档里的 `8s/eight seconds` 残留。
- sticky-break 不能只看 sampling 主链或 request retry；compact 是用户在长线程最容易触发的恢复路径，local compact 和 remote compaction v2 必须同时证明“会 retry”和“每 3 次 retry 会 sticky-break”。如果 compact 的 retry budget、fallback threshold 或配置口径在第 3 次前截断，用户仍会卡在旧粘连路由上，表面看有 hard route recovery，实际没有换掉粘连路由。
- 验收时必须区分两类结果：默认无界 retry 模式下，第 3、6、9 次等必须能触发 sticky-break；显式 bounded 或 `stream_max_retries = 0` 是主动限制/禁用 retry，不能拿 bounded 口径替代 local3 默认体验。
- realtime/WebSocket 是独立入口，不能只因为 Responses request/stream 已经有 route recovery 就判定闭合；普通 websocket connect 和 WebRTC sideband join 都要单独证明临时失败后会 retry、第三次失败会进入 recovery generation、retry 状态只 transient 展示。
- old remote compact `/responses/compact` 不能只证明 prompt_cache_key 第四次变了；还要证明前三次 request retry 的用户可见中间态存在，并且这些中间态不写历史、不污染 replay。

## 2026-07-01 experimental_bearer_token provider 隔离思路

- 目标口径：`experimental_bearer_token` 是 provider-scoped 静态 bearer token；用户能感知到的是“这个 provider 自己的 token 管自己”，不因为本机登录了 ChatGPT 或保存了 `auth.json` 就改用 ChatGPT 路由、账号 header 或其他全局登录上下文。
- 最小 hook 边界：不要只在 `resolve_provider_auth()` 修 `Authorization` header；还要阻断 `ConfiguredModelProvider::auth()`、`OpenAiModelsEndpoint::auth()` 和 `supports_attestation()` 从全局 `AuthManager` 取值。否则 header 可能是 provider token，但 `auth_mode`、默认 base_url、`/models`、ChatGPT account header 或 attestation 仍会被 AuthManager 间接污染。
- 推荐实现打法：当 `experimental_bearer_token` 非空时，provider 请求路径直接把当前 provider auth 视为 `None`，再由 provider 配置生成 `BearerAuthProvider`；保留没有 token 的 provider 继续走原 AuthManager。若要支持运行时移除 token 后回退 AuthManager，优先在 `auth()` 等读取点早返回，不要过早丢弃保存的 AuthManager 引用。
- Provider runtime refresh 必须支持“带非空 `experimental_bearer_token` 的 provider”与“无 `experimental_bearer_token`、继续使用 AuthManager 的 provider”这两类代表 provider 之间双向互刷；`openai_http ↔ yunyi` 只是当前配置里的代表样例，不应写死为特例。无 token provider 刷新到有 token provider 时，下一次请求必须进入 provider-scoped bearer token 隔离模式；有 token provider 刷新回无 token provider 时，必须清掉旧 provider token 状态并恢复原有 AuthManager / `auth.json` 行为。实现上不能把 AuthManager 全局丢弃，只能在当前 provider 有非空 token 时让请求读取点临时忽略 AuthManager；这样任意两个同类 provider 互相 refresh 都不会串旧 token、旧 auth mode、ChatGPT account header、FedRAMP header 或 attestation。
- 优先级与空值：`experimental_bearer_token` 必须按非空字符串判断；空字符串不能生成 `Bearer ` 坏请求。若同一 provider 同时配置 `env_key` 与 `experimental_bearer_token`，必须明确最终优先级；local3 目标是“有非空 `experimental_bearer_token` 就优先用它”，避免环境变量或 AuthManager 抢占。
- 验收清单：即使 AuthManager 内存在 ChatGPT token、API key、account id 或 FedRAMP 状态，聊天请求、`/models` 请求和 refresh 后请求都必须只带 `Authorization: Bearer <experimental_bearer_token>`；不得带 AuthManager 派生的 `ChatGPT-Account-ID`、FedRAMP header 或 ChatGPT auth mode；移除 `experimental_bearer_token` 后，未配置 token 的 provider 旧行为不回归破坏。

## 2026-05-30 回归经验

- 503、429、402、网络断开等请求级 retry 不能只留在 telemetry/log；用户必须看到 `willRetry=true` 的中间态提示，提示里至少包含 HTTP 状态码、当前 retry 次数、最大 retry 次数和可诊断 details。否则用户只会感觉“卡住了/后台在重试但没告诉我”。
- HTTP request retry 和 stream/WebSocket retry 是两条不同链路；隐藏 WebSocket 首次重连提示时，不能顺手把 HTTP 503 这类请求级 retry 也隐藏掉。
- Provider 的 `base_url` 与 `experimental_bearer_token` 写入后，不能只清 plugin/skill cache；必须刷新 loaded threads 的 provider runtime。否则已经打开的窗口或会话会继续拿旧 URL/token 发请求。
- Provider refresh 的结果要区分两件事：配置字段是否已经保存、当前是否真的刷新到了 live instance。没有 live instance 时仍然是保存成功，但必须明确提示“未刷新任何实例”。
- Windows tray 的 provider apply 必须优先调用 app-server 控制面的 `apply_provider_runtime_from_effective_provider`，让实际运行中的 app-server 自己完成 effective config 读取、写入、reload user config 和 loaded thread refresh；只有所有 live instance 都明确不支持该控制操作时，才回退到 Python 直接改 `config.toml` 再发 `refresh_all_loaded_threads`。否则真实 `codex.exe` 被 IFEO、wrapper、runtime selector 或 Windows App 重定向后，用户会看到“配置像是改了，但当前会话仍拿旧 URL/token”。

## 2026-05-31 retry 错误显示经验

- 无界 retry 不能把内部哨兵值显示给用户；`u64::MAX` 只代表“不设上限”，用户界面禁止出现 `18446744073709551615`，应显示为 `unbounded` 或省略最大次数。否则用户会把内部实现数字误认为异常错误码。
- retry 标题和详情要分工清楚；标题说明“HTTP 状态 + 当前第几次 retry + 是否无界”，详情说明“状态含义 + 正在自动重试 + 安全诊断字段”。不能出现标题是 `429 retry 4/18446744073709551615`、详情只有 `http 429` 这种难以排查的组合。
- telemetry/log 的短字符串不能直接当用户详情；`http 429`、`http 503` 适合内部统计，不足以给用户解释发生了什么。用户可见详情至少应包含 `HTTP 429 Too Many Requests, retrying` 或 `HTTP 503 Service Unavailable, retrying` 这类人话状态。
- HTTP response body 不能原样放进用户详情；body 可能包含 token、API key、auth error 或 provider 返回的敏感内容。允许展示的诊断信息应限制在状态码、标准 reason、去 query/userinfo 的 endpoint、request id、cf-ray、auth error 和 auth error code 等安全字段。
- app-server/TUI/Windows App 的 `willRetry=true` 中间态必须继续可见；修复文案时不能回退成只写 telemetry/log，也不能把请求级 HTTP retry 和 stream/WebSocket reconnect 混在一起隐藏。

## 2026-05-31 local3 版本身份与 GitHub 打包经验

- local3 版本身份不能只查 `codex.exe --version`；`codex doctor --json`、doctor runtime details、`codex-app-server --version`、app-server initialize 返回的 `user_agent`、daemon/remote-control JSON、device-code 登录欢迎文案、线程历史元数据和 rollout 元数据都是用户或客户端能看到的版本面，也必须显示 `<版本>-local3`。
- app-server 的 `user_agent` 不是普通 telemetry 字符串；daemon 会从 initialize 响应里解析它，再显示到 doctor 的 `app-server version` 详情里。这里如果继续使用裸 `CARGO_PKG_VERSION`，用户会看到 CLI 是 local3、后台 app-server 却像官方裸版本。
- `cli_version`、`client_version`、`app_server_version` 字段要按用途区分：进入历史列表、远端诊断、daemon JSON 或用户界面的用 display version；用于更新比较、Python wheel 版本、配置锁、OpenTelemetry service_version、OAuth/device-code 协议参数的仍用裸 semver，避免破坏包版本和协议兼容。
- GitHub workflow 不能把 `GITHUB_REF_NAME` 当 Python wheel 的 Codex 版本；手动从 `main` 分支触发时它是 `main`，不符合 PEP 440，会导致 wheel 打包失败。云端打包应从 `codex-rs/Cargo.toml` 读取裸 semver，再把 local3 只用于用户可见版本输出。
- Windows release smoke test 必须明确断言 `-local3`，不能只检查输出里包含裸 `0.135.0`；否则 `0.135.0` 和 `0.135.0-local3` 都会通过，无法阻止本地身份后缀回退。
- GitHub Actions artifact 只是单次 workflow 的临时产物，不会自动显示在 Releases 页面；如果用户要从 Releases 页面下载，云端编译成功后必须单独创建 GitHub Release，并把已验证的 artifact 上传为 release assets。

## 2026-06-02 app-server stderr 与 node_repl 回归经验

- 检查 app-server stderr 降噪时不能只看 tracing layer；WebSocket 启动 banner 里的 `listening on`、`readyz`、`healthz` 也是 stderr 输出，必须一起验证默认静音和显式开启两种状态。
- `node_repl` 的 CLI 路径覆盖必须按 server 名精准限制在 `node_repl`，不能全局改写其他 MCP server 的 env；否则可能破坏用户自己配置的 MCP 环境变量。
- 云端构建验证必须下载 GitHub Actions 产物后测实际 `codex.exe`；不能用源码静态检查、本地路径旧 exe，或本地编译产物替代 release smoke test。

## 2026-06-03 136 更新与 refresh/retry 回归经验

- 更新到官方 `rust-v0.136.0` 时不能只合版本号；local3 清单、显示版本、历史跨 provider、日志降噪、node_repl 继承和 runtime 清理都要按用户可见结果逐项复核。
- Provider refresh 必须能打断所有正在进行的 retry：HTTP 503/429/402、无界 503、网络失败、SSE 断流/空闲、WebSocket 503/426/401 都要验证旧 endpoint/token 不再继续增长，并切到新 endpoint/token。
- Provider refresh 的覆盖字段必须包含 `base_url`、`experimental_bearer_token`、`force_service_tier_priority` 和 fast mode 有效配置；refresh 触发也不能依赖 retry，用户主动改配置后所有 live runtime 都应尽快刷新。
- retry 粘连故障的补救核心是 sticky-break：所有 Codex retry 入口都要覆盖，所有远端/模型请求错误都要 retry，不能按错误类型、状态码、协议映射或续链场景保留豁免；连续 retry 计数每满 3 次都必须重置一次 sticky-break，第 3、6、9、12 次等每个 3 次周期都要旋转 `prompt_cache_key`、重置 WebSocket session，并让紧随其后的可全量重放 recovery 请求清掉旧 `previous_response_id`；禁止把真实 thread id/session id 改掉，也不要通过改用户 prompt 来“换内容”；`function_call_output` 等必须续链场景不得强行丢续接 ID，但也不能因此跳过 retry 或 sticky-break 计数，必须用不破坏续链语义的方式继续重试。
- 用户报告的 `503 retry N (unbounded)` 必须单独建无界场景覆盖；不写 `request_max_retries` 才是 release exe 的正式无界 retry 口径，不能只用有界 retry 代替。
- 分开刷新要保留旧的全刷，同时新增 `console` 与 `appServer` scope；动态验收至少要证明 `appServer` 能刷新 Windows App app-server thread，`console` 不误刷 app-server thread。
- GitHub CLI 查询和触发必须显式带 `--repo dqIndieGames/codex`；否则可能落到 `openai/codex`，导致 run/release 证据查错仓库。
- 禁止本地编译时，编译证据只能来自 GitHub Actions；本地只下载 release zip，比对 GitHub asset digest，再用下载的 `codex.exe` 做真实 smoke 与 refresh 矩阵。

## 2026-06-14 历史线程 provider 重绑经验

- `codex://threads/{id}` 不能只当作“接回旧 loaded thread”；当用户已切换当前顶层 provider 时，恢复旧线程也要验证实际请求 provider 是否同步切换，否则旧线程会因历史 `session_meta.model_provider` 或 loaded `config_snapshot.model_provider_id` 继续走旧 provider。
- 历史会话“跨 provider 可发现”和 fork 边界要分开验收：resume picker、resume last、deep link 默认应不带 provider filter；本地 fork picker / fork last 仍要按当前 provider 过滤，避免把“继续旧线程”需求误扩成“从旧 provider 派生新线程”。
- 恢复旧线程不能只看列表能不能选中；app-server resume 端必须确认显式 request provider 覆盖历史 `SessionThreadConfig.model_provider`，TUI 同一 thread resume 遇到 active provider 不同要先 shutdown 再 cold resume/rebind，否则用户以为切到新 provider，实际下一轮仍走旧 provider。
- `Selected model is at capacity. Please try a different model.` 必须作为 typed `ServerOverloaded` 进入 retry / hard route recovery；只改 UI 文案或只让 `is_retryable()` 返回 true 不够，必须覆盖普通 sampling、local compact、remote compact v2 三条会把错误变成终态 `ErrorEvent` 的链路。
- capacity 可以穿透 zero/finite stream budget 争取第 3 次 sticky-break，但不能把所有 retryable 错误都顺手改成无界；compact 普通 500/timeout/断流仍要使用 compact 专属有限 terminal budget，否则历史压缩失败会让用户看到长时间卡住而不是明确失败。
- 禁止本地编译时，验收必须走“subagent 静态反推 + GitHub Actions 远端 prerelease + 下载 release zip 运行下载的 `codex.exe --version` / `--help`”；旧 run 或本地旧 exe 不能替代新提交产物 smoke。

## 2026-06-28 retry 计数器回绕修复经验

- 用户可见 retry 序号必须和内部 route recovery 局部计数分离；第 3、6、9 次 sticky-break 可以重启 HTTP 请求、重置 WebSocket session 或刷新局部 fallback 计数，但 `503 retry N (unbounded)` / `Reconnecting... N (unbounded)` 的 N 必须继续累计为 4、5、6，不能回到 0 或 1。
- HTTP request retry 的显示计数要在 telemetry 层加上已被 route recovery 消费的 retry offset；只减少下一轮 `max_attempts` 不够，否则新建 API client 后 `on_request_retry(1, ...)` 会让 UI 重新显示第 1 次 retry。
- stream/WebSocket retry 要保留两个计数器：内部 `retries` 继续用于 backoff、fallback threshold 和 sticky-break 周期；独立的 display retry 只用于用户提示和日志显示，provider runtime refresh、fallback transport 或 session reset 不得清零 display retry。
- 验证必须同时覆盖 request-layer 与 stream/WebSocket，至少断言连续 1..6 可见序号、不出现 `18446744073709551615`，并在最终 release smoke 中从 GitHub Release asset 重新下载 exe 后复核，不能用 Actions artifact 或本地旧 exe 代替。
