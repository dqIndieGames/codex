# local2 定制功能清单（2026-04-27）

## 用途

- 本文只记录 local2 需要保留、已经存在或后续必须继续保住的功能。
- 本文用于以后并入 upstream 时逐项核对，不写代码方案，不写实现步骤，不写分析报告。
- `upstream` 是官方上游仓库；用户视角是以后合并官方新版本时，按这份清单检查自己的功能有没有丢。

## 功能清单

| ID | 功能 | 必须保留的口径 |
|---|---|---|
| `L2-F1` | local2 版本显示 | CLI、TUI、状态卡片、标题区、历史单元、升级提示等用户可见位置，都要显示 local2 本地构建身份，不能被 upstream 合并改回官方裸版本。 |
| `L2-F2` | local2 版本显示测试保护 | 和 local2 版本显示直接相关的断言、快照或等效测试保护要保留；合并 upstream 后，如果版本显示被冲掉，应能被测试或静态复核发现。 |
| `L2-F3` | “你好”首轮显示清单 | brand-new thread 或 Clear 后的新线程，首个普通用户输入恰好为纯文本 `你好` 时，首个 assistant 主消息第一段显示固定 local2 功能清单；其他输入如 `hello`、`hi`、`你好啊`、带图片、多输入项、富文本 `你好` 不触发。 |
| `L2-F4` | “你好”清单只显示一次 | 同一线程后续轮次、resume、continue、fork、历史线程重开、MCP `codex-reply` 都不能重复插入这段清单；subagent、reviewer、guardian 等子会话也不能触发。 |
| `L2-F5` | “你好”清单走普通 assistant 主消息 | 清单必须作为普通 assistant 主消息的首段文本跨 CLI、app-server、VS Code、MCP 一致可见；不能退回 TUI 专用 banner、status、计划事件、history cell 或客户端旁路。 |
| `L2-F6` | `/responses` 远端 HTTP 错误统一自动重试 | `/responses` 主链远端 HTTP 错误统一进入普通自动 retry，包含 `401`，不再只依赖 `402`、`429`、`5xx` 白名单。 |
| `L2-F7` | 非 `/responses` 端点保留旧重试白名单 | 非 `/responses` 端点继续只按旧口径重试：`402 usage-limit`、`429`、`5xx`、传输层错误；非 usage-limit 的 `402` 不能被误判为可重试。 |
| `L2-F8` | `401` 走普通 retry | `/responses` 主链执行中出现 `401` 时，request-layer、stream、websocket reconnect、fallback to HTTP 都直接走普通 retry；不能退回 unauthorized recovery 优先分支，也不能直接终止。 |
| `L2-F9` | retry 次数保持大次数或等效无界目标 | 普通主链 retry 的 bounded/unbounded 语义要保留；分类扩展不能被误改成很少次数的短 retry，也不能把中间 retry 误当终态失败。 |
| `L2-F10` | 单次 retry 等待上限为 `10s` | 无论 HTTP 错误分类扩展到多少，单次指数退避或 `Retry-After` 等待都不能超过 `10s`。 |
| `L2-F11` | retry 中间态不写入历史 | `/responses` 主链 retry 或 reconnect 的中间态只更新状态区和状态详情，不往历史区写入脏错误记录；只有最终失败才进入终态错误路径。 |
| `L2-F12` | retry 可见提示保留 | 首个 websocket retry、`Reconnecting... N`、retry 详情字段、`additional_details`、`request_retry_notifier`、`will_retry = true` 等用户可见或可诊断提示不能丢。 |
| `L2-F13` | retry 日志降噪 | `/responses` retry 中间态不再刷 `codex.api_request`、`codex.websocket_connect`、`codex.websocket_request`、失败型 `codex.websocket_event` 的中间态 OTEL log-trace；sampling reconnect warn 也不能刷屏。 |
| `L2-F14` | retry metrics 保留 | retry 中间态 metrics 继续保留；不能因为降噪把可观测统计一起删掉。 |
| `L2-F15` | 历史默认跨 provider 可发现 | 历史列表、最近会话、resume picker 默认不按 provider 过滤；继续旧线程时执行仍使用当前 `config.toml` 顶层 provider。 |
| `L2-F16` | provider provenance 不作为保真要求 | 历史与 resume 默认跨 provider 可发现；不要求 `thread/list` 历史 provider provenance 保真。 |
| `L2-F17` | 全局 `service_tier=priority` 默认开启 | 顶层 `force_service_tier_priority` 省略或显式 `true` 时，所有 `/responses` 请求在底层构造时强制带 `service_tier=priority`。 |
| `L2-F18` | `force_service_tier_priority = false` 恢复官方映射 | 顶层显式 `false` 时恢复官方原始映射：`Fast -> priority`、`Flex -> flex`、`None -> unset`，不能误压 `flex`。 |
| `L2-F19` | `force_service_tier_priority` 只允许顶层配置 | 该字段只允许写在顶层 `config.toml`；任何 `[profiles.*].force_service_tier_priority` 都不得生效。 |
| `L2-F20` | Windows app/app-server 默认日志降噪 | 未显式设置 `RUST_LOG` 时，Windows app/app-server 默认日志过滤为 warn 级别，不能高噪声写入 sqlite 日志；显式 `RUST_LOG` 时仍可覆盖。 |
| `L2-F21` | TUI 默认日志降噪 | 未显式设置 `RUST_LOG` 时，`codex-tui.log` 与 TUI sqlite log layer 默认降噪；显式 `RUST_LOG` 时仍可恢复详细日志。 |

## 使用方式

- 合并 upstream 前，先读本文确认 local2 要保留哪些功能。
- 合并 upstream 后，按 `L2-F1` 到 `L2-F21` 逐条核对。
- 如果后续新增 local2 功能，继续追加新的 `L2-F*` 条目。
- 本文只是一份功能清单，不负责写代码方案、执行步骤或测试报告。

## 最终结论

local2 当前需要保留的功能已按条目列出；后续并入 upstream 时按本清单逐项核对即可。
