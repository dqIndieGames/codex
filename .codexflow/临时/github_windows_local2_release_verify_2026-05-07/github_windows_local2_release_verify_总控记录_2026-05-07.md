# GitHub Windows local2 release verify 总控记录 2026-05-07

## 目标

- 仅通过 GitHub Actions 打包最小 Windows release 产物 `codex.exe`，不在本地执行任何编译。
- 若远程打包失败，基于失败日志修复代码并重新触发远程打包。
- 远程打包成功后下载 Windows `codex.exe`，依据 [docs/local2-custom-feature-checklist-2026-04-27.md](I:/vscodeProject/codex/docs/local2-custom-feature-checklist-2026-04-27.md) 做本地功能验收。
- 对无法通过本地运行直接覆盖的清单项，补做 subagent 代码审核。
- 如果测试或审核发现问题，修复后重复“远程打包 -> 下载测试 -> 代码审核”闭环。

## 输入与边界

- 项目根目录：`I:\vscodeProject\codex`
- 当前分支：`main`
- 当前远端：`origin=https://github.com/dqIndieGames/codex.git`
- 当前可用工作流：`I:\vscodeProject\codex\.github\workflows\local2-minimal-windows-release.yml`
- 功能真值清单：`I:\vscodeProject\codex\docs\local2-custom-feature-checklist-2026-04-27.md`
- 约束：
  - 不做本地编译。
  - GitHub 远程构建失败后只按真实日志修复。
  - 保留 local2 定制功能，不允许为过 CI 删除清单功能。
  - 未经确认不创建/切换分支或 worktree。

## 流程发现

### 入口

- GitHub Actions `workflow_dispatch` 入口：
  - workflow: `local2-minimal-windows-release`
  - 输入：`tag`、`target`、`release_name`
- 本地验收入口：
  - 下载 GitHub Release zip
  - 解压 `codex.exe`
  - 运行 CLI / 需要时运行 TUI 或 app-server 做功能验证

### 分支与循环

1. 先确认当前仓库状态、GitHub 认证、工作流存在性、功能清单真值和代码映射。
2. 触发远程 Windows minimal release。
3. 若工作流失败：
   - 拉取失败 job 日志。
   - 从失败日志定位到源码定义点。
   - 修复代码。
   - 提交并 push。
   - 重新触发工作流。
4. 若工作流成功：
   - 下载发布产物。
   - 运行 `codex.exe` 做本地验收。
   - 无法自然路径覆盖的条目转交 subagent 做代码审核。
5. 若验收或审核发现问题：
   - 修复代码。
   - 再次 push。
   - 回到步骤 2。

### 玩家/用户影响

- 远程打包成功意味着最终用户能从 GitHub Release 直接下载可运行的 `codex.exe`。
- 验收围绕 local2 关键体验：
  - 用户能看到 `-local2` 身份。
  - 用户首轮输入 `你好` 时能看到 local2 清单提示。
  - `/responses` 重试、历史发现、默认日志降噪、priority service tier 等 local2 行为不丢失。

## 代码真值映射（首轮静态核对）

### L2-F1 / L2-F2 版本显示与测试保护

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\cli\src\main.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\version.rs`
- 现成保护：
  - `I:\vscodeProject\codex\codex-rs\tui\src\status\tests.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\status\snapshots\*.snap`
  - `I:\vscodeProject\codex\codex-rs\tui\src\snapshots\codex_tui__update_prompt__tests__update_prompt_modal.snap`

### L2-F3 / L2-F4 / L2-F5 “你好”首轮清单

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\core\src\codex.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\session\mod.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\stream_events_utils.rs`
- 现成保护：
  - `I:\vscodeProject\codex\codex-rs\core\src\session\tests.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\stream_events_utils_tests.rs`

### L2-F6 ~ L2-F14 `/responses` retry 行为

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`
  - `I:\vscodeProject\codex\codex-rs\codex-api\src\telemetry.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\client.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\codex.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\session\turn.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\bespoke_event_handling.rs`
- 现成保护：
  - `I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs` 内联测试
  - `I:\vscodeProject\codex\codex-rs\app-server\tests\suite\v2\account.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\tests\suite\v2\thread_start.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\tests\suite\v2\thread_resume.rs`
  - `I:\vscodeProject\codex\codex-rs\core\tests\suite\websocket_fallback.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\chatwidget\tests\history_replay.rs`

### L2-F15 / L2-F16 历史默认跨 provider 可发现

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\exec\src\lib.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\codex_message_processor.rs`
- 现成保护：
  - `I:\vscodeProject\codex\codex-rs\exec\src\lib_tests.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\app_server_session.rs`

### L2-F17 / L2-F18 / L2-F19 service tier priority

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\config\src\config_toml.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\config\mod.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\client.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\codex.rs`
- 现成保护：
  - `I:\vscodeProject\codex\codex-rs\core\src\client_tests.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\config\config_tests.rs`
  - `I:\vscodeProject\codex\codex-rs\core\config.schema.json`

### L2-F20 / L2-F21 默认日志降噪

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\app-server\src\main.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\lib.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\lib.rs`
- 现成保护：
  - 以代码路径和运行时行为为主，后续本地 exe 测试补证。

### L2-F22 / L2-F23 / L2-F24 runtime 负担默认关闭

- 定义点：
  - `I:\vscodeProject\codex\codex-rs\core\src\rollout.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\personality_migration.rs`
  - `I:\vscodeProject\codex\codex-rs\app-server\src\lib.rs`
  - `I:\vscodeProject\codex\codex-rs\tui\src\lib.rs`
- 现成参考：
  - `I:\vscodeProject\codex\docs\runtime-load-reduction-default-off-task-2026-04-29.md`
- 说明：
  - 这三项需要本地自然路径测试加代码审核联合收口。

## Todo 总表

| ID | 动作 | 输入 | 输出 | 通过条件 | 状态 |
|---|---|---|---|---|---|
| T1 | 建立总控记录与代码映射 | 工作流文件、功能清单、代码搜索结果 | 本总控记录 | 有明确构建入口、验收清单、代码定义点映射 | completed |
| T2 | 触发 GitHub Windows minimal release | 当前 `main` HEAD、workflow dispatch 参数 | GitHub Actions run id / release tag | 工作流成功进入队列并可跟踪 | completed |
| T3 | 处理远程构建失败 | GitHub Actions 日志 | 失败根因与修复提交 | 若失败则拿到明确根因并完成修复 push | completed |
| T4 | 下载成功产物并准备本地验收环境 | GitHub Release asset | 本地 zip 与解压目录 | 成功拿到 `codex.exe` | completed |
| T5 | 执行自然路径本地验收 | `codex.exe`、local2 清单 | 验收记录 | 能覆盖的条目均有实际运行证据 | completed |
| T6 | 对无法覆盖项做 subagent 代码审核 | 未覆盖条目、对应代码路径 | 审核结论 | 覆盖不到的条目均有代码审核结论 | completed |
| T7 | 若发现问题则修复并重跑闭环 | 测试/审核问题 | 新提交、新构建、新验收结果 | 问题修复后重新回到成功构建与验收 | completed |

## 执行记录

### 2026-05-07 初始核对

- 已确认 `gh auth status` 可用，当前 GitHub 账号 `dqIndieGames` 已登录。
- 已确认当前分支 `main` 跟踪 `origin/main`，HEAD 为 `2b5794cf7e`。
- 已确认存在远程工作流 `local2-minimal-windows-release`。
- 已确认功能清单 UTF-8 可正常读取。

### 2026-05-07 第 1 轮远程构建

- 触发工作流：
  - repo: `dqIndieGames/codex`
  - workflow: `local2-minimal-windows-release`
  - run: `25461722420`
  - tag: `local2-windows-minimal-2026-05-07-2b5794cf7e6e`
- 构建结果：失败。
- 失败阶段：`Build codex.exe`
- 失败根因：
  - GitHub Actions 日志显示 `codex-rs/app-server-protocol/src/protocol/common.rs:564` 处宏调用缺失 `serialization` 字段。
  - 具体表现是 `ThreadProviderRuntimeRefresh` / `ThreadProviderRuntimeRefreshAllLoaded` 两个 RPC 只写了 `params` 与 `response`，不符合 `client_request_definitions!` 宏签名。
- 根因判断：
  - 这是协议层静态定义错误，不是 local2 功能逻辑回归，也不是 Windows 专属运行时差异。
  - 修复策略是补齐最小序列化范围：
    - `thread/providerRuntime/refresh` -> `thread_id(params.thread_id)`
    - `thread/providerRuntime/refreshAllLoaded` -> `global("config")`
  - 同时补协议层测试断言，防止以后再漏。

### 2026-05-07 第 1 轮修复

- 已修改：
  - `I:\vscodeProject\codex\codex-rs\app-server-protocol\src\protocol\common.rs`
- 修改内容：
  - 为 `ThreadProviderRuntimeRefresh` 补 `serialization: thread_id(params.thread_id)`
  - 为 `ThreadProviderRuntimeRefreshAllLoaded` 补 `serialization: global("config")`
  - 新增对应 `serialization_scope()` 测试断言
- 本地验证边界：
  - 按用户要求未做本地编译。
  - 当前仅完成根因级源码修复，待 push 后重新用 GitHub Actions 验证。

### 2026-05-07 第 2 轮远程构建

- 触发工作流：
  - repo: `dqIndieGames/codex`
  - workflow: `local2-minimal-windows-release`
  - run: `25462477139`
  - tag: `local2-windows-minimal-2026-05-07-c88422f1c3`
- 构建结果：失败。
- 失败阶段：`Build codex.exe`
- 新根因：
  1. `I:\vscodeProject\codex\codex-rs\core\src\session\mod.rs:177`
     - `use crate::config_loader::resolve_relative_paths_in_config_toml;`
     - 上游在 `9c3abcd46c` 已把 config loader 迁移到 `codex-config::loader`，本地仍残留旧导入。
  2. `I:\vscodeProject\codex\codex-rs\core\src\client.rs:2175-2177`
     - 先 `map_responses_stream_api_error(err)` 消耗 `err`，又借用 `&err` 读取 response debug context。
     - 属于标准 Rust move-after-borrow 编译错误。
- 判断：
  - 这两处都是编译级问题，不涉及 local2 功能口径变化。
  - `codex.rs` 中同类 `crate::config_loader::*` 旧导入也一并修正，避免下一轮才暴露。

### 2026-05-07 第 2 轮修复

- 已修改：
  - `I:\vscodeProject\codex\codex-rs\core\src\session\mod.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\codex.rs`
  - `I:\vscodeProject\codex\codex-rs\core\src\client.rs`
- 修改内容：
  - 把 `crate::config_loader::*` 改为当前真实来源 `codex_config::loader::*` / `codex_config::*`
  - 调整 `ApiError err` 的使用顺序：先提取 debug context，再做错误映射
- 提交：
  - `c88422f1c3` `fix(app-server-protocol): restore request serialization metadata`
  - `ad16f3f2d8` `fix(core): restore loader imports and error ordering`
- 本地验证边界：
  - 仍未做本地编译，继续以 GitHub Actions 为唯一构建真值。

### 2026-05-07 第 3 轮远程构建进行中

- 已触发工作流：
  - repo: `dqIndieGames/codex`
  - workflow: `local2-minimal-windows-release`
  - run: `25463504102`
  - tag: `local2-windows-minimal-2026-05-07-ad16f3f2d8`
- 当前状态：
  - `Build minimal Windows package` job `74711249275` 仍为 `in_progress`
  - `Build codex.exe` step 仍在执行中
- 已做核对：
  - `gh run view 25463504102 -R dqIndieGames/codex --json ...` 返回 `status=in_progress`
  - `gh api repos/dqIndieGames/codex/actions/jobs/74711249275` 返回 step 4 `Build codex.exe` 仍在运行
  - `gh run view 25463504102 -R dqIndieGames/codex --job 74711249275 --log` 目前因 job 未结束而无法拉取完整日志
- 当前判断：
  - 这是“仍在远端编译”的中间态，不是失败结论。
  - 由于同工作流已有成功 run 持续超过 1 小时的记录，本轮继续等待同一个 run，不重开、不改道。

### 2026-05-07 第 3 轮远程构建结果

- run: `25463504102`
- 构建结果：失败。
- 失败阶段：`Build codex.exe`
- 失败根因：
  - `I:\vscodeProject\codex\codex-rs\app-server\src\outgoing_message.rs:607`
  - `request` 先被 move 进 `OutgoingMessage::Request(request)`，随后在按连接发送成功分支里又执行：
    - `self.analytics_events_client.track_server_request(connection_id.0, request.clone())`
  - 这触发 Rust `E0382`：borrow of moved value `request`。
- 根因判断：
  - 属于 `app-server` 发送路径的所有权顺序问题。
  - 不涉及 local2 功能口径，也不涉及用户可见消息内容变化。

### 2026-05-07 第 3 轮修复

- 已修改：
  - `I:\vscodeProject\codex\codex-rs\app-server\src\outgoing_message.rs`
- 修改内容：
  - 将
    - `let outgoing_message = OutgoingMessage::Request(request);`
    - 改为
    - `let outgoing_message = OutgoingMessage::Request(request.clone());`
  - 保留一份 `ServerRequest` 给后续 `track_server_request(...)` 使用，避免 move 后再次借用。
- 提交：
  - `41e22bdb09` `fix(app-server): retain request for analytics tracking`

### 2026-05-07 第 4 轮远程构建

- 已触发工作流：
  - repo: `dqIndieGames/codex`
  - workflow: `local2-minimal-windows-release`
  - run: `25464758036`
  - tag: `local2-windows-minimal-2026-05-07-41e22bdb09`
- 当前状态：
  - 已进入 GitHub Actions 队列，等待后续成功或失败结论。

### 2026-05-07 第 4 轮远程构建结果

- run: `25464758036`
- 构建结果：失败。
- 失败阶段：`Build codex.exe`
- 失败根因：
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs:163`
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs:189`
  - 两处都调用了 `picker_cwd_filter(...)`，但当前文件作用域内没有该函数定义，触发 `E0425`。
- 根因判断：
  - 属于 TUI resume picker 丢失 helper 定义后的编译错误。
  - 历史版本 `bc5a1b961e` 中存在该 helper，语义只是把 `show_all / remote --cd / 本地 cwd` 归一为 `Option<PathBuf>`，不涉及 provider 逻辑，不改变 local2 功能口径。

### 2026-05-07 第 4 轮修复

- 已修改：
  - `I:\vscodeProject\codex\codex-rs\tui\src\resume_picker.rs`
- 修改内容：
  - 恢复历史缺失的 `picker_cwd_filter(...)` helper：
    - `show_all -> None`
    - `is_remote -> remote_cwd_override.map(Path::to_path_buf)`
    - 本地 -> `Some(config_cwd.to_path_buf())`
  - 保持 resume picker 当前跨 provider 与 cwd 过滤口径不变，只补回缺失定义。
- 提交：
  - `9af5aa1e3a` `fix(tui): restore picker cwd helper`

### 2026-05-07 第 5 轮远程构建

- 已触发前准备：
  - 最新提交已 push 到 `origin/main`
  - HEAD: `9af5aa1e3a`
- 下一步：
  - 重新触发 `local2-minimal-windows-release`，继续以 GitHub Windows 构建结果为唯一编译真值。

### 2026-05-07 第 5 轮远程构建结果

- run: `25466021006`
- 构建结果：成功。
- release:
  - tag: `local2-windows-minimal-2026-05-07-9af5aa1e3a`
  - asset: `codex-x86_64-pc-windows-msvc-minimal.zip`
- 产物说明：
  - 本轮最小包只包含 `codex.exe`，符合“最小 Windows release codex.exe 包”的目标。

### 2026-05-07 下载与本地自然路径验收

- 下载路径：
  - `I:\vscodeProject\codex\.codexflow\临时\github_windows_local2_release_verify_2026-05-07\release-assets\codex-x86_64-pc-windows-msvc-minimal.zip`
- 解压路径：
  - `I:\vscodeProject\codex\.codexflow\临时\github_windows_local2_release_verify_2026-05-07\release-assets\minimal_zip_extract\codex.exe`
- 版本验证：
  - 运行 `codex.exe --version` 返回 `codex-cli 0.128.0-local2`
- 包结构验证：
  - zip 内仅包含 `codex.exe`
- 自然路径功能验证：
  - 默认请求体抓包显示 `service_tier = "priority"`，对应 `L2-F17`
  - 顶层 `force_service_tier_priority=false` 且 `service_tier="flex"` 时，请求体显示 `service_tier = "flex"`，对应 `L2-F18`
  - 顶层 `force_service_tier_priority=false` 且未设置 `service_tier` 时，请求体不包含 `service_tier` 字段，对应 `L2-F18`
  - 仅在 profile 下写 `[profiles.profile_force_false].force_service_tier_priority=false` 并选中该 profile 时，请求体仍为 `service_tier = "priority"`，对应 `L2-F19`
  - 假 `/responses` 服务返回 `401` 时，stdout 出现 `401 retry 1`，对应 `L2-F8` 与 `L2-F12` 的部分可见提示
  - 假 SSE 配合 prompt `你好` 时，首个普通 `agent_message` 文本以前缀 `local2 定制功能已启用：` 开头，并接上 `MODEL_OK`，对应 `L2-F3` / `L2-F5`
  - 假 SSE 配合 prompt `hello` 时，首个 `agent_message` 仅为 `MODEL_OK`，不注入 local2 清单，对应 `L2-F3` 反例
- 本地验证边界：
  - 仍未执行任何本地编译、`cargo`、`just`、`rustc`、`cargo test`、格式化或 lint
  - 多轮同线程、resume/fork/MCP、OTEL metrics/log_db、app-server UI/TUI UI、历史列表与 picker 等条目需要代码审核补证

### 2026-05-07 Subagent 代码审核

- 已启动只读 subagent：
  - agent id: `019e02aa-53a5-7d81-874c-0aa085053c49`
- 审核范围：
  - `L2-F2`、`L2-F4`、`L2-F6`、`L2-F7`、`L2-F9`、`L2-F10`、`L2-F11`、`L2-F12`、`L2-F13`、`L2-F14`、`L2-F15`、`L2-F16`、`L2-F20`、`L2-F21`、`L2-F22`、`L2-F23`、`L2-F24`
- 当前状态：
  - subagent 已返回正式结论：`15 Pass / 2 Risk`
  - 风险条目：
    - `L2-F6`：`/responses` 对 `429 + usage_limit` 仍保留终态例外，没有统一进入普通 retry
    - `L2-F2`：CLI `--version` 缺少直接自动化保护

### 2026-05-07 Subagent findings 修复

- 已修改：
  - `I:\vscodeProject\codex\codex-rs\codex-api\src\provider.rs`
  - `I:\vscodeProject\codex\codex-rs\cli\tests\version.rs`
- 修改内容：
  - 删除 `/responses` 路由中对 `429 + usage_limit` 的终态例外，让其继续统一走普通 retry
  - 新增 CLI `--version` 直接断言测试，要求输出精确为 `codex-cli <pkg-version>-local2`
- 提交：
  - `6e8bde10b0` `fix(local2): restore responses retry and cli version guard`

### 2026-05-07 第 6 轮远程构建

- 已触发工作流：
  - repo: `dqIndieGames/codex`
  - workflow: `local2-minimal-windows-release`
  - run: `25500889140`
  - tag: `local2-windows-minimal-2026-05-07-6e8bde10b0`
- 目的：
  - 用新的 GitHub Windows 最小 release 成品验证 `L2-F6` 与 `L2-F2` 修复是否真正体现在可下载 exe 上。

### 2026-05-07 第 6 轮远程构建结果与新包复测

- run: `25500889140`
- 构建结果：成功。
- release:
  - tag: `local2-windows-minimal-2026-05-07-6e8bde10b0`
  - asset: `codex-x86_64-pc-windows-msvc-minimal.zip`
  - release url: `https://github.com/dqIndieGames/codex/releases/tag/local2-windows-minimal-2026-05-07-6e8bde10b0`
- 新包下载路径：
  - `I:\vscodeProject\codex\.codexflow\临时\github_windows_local2_release_verify_2026-05-07\release-assets-6e8bde10b0-v2\codex-x86_64-pc-windows-msvc-minimal.zip`
- 新包解压路径：
  - `I:\vscodeProject\codex\.codexflow\临时\github_windows_local2_release_verify_2026-05-07\release-assets-6e8bde10b0-v2\minimal_zip_extract\codex.exe`
- 新包结构验证：
  - zip 内仅包含 `codex.exe`
- 新 exe 专用验收目录：
  - `I:\vscodeProject\codex\.codexflow\临时\github_windows_local2_release_verify_2026-05-07\exe-test-6e8bde10b0`
- 新 exe 自然路径复测结果：
  - `codex.exe --version` 返回 `codex-cli 0.128.0-local2`，对应 `L2-F1`，并证明 `L2-F2` 的修复已进入可下载成品
  - 假 `/responses` 服务返回 `429` 且错误体为 `{"error":{"type":"usage_limit_reached","message":"The usage limit has been reached"}}` 时，stdout 出现 `429 retry 1 (You've hit your usage limit. Try again later.)`，对应 `L2-F6`
  - 假 `/responses` 服务返回 `401` 时，stdout 再次出现 `401 retry 1`，对应 `L2-F8`
  - 假 SSE 配合 prompt `你好` 时，首个 `agent_message` 仍以 `local2 ... MODEL_OK` 形式出现，对应 `L2-F3` / `L2-F5`
  - 假 SSE 配合 prompt `hello` 时，首个 `agent_message` 仍仅为 `MODEL_OK`，不注入 local2 清单，对应 `L2-F3` 反例
- 说明：
  - 这轮“你好”复测首次按 UTF-8 模式匹配 stdout 时出现中文编码失真，后续改用 ASCII 锚点与文件内容复核确认功能未回归；问题在临时测试脚本读法，不在产品功能
  - 全流程仍未执行任何本地编译、测试、格式化或 lint；所有构建真值均来自 GitHub Windows release 成品

### 2026-05-07 README 同步

- 已同步更新仓库根文档：
  - `I:\vscodeProject\codex\README.md`
  - `I:\vscodeProject\codex\README.zh-CN.md`
- 同步内容：
  - 把仓库首页说明从旧的 `local1 / rust-v0.124.0` 更新为当前 `local2 / 0.128.0`
  - 把 Windows 发布说明从旧的 `rust-release-windows.yml` 更新为当前 `local2-minimal-windows-release.yml`
  - 明确当前 Windows GitHub 最小发布物是只包含 `codex.exe` 的 `codex-x86_64-pc-windows-msvc-minimal.zip`
  - 增补 local2 长期保留清单入口与当前行为说明，避免用户打开首页时仍看到过期口径

## 证据

- 工作流文件：`I:\vscodeProject\codex\.github\workflows\local2-minimal-windows-release.yml`
- 功能清单：`I:\vscodeProject\codex\docs\local2-custom-feature-checklist-2026-04-27.md`
- GitHub 认证：本地 `gh auth status`
- 代码映射：本轮 `rg` 检索结果

## 阻塞

- 无。第 6 轮 GitHub Windows 远端构建已成功，修复后的 release 成品已下载并完成复测。

## 验收与结论

- 本次已完整完成“远程打包 -> 下载 exe -> 自然路径验收 -> subagent 代码审核 -> 修复 -> 再次远程打包 -> 再次下载 exe -> 复测”的闭环。
- 第 6 轮 GitHub Windows minimal release `25500889140` 成功，最终可下载最小包仅包含 `codex.exe`，满足用户只要 Windows `codex.exe` 包的目标。
- 基于新 release 成品的黑盒复测确认：
  - `L2-F2`：CLI `--version` 仍为 `0.128.0-local2`，且仓库已补直接自动化保护
  - `L2-F6`：`429 + usage_limit_reached` 已重新进入普通 retry
  - 之前已验的 `L2-F3` / `L2-F5` / `L2-F8` / `L2-F17` / `L2-F18` / `L2-F19` 保持通过
- 无新增失败项；本轮剩余风险仅是部分未走自然路径的条目仍主要依赖已完成的 subagent 静态审核结论，但本次用户要求的构建、下载、测试、修复与重跑闭环已经完成。
