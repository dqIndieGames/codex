# 当前Thread Provider两字段刷新_TASK_2026-04-01

## Context
- 当前 thread 真正运行时使用的是 session 初始化后固定下来的 provider 运行态，而不是仅看配置层堆栈。
- 现有用户配置刷新链路只会更新 user config layer 和部分缓存，不能让当前 thread 正在使用的 provider 两字段自动生效。
- 本次需求不是完整 provider 热切换，也不是重开 thread；只要求刷新当前 thread 运行态中的 `base_url` 和 `experimental_bearer_token`。
- 本次需求允许在 thread 非空闲时触发刷新，但不允许打断当前 turn；新值应当从下一轮开始生效。
- 本次需求新增一个 Windows 常驻托盘入口，用于一键触发当前系统内所有 app-server 实例执行批量刷新。
- 当前系统内的 app-server 实例发现方式唯一固定为实例自注册目录；注册根目录固定为 `$CODEX_HOME/app_servers/`，当前本机对应路径是 `C:\Users\Administrator\.codex\app_servers\`。
- 每个 app-server 实例需要维护自己的注册文件生命周期：启动时创建，运行中更新心跳，正常退出时删除；异常退出导致的脏注册项由托盘侧在批量刷新前做存活检查并清理。

## Goal
- 在 app-server 中提供一个面向当前 thread 的刷新入口。
- 在 app-server 中提供一个面向当前实例所有已加载 threads 的批量刷新入口，供 Windows 托盘统一调用。
- 入口触发后，按每个 thread 当前运行时使用的 provider 重新读取最新用户配置，并提取该 provider 对应的 `base_url` 与 `experimental_bearer_token`。
- 若当前 thread 空闲，则立即刷新当前 thread 的运行态，为下一轮请求生效。
- 若当前 thread 非空闲，则先记录待应用刷新；待当前 turn 结束后自动应用，并从下一轮开始生效。
- 整个实现范围只覆盖这两个字段；其他 provider 字段即使配置中发生变化，也不在本次刷新范围内。
- Windows 托盘最小范围仅包括：一个常驻托盘图标、一个“刷新全部 app-server”菜单项、一个只显示本次实例级成功数/失败数的结果面板。
- Windows 托盘范围明确排除：设置页、日志页、实例列表、手动清理入口、自动刷新、后台轮询历史记录以及其他额外控制能力。
- Windows 托盘工具固定使用系统 Python 实现，不使用虚拟环境。
- Windows 托盘工具默认以可直接运行的脚本形态交付，不额外要求桌面安装器、服务化封装或复杂发布流程。
- 每个 app-server 实例启动后都应在 `$CODEX_HOME/app_servers/` 下写入自己的实例注册文件；托盘通过扫描该目录发现实例，而不是猜测系统进程。
- 每个实例正常关闭时必须删除自己的注册文件；托盘在执行批量刷新前需要结合 `pid`、心跳时间和轻量连通性检查清理脏注册项。
- 批量 RPC 以 thread 为执行单位，以 instance 为汇总单位：实例内所有已加载 thread 都成功立即应用或成功挂起待下一轮应用时，该实例记为成功；任一 thread 刷新或挂起失败则该实例记为失败。
- 本次修改完成后，需要在 `docs/local1-custom-feature-checklist-2026-03-28.md` 中同时完成两处记录：
- 在“定制功能主清单”中新增一条新的 `F*` 功能项，使用当时下一个可用 ID。
- 该功能项至少包含 `功能项`、`明确定义`、`当前代码迹象`、`后续验收口径` 四列内容。
- 在“同步官方后的必查清单”中新增一条对应检查项，用于后续同步官方更新后的回归核对。

## Checklist
- 保留 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 两个 RPC，不扩展 Python SDK 公共包装层。
- provider runtime 刷新改为只读读取最新 user config，再按当前 session 绑定的 provider id 提取 `base_url` 与 `experimental_bearer_token`，不复用旧 runtime 快照。
- 两字段提取固定支持“字段缺失按 `None` 覆盖”；若当前 provider id 在最新配置中消失，则刷新失败且保持运行态与 session 配置快照不变。
- session 级保留 `pending_provider_runtime_refresh`，空闲 thread 立即 `Applied`，非空闲 thread 仅 `Queued`。
- 待应用刷新固定在新 regular turn / review turn 启动前生效；除用户输入显式起新 turn 外，也覆盖 `ensure_task_for_pending_inputs_with_sub_id(...)` 这类空闲唤起 regular turn 的路径。
- 真正运行态更新只覆盖 `session_configuration.provider`、`ModelClient` provider runtime、`ModelClient` cached websocket session；`wire_api`、`supports_websockets`、重试预算、`disable_websockets` 等其余字段保持原值。
- provider refresh 失败路径保持 no-op：不提前整体 reload user config layer，不清理 skills/plugins cache，不提前修改 `original_config_do_not_use`，不留下脏 pending 状态。
- app-server 启动后在 `$CODEX_HOME/app_servers/<instance_id>.json` 注册实例；注册文件字段固定包含 `instance_id`、`pid`、`control_endpoint`、`started_at`、`heartbeat_at`。
- Windows 最小控制面固定为 named pipe + NDJSON 一问一答，只暴露 `ping` 与 `refresh_all_loaded_threads`，不引入新的通用 transport。
- 正常退出删除注册文件；5 秒心跳持续更新；异常退出残留仍由托盘按既定规则清脏。
- 托盘实例发现唯一来源固定为 `$CODEX_HOME/app_servers/*.json`；无效 JSON、缺字段、死 PID 直接删；仅在 `heartbeat_at` 超过 15 秒时再做一次 `ping` 决定保留或删除。
- 托盘脚本固定使用系统 Python 直接运行，不使用虚拟环境；UI 仍为 `pystray` + Pillow 运行时图标 + `MessageBoxW` 结果面板。
- 托盘里的 Win32 调用补齐 `ctypes` `argtypes/restype`，避免 64 位 Windows 下句柄和布尔返回值被错误截断。
- 单线程 RPC 失败时保留错误分类：provider 缺失、user config 解析失败这类请求/配置错误返回 invalid-request 级错误；真正异常仍走 internal error。
- 补源码侧测试但不运行：覆盖 idle/apply、active/queued、删除字段清空、provider 缺失 no-op、invalid config no-op、空闲唤起 regular turn 前应用 pending refresh、app-server 错误码分类、tray Win32 prototype 与清脏逻辑。
- 更新 app-server README，补充两个 RPC、实例注册目录、named pipe 控制面与托盘脚本运行方式。
- 更新 `docs/local1-custom-feature-checklist-2026-03-28.md` 两处：主清单追加 `F10`，必查清单追加对应回归项。

## Acceptance
- 调用刷新入口时，无需重开 thread。
- 调用实例级批量刷新入口时，无需重开 app-server 或其中任何 thread。
- 在 thread 非空闲时触发刷新，当前 turn 行为保持不变，不会被中断、重连到新地址，或中途切换新 token。
- 当前 turn 结束后，下一轮请求开始前，当前 thread 已切换到新的 `base_url` 和 `experimental_bearer_token`。
- 若 thread 触发刷新时本来就是空闲状态，则刷新后紧接着的下一轮请求直接使用新值。
- 本次刷新只影响 `base_url` 和 `experimental_bearer_token`；其他 provider 配置仍保持 session 初始化时的运行态值。
- provider 缺失或 user config 语法错误时，刷新请求返回失败，且运行态、`original_config_do_not_use`、`pending_provider_runtime_refresh` 都保持不变。
- 非空闲触发后的 pending refresh 会在用户输入显式起新 turn、空闲自动唤起 regular turn、review turn 三类入口前统一应用。
- 每个 app-server 实例在启动后都能在 `$CODEX_HOME/app_servers/` 下看到对应注册文件，且注册信息可被托盘读取。
- 每个 app-server 实例正常退出后，其注册文件会被删除，不依赖托盘兜底清理才能保持目录干净。
- 托盘点击“刷新全部 app-server”后，能够发现当前所有存活实例，并显示本次刷新成功几个实例、失败几个实例。
- 若目录中存在异常退出留下的脏注册项，托盘在本次刷新前能够识别并清理这些无效实例记录，不把它们误算成活跃实例。
- Windows 托盘工具可在当前 Windows 环境中通过系统 Python 直接启动，不依赖虚拟环境。
- Windows 托盘工具的默认交付形态为直接运行脚本，而不是安装包、服务或其他额外包装形式。
- Windows 托盘的 Win32 API 调用在 64 位环境下仍按正确句柄/布尔 ABI 工作，不因 `ctypes` 默认签名导致 PID 检查、pipe 连接或结果弹窗误判。
- 批量 RPC 的实例级结果口径固定为：实例内所有已加载 thread 都成功立即应用或成功挂起待下一轮应用时，该实例记为成功；任一 thread 刷新或挂起失败则该实例记为失败。
- 单线程 RPC 在 provider 缺失、user config 解析失败等可归因错误上返回 invalid-request 级 JSON-RPC 错误，而不是统一 internal error。
- 本次刷新只允许清理那些会导致下一轮继续沿用旧 `base_url` 或旧 token 的派生缓存；不得顺带重算、切换或热更新 `wire_api`、`supports_websockets`、模型目录来源、其他鉴权模式或其他 provider 能力字段。
- 实现完成后，`docs/local1-custom-feature-checklist-2026-03-28.md` 的“定制功能主清单”中已存在本功能对应的 `F*` 条目，且四列信息填写完整。
- 实现完成后，`docs/local1-custom-feature-checklist-2026-03-28.md` 的“同步官方后的必查清单”中已存在本功能对应的检查项。

## Notes
- 本次任务是刻意收窄的“脏但可控”的运行态刷新方案，不追求完整 provider 一致性。
- 本次任务默认不处理完整 provider 能力切换，例如 `wire_api`、`supports_websockets`、模型目录来源或其他鉴权模式变更。
- 若用户配置中的其他 provider 字段发生变化，本次逻辑默认忽略，不因这些差异阻止两字段刷新。
- provider refresh 不再复用“整体 reload user config layer”副作用链；成功时只更新本任务要求的两字段运行态，失败时保持 no-op。
- 实例发现的唯一枚举来源固定为 `$CODEX_HOME/app_servers/` 下的注册文件；不得引入进程扫描、端口扫描或窗口标题匹配作为备用发现机制。
- 注册文件推荐采用“每实例一个文件”的目录式结构，例如 `$CODEX_HOME/app_servers/<instance_id>.json`，避免多实例并发写入单总表。
- “关闭时取消注册”仅针对正常退出路径强制要求；对于崩溃、强杀等非正常退出，默认由托盘基于 `pid`、心跳和连通性检查清理脏注册项。
- 托盘清脏规则维持原 TASK 口径：`pid` 校验后，仅在 `heartbeat_at` 超过 15 秒未更新时再做 `ping`；不额外扩展成“所有存活 PID 都强制 ping”。
- Windows 托盘结果面板只负责显示本次实例级成功数/失败数，不扩展为通用控制台。
- Windows 托盘工具使用系统 Python；若需要第三方依赖，也应按系统 Python 直接安装和运行，不把虚拟环境写成默认前提。
- `ctypes` Win32 API 调用必须显式绑定 `argtypes/restype`；这是脚本正确性要求，不是可选优化。
- checklist 文档中的功能项 ID 不在本 TASK 中写死；实现完成时，按目标文档当时的下一个可用 `F*` ID 追加。
- 当前文档产出阶段与文档复核阶段均禁止执行任何编译命令；复核以代码链路审阅和任务边界检查为准。

## 执行回写
- 实际实现结果：core 已新增 session 级 `pending_provider_runtime_refresh`、thread 级单线程刷新入口、实例级批量刷新汇总，以及 `ModelClient` 两字段 runtime mutator；空闲线程返回 `Applied`，活跃线程返回 `Queued`。
- 实际实现结果：pending provider refresh 现已在用户输入起新 regular turn、空闲自动唤起 regular turn、review turn 三类入口前统一应用。
- 实际实现结果：provider refresh 改为只读读取最新 user config；provider 缺失或 user config 语法错误时，失败返回且不提前修改 runtime、`original_config_do_not_use`、`pending_provider_runtime_refresh` 或 skills/plugins cache。
- 实际实现结果：app-server 已新增 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 两个 RPC；单线程 RPC 现会把 provider 缺失、config 解析失败映射成 invalid-request 级错误。
- 实际实现结果：Windows 控制面已落到 `$CODEX_HOME/app_servers/<instance_id>.json` + `\\\\.\\pipe\\codex-app-server-<instance_id>`；启动时写注册、5 秒心跳、正常退出删注册。
- 实际实现结果：托盘脚本已按系统 Python 直接运行形态落到 `scripts/windows_app_server_refresh_tray.py`；枚举来源唯一是 `$CODEX_HOME/app_servers/*.json`；清脏规则仍按既定 `pid + heartbeat + stale ping` 执行；Win32 API 已补 `ctypes` 原型声明。
- 实际实现结果：已补“只写不跑”的源码级测试代码，新增覆盖 provider 缺失 no-op、invalid config no-op、空闲自动唤起 regular turn 前应用 pending refresh、单线程 RPC 错误码分类、tray Win32 prototype。
- 与 TASK 原口径一致/偏离点：一致，未扩展 Python SDK 公共 API；未把 named pipe 做成第三种通用 transport；实例发现仍只依赖注册目录；刷新口径仍只覆盖 `base_url` 与 `experimental_bearer_token`。
- 与 TASK 原口径一致/偏离点：一致，托盘脏注册项的 `ping` 触发条件仍保持“`heartbeat_at` 超过 15 秒后再探活”，没有改成 fresh heartbeat 也强制 ping。
- 与 TASK 原口径一致/偏离点：一致，文档已同步更新 app-server README 与本地 checklist 双处。
- 风险说明：本轮严格遵守限制，未执行编译、构建、测试、格式化；当前结论仅基于源码链路审阅、源码级测试代码补充与 reviewer 复核，若存在编译层或运行时残余问题，需要在后续真实验证时再收敛。
- checklist 文档实际写入位置与使用的 `F*` ID：文档位置 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md`；实际新增 ID 为 `F10`；已写入 `定制功能主清单` 和 `同步官方后的必查清单` 两处。

## Subagent 严格复核
### 问题清单
- 高。文件：[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L32)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L93)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L113)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L139)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L152)、[test_windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/tests/test_windows_app_server_refresh_tray.py#L40)。问题：托盘脚本直接调用 Win32 API，但没有为 `OpenProcess`、`GetExitCodeProcess`、`CloseHandle`、`WaitNamedPipeW`、`CreateFileW`、`ReadFile`、`WriteFile`、`MessageBoxW` 绑定 `argtypes/restype`。原因：`ctypes.WinDLL` 默认按 `c_int` 处理返回值和参数，64 位 Windows 下 `HANDLE` 会被截断，布尔/指针参数也缺少 ABI 约束。影响：PID 存活检查、named pipe `ping`、批量刷新都可能在真实 Windows 上随机失败或误判，直接打穿“托盘发现实例并批量刷新”的主链路；当前测试只覆盖纯 Python 逻辑，没有任何用例会暴露这个 64 位 ABI 问题。
- 高。文件：[codex.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs#L2562)、[codex.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs#L2600)、[codex.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs#L2601)、[codex.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs#L4350)、[codex_tests.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex_tests.rs#L4046)、[client_tests.rs](E:/vscodeProject/codex_github/codex/codex-rs/core/src/client_tests.rs#L148)。问题：`refresh_provider_runtime()` 复用了整套 `reload_user_config_layer_result()`，导致“仅刷新 provider 两字段”的 RPC 在真正应用前就会修改 session 内部配置快照并清理 skills/plugins cache；如果 provider 已从新配置中消失，刷新虽然返回失败，但这些副作用已经发生。原因：刷新入口先整体重载 user config layer，再去解析 provider 两字段，而不是做一次只读解析并在校验通过后再原子更新 runtime。影响：实现范围被放大，已经不再是 TASK 要求的“只影响 `base_url` / `experimental_bearer_token`、只清理会继续沿用旧值的派生缓存”；失败路径也不是严格 no-op，后续 turn 可能收到与 provider 刷新无关的 cache 失效或配置快照变化。现有测试只断言 provider 字段和 applied/queued 状态，没有约束“无额外副作用”。
- 中。文件：[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L196)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L216)、[windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/windows_app_server_refresh_tray.py#L238)、[test_windows_app_server_refresh_tray.py](E:/vscodeProject/codex_github/codex/scripts/tests/test_windows_app_server_refresh_tray.py#L70)。问题：托盘清脏逻辑只在心跳已过期时才做连通性 `ping`，心跳仍“新鲜”但 pipe 已不可达的注册项会被当成活实例保留。原因：`prune_stale_registration()` 把轻量连通性检查硬性挂在 `if stale` 分支下，没有在刷新前统一结合 `pid + heartbeat + ping` 做活性判定。影响：最近刚异常退出、PID 被复用、或 control pipe 已经失效但心跳窗口还没过期的脏注册项，会在本次刷新里被误算为活跃实例，结果面板把它们算进失败实例数，而不是按 TASK 要求在刷新前清理掉。现有测试只覆盖“心跳过期才 ping”的分支，没有覆盖“心跳未过期但 endpoint 已死”的边界。
- 中。文件：[codex_message_processor.rs](E:/vscodeProject/codex_github/codex/codex-rs/app-server/src/codex_message_processor.rs#L3429)、[codex_message_processor.rs](E:/vscodeProject/codex_github/codex/codex-rs/app-server/src/codex_message_processor.rs#L3455)、[thread_provider_runtime_refresh.rs](E:/vscodeProject/codex_github/codex/codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs#L116)。问题：单线程 RPC `thread/providerRuntime/refresh` 把所有刷新失败都包装成 `INTERNAL_ERROR_CODE`，包括 provider 已被删除、用户配置解析失败这类本应属于请求/配置错误的场景。原因：handler 直接吞掉 core 返回的错误分类，用统一 internal error 对外返回。影响：协议口径和 bulk RPC 不一致，客户端无法区分“配置有问题，需要用户修正”与“服务端内部故障”，不利于调用方正确提示和恢复；当前 app-server 集成测试只有 applied/queued/zero-thread 成功路径，没有覆盖 provider 缺失或坏配置下的错误码契约。

### 修改建议
- 对应问题 1：为脚本里实际调用的 Win32 API 全量声明 `argtypes/restype`，尤其把所有句柄相关函数改成 `wintypes.HANDLE` 或 `ctypes.c_void_p`，把布尔返回值改成 `wintypes.BOOL`；同时补一个不依赖真实 pipe 的脚本测试，至少断言这些 prototype 已设置，并验证 64 位句柄值不会在 Python 层被截断。
- 对应问题 2：不要在 provider refresh 里直接复用 `reload_user_config_layer_result()`；拆出一个“只读取最新 user config、只解析当前 provider 的 `base_url`/`experimental_bearer_token`”的只读 helper，校验通过后再一次性更新 `session_configuration.provider`、`ModelClient` runtime 和 websocket cache。失败路径应保持 `original_config_do_not_use`、`skills/plugins cache`、`pending_provider_runtime_refresh` 全部不变，并补测试约束这些 no-op 语义。
- 对应问题 3：把轻量连通性检查前移到“判定实例是否活着”的阶段，而不是只在心跳过期时才做；至少对所有 `pid` 存活的注册项在计入 live 列表前做一次 `ping`，失败就删文件并不计入结果。补一个“heartbeat 新鲜但 pipe 不可达 / PID 复用”的测试，确保这类脏注册项在本次刷新前被清理。
- 对应问题 4：保留 core 错误分类，对 `CodexErr::InvalidRequest` 之类的可归因错误返回请求级/参数级 JSON-RPC 错误，只把真正的不可预期异常映射成 `INTERNAL_ERROR_CODE`；同时补 app-server 集成测试覆盖 provider 已消失、user config 语法错误两条错误路径，校验错误码和“运行态不变”的契约。

## 主Agent 二次审核与复写
- 问题 1：采纳并已修改。已在 `scripts/windows_app_server_refresh_tray.py` 为 `OpenProcess`、`GetExitCodeProcess`、`CloseHandle`、`WaitNamedPipeW`、`CreateFileW`、`ReadFile`、`WriteFile`、`MessageBoxW` 补齐 `argtypes/restype`，并把写 pipe 请求改为显式 buffer；`scripts/tests/test_windows_app_server_refresh_tray.py` 追加了 prototype 静态断言。
- 问题 2：采纳并已修改。`core/src/codex.rs` 已拆出只读读取最新 user config 的 helper，`refresh_provider_runtime()` 不再先整体 reload user config layer；provider 缺失和 invalid config 两类失败路径现在都保持 runtime、`original_config_do_not_use`、`pending_provider_runtime_refresh` 不变；`core/src/codex_tests.rs` 已补 no-op 断言。
- 问题 3：不采纳及理由。当前 TASK 已明确把托盘清脏规则固定为“`heartbeat_at` 超过 15 秒未更新时，先做一次 pipe ping；ping 失败则删，成功则保留”；因此不应擅自扩大为“fresh heartbeat 也统一 ping”。该建议与已冻结任务口径冲突，所以保持现实现状，并在顶部 `Notes` 中补充写死此口径。
- 问题 4：采纳并已修改。`app-server/src/codex_message_processor.rs` 已保留 core 错误分类，对 `CodexErr::InvalidRequest` 返回 invalid-request 级错误；`app-server/tests/suite/v2/thread_provider_runtime_refresh.rs` 已补 provider 缺失、user config 语法错误两条错误码用例。
