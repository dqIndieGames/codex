# local3 定制功能清单_2026-05-10
本清单用于在 local3 合并官方上游版本后，按用户可感知的功能结果核对本地定制能力是否仍然保留。

1. local3 版本身份保留为 `<Codex 版本>-local3`，所有用户能看到版本的位置都要显示这个本地构建后缀；原来可能被上游版本覆盖成官方裸版本，修改后 CLI、TUI、状态卡片、标题区、历史单元和升级提示都继续显示 local3 身份，这样用户能确认自己正在使用本地定制版。

2. 首次输入纯文本 `你好` 时显示 local3 功能清单，并且每个新线程只显示 1 次；原来清单可能被做成某个客户端专用提示或重复插入，修改后 brand-new thread 或 Clear 后的新线程中，首个普通用户输入恰好为 `你好` 才会在首个 assistant 主消息第一段显示全量 local3 清单，resume、continue、fork、历史线程重开、子会话和其他输入都不重复触发，这样用户首次检查定制功能时能稳定看到完整清单且不会被反复打扰。

3. 远端请求失败后的自动重试覆盖所有错误，保持更耐用、更少打断的体验；原来只有部分远端错误会按白名单重试，修改后所有远端请求错误都进入普通自动重试，单次等待上限保持 `10s`，中间失败不写入历史，只保留可见重连提示和可诊断信息，这样用户在临时鉴权、网络、服务抖动或其他远端异常时更少看到终态失败。

4. 重试期间的可见提示、日志噪声和统计口径保持平衡；原来中间态可能刷屏或让诊断信息丢失，修改后用户仍能看到首次重连、重试次数、重试详情等提示，日志不再被中间失败刷屏，同时 retry metrics 继续保留，这样用户界面更安静，排查问题时仍有统计依据。

5. 历史会话默认跨 provider 可发现，并且继续旧线程时使用当前顶层 provider；原来历史入口可能按 provider 收窄，修改后历史列表、最近会话和 resume picker 默认都能看到旧会话，不要求历史 provider 来源绝对保真，这样用户切换 provider 后仍能找回并继续之前的工作。

6. 全局优先服务层默认开启，并允许显式恢复官方映射；原来不同配置层级可能让请求服务层表现不一致，修改后顶层未配置或设为开启时统一使用 priority，顶层显式关闭时恢复 Fast -> priority、Flex -> flex、None -> unset 的官方映射，profile 内同名设置不生效，这样用户默认获得更稳定的优先体验，也能按需回到官方行为。

7. Windows app、app server 和 TUI 默认日志降噪，运行时负担默认更轻；原来未显式设置日志时可能产生高噪声记录，修改后 Windows app/app server 和 TUI 默认只保留更安静的日志级别，显式设置后仍可打开详细日志，同时 analytics、feedback、log_db 默认关闭但可配置开启，这样用户日常使用更轻、更安静，需要排查或反馈时还能手动打开。

8. 默认开启不影响使用的批量优化，并保留即时反馈和历史安全；原来 rollout 批量 flush 与 app-server 高频通知合并默认关闭，修改后这些优化可以默认开启，但前提是输出节奏、token usage、diff/plan 更新、命令完成状态和崩溃恢复与未开启优化时保持用户可感知等价，同时必须保留显式关闭开关；这样用户默认获得更低 I/O 和更少客户端负担，但仍能看到及时刷新和可靠历史。

9. Provider refresh 的 URL 和 token 刷新范围扩大到所有正在使用的 Codex 入口；原来 provider runtime 刷新只要求覆盖 `base_url` 与 `experimental_bearer_token` 两个字段，刷新结果可能只影响部分 live instance，修改后这两个字段刷新必须对所有 app server、已经打开的 Codex 窗口/会话、以及 `codex exec` 都生效，Windows tray 从 source provider 复制字段到当前 target provider 后也要触发同一刷新口径；这样用户换 URL 或 token 后，不同入口不会继续拿旧地址或旧 token 发请求。

10. 无 live instance 的 provider 字段复制仍视为成功，并给出明确反馈；原来没有可刷新实例时可能让用户误以为字段写入失败，修改后只要 `base_url` 与 `experimental_bearer_token` 写入成功，即使没有任何正在运行的实例，也反馈“未刷新任何实例”，这样用户能区分“配置已保存”和“当前没有可通知的运行入口”。

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
