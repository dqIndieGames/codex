# local3 定制功能清单_2026-05-10
本清单用于在 local3 合并官方上游版本后，按用户可感知的功能结果核对本地定制能力是否仍然保留。

1. local3 版本身份保留为 `<Codex 版本>-local3`，所有用户能看到版本的位置都要显示这个本地构建后缀；原来可能被上游版本覆盖成官方裸版本，修改后 CLI、TUI、状态卡片、标题区、历史单元和升级提示都继续显示 local3 身份，这样用户能确认自己正在使用本地定制版。

2. 首次输入纯文本 `你好` 时显示 local3 功能清单，并且每个新线程只显示 1 次；原来清单可能被做成某个客户端专用提示或重复插入，修改后 brand-new thread 或 Clear 后的新线程中，首个普通用户输入恰好为 `你好` 才会在首个 assistant 主消息第一段显示全量 local3 清单，resume、continue、fork、历史线程重开、子会话和其他输入都不重复触发，这样用户首次检查定制功能时能稳定看到完整清单且不会被反复打扰。

3. 远端请求失败后的自动重试覆盖所有错误，保持更耐用、更少打断的体验；原来只有部分远端错误会按白名单重试，修改后所有远端请求错误都进入普通自动重试，所有 retry 等待间隔最高 `8s`，包括 HTTP 请求 retry、stream/WebSocket retry、compact retry 和服务端 `Retry-After` 建议等待；中间失败不写入历史，只保留可见重连提示和可诊断信息，这样用户在临时鉴权、网络、服务抖动或其他远端异常时更少看到终态失败，也不会被越来越长的本地重试等待卡住。

4. 重试期间的可见提示、日志噪声和统计口径保持平衡；原来中间态可能刷屏或让诊断信息丢失，修改后用户仍能看到首次重连、重试次数、重试详情等提示，日志不再被中间失败刷屏，同时 retry metrics 继续保留，这样用户界面更安静，排查问题时仍有统计依据。

5. 历史会话默认跨 provider 可发现，并且继续旧线程时使用当前顶层 provider；原来历史入口可能按 provider 收窄，修改后历史列表、最近会话、resume picker 和 `codex://threads/{id}` deep link 默认都能看到旧会话，并且恢复旧线程时不能因历史 `session_meta.model_provider`、已加载线程快照或 `thread/read` 回退继续粘住旧 provider；若旧线程 provider 与当前顶层 provider 不一致，应重建/换绑到当前 provider，做不到时必须明确提示仍在使用旧 provider，这样用户切换 provider 后仍能找回并继续之前的工作，不会误以为已经走新 provider。

6. 全局优先服务层默认开启，并允许显式恢复官方映射；原来不同配置层级可能让请求服务层表现不一致，修改后顶层未配置或设为开启时统一使用 priority，顶层显式关闭时恢复 Fast -> priority、Flex -> flex、None -> unset 的官方映射，profile 内同名设置不生效，这样用户默认获得更稳定的优先体验，也能按需回到官方行为。

7. Windows app、app server 和 TUI 默认日志降噪，运行时负担默认更轻；原来未显式设置日志时可能产生高噪声记录，修改后 Windows app/app server 和 TUI 默认只保留更安静的日志级别，显式设置后仍可打开详细日志，同时 analytics、feedback、log_db 默认关闭但可配置开启，这样用户日常使用更轻、更安静，需要排查或反馈时还能手动打开。

8. 默认开启不影响使用的批量优化，并保留即时反馈和历史安全；原来 rollout 批量 flush 与 app-server 高频通知合并默认关闭，修改后这些优化可以默认开启，但前提是输出节奏、token usage、diff/plan 更新、命令完成状态和崩溃恢复与未开启优化时保持用户可感知等价，同时必须保留显式关闭开关；这样用户默认获得更低 I/O 和更少客户端负担，但仍能看到及时刷新和可靠历史。

9. Provider refresh 的刷新范围扩大到所有正在使用的 Codex 入口，并覆盖会影响路由和速度的关键 provider 字段；原来 provider runtime 刷新只要求覆盖 `base_url` 与 `experimental_bearer_token` 两个字段，刷新结果可能只影响部分 live instance，修改后 `base_url`、`experimental_bearer_token`、`force_service_tier_priority` 与 fast mode 相关有效配置都必须对所有 app server、已经打开的 Codex 窗口/会话、`codex exec`、已打开和后续新开的 subagent、以及 agent_jobs 批量子任务尽可能更快生效，Windows tray 从 source provider 复制字段到当前 target provider 后也要触发同一刷新口径；这样用户换 URL、token、优先服务层或 fast 开关后，不同入口不会继续拿旧地址、旧 token 或旧速度/服务层策略发请求。

10. Provider refresh 不是只在 retry 时才生效，而是配置变化后面向所有 live runtime 的通用刷新能力；原来 refresh 容易被理解成“请求失败后的补救动作”，修改后只要 provider 有效配置发生变化，就应尽快刷新已加载线程、app-server runtime、console/exec runtime 和正在等待下一次请求的会话，即使当前没有 retry、没有报错、没有正在流式输出，也应让下一次请求使用新 provider 状态；这样用户主动切换 provider 参数后，不必靠失败重试或新开对话才能看到新配置。

11. 所有 Codex retry 入口都要接入 hard route recovery，但只有连续第 3 次可重试路由失败后才启动：前 2 次只做普通重试，不重置 WebSocket session、不旋转 `prompt_cache_key`、不主动丢 `previous_response_id`；第 3 次起，HTTP 503/502/504、SSE/WebSocket 未完成断流、WebSocket handshake 失败和 `codex exec`、subagent、agent_jobs 等入口都应使用同一恢复口径，将 `prompt_cache_key` 从默认 thread id 派生为带 recovery generation 的新值，同时重置 WebSocket session，使下一次普通全量重放不携带旧 `previous_response_id`；必要时继续走 fallback transport，但不修改真实 thread id、session id 或用户提示词，且 `function_call_output` 等必须续接旧响应链的请求不能强行丢 `previous_response_id`。这样用户遇到“503 retry N (unbounded) / Reconnecting... N (unbounded) / stream closed before response.completed”时，下一轮 retry 有机会换掉有问题的中转账号粘连，而不是只能新开对话或派生线程。

12. 无 live instance 的 provider 字段复制仍视为成功，并给出明确反馈；原来没有可刷新实例时可能让用户误以为字段写入失败，修改后只要 provider 字段写入成功，即使没有任何正在运行的实例，也反馈“未刷新任何实例”，这样用户能区分“配置已保存”和“当前没有可通知的运行入口”。

13. app-server stderr 默认保持安静，只有显式配置才打开后台诊断输出；原来 `warn` 日志或 WebSocket 启动 banner 可能默认写入 stderr，修改后默认不再显示 `codex app-server (WebSockets)`、`listening on`、`readyz`、`healthz` 等后台诊断文字，只有用户在 `config.toml` 配置 `[logging] app_server_stderr = true` 后才恢复这些诊断信息，这样日常使用更安静，排查问题时仍能手动打开。

14. `node_repl` MCP 自动继承当前 local3 CLI 路径；原来用户实际运行 local3 时，`node_repl` 子进程仍可能使用 AppData 自动安装目录里的旧版 `codex.exe`，修改后启动 `[mcp_servers.node_repl]` 时会把 `CODEX_CLI_PATH` 指向当前 `Config.codex_self_exe`，这样 refresh、诊断和 app-server 行为跟当前 local3 版本保持一致。

15. app-server 退出时只补已有 runtime 引用清理，不做激进进程管理；原来 shutdown 路径可能漏释放外部 auth、apps runtime 和 skills watcher 引用，修改后主 app-server 退出时补调已有 `clear_runtime_references()`，但不新增 idle timeout，不全局扫描或 kill `node_repl.exe`，也不因为当前 UI 订阅断开就杀仍加载的线程，这样能减少残留引用，同时避免误伤正在使用的会话。

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
- retry 粘连故障的补救核心是 sticky-break：所有 Codex retry 入口都要覆盖，但只在第 3 次连续可重试路由失败后旋转 `prompt_cache_key`，并让可全量重放的 recovery 请求清掉旧 `previous_response_id`；禁止把真实 thread id/session id 改掉，也不要通过改用户 prompt 来“换内容”，`function_call_output` 等必须续链场景不得强行丢续接 ID。
- 用户报告的 `503 retry N (unbounded)` 必须单独建无界场景覆盖；不写 `request_max_retries` 才是 release exe 的正式无界 retry 口径，不能只用有界 retry 代替。
- 分开刷新要保留旧的全刷，同时新增 `console` 与 `appServer` scope；动态验收至少要证明 `appServer` 能刷新 Windows App app-server thread，`console` 不误刷 app-server thread。
- GitHub CLI 查询和触发必须显式带 `--repo dqIndieGames/codex`；否则可能落到 `openai/codex`，导致 run/release 证据查错仓库。
- 禁止本地编译时，编译证据只能来自 GitHub Actions；本地只下载 release zip，比对 GitHub asset digest，再用下载的 `codex.exe` 做真实 smoke 与 refresh 矩阵。

## 2026-06-14 历史线程 provider 重绑经验

- `codex://threads/{id}` 不能只当作“接回旧 loaded thread”；当用户已切换当前顶层 provider 时，恢复旧线程也要验证实际请求 provider 是否同步切换，否则旧线程会因历史 `session_meta.model_provider` 或 loaded `config_snapshot.model_provider_id` 继续走旧 provider。
