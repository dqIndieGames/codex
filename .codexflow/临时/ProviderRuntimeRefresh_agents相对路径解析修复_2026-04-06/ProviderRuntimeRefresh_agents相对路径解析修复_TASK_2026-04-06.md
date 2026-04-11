# ProviderRuntimeRefresh_agents相对路径解析修复_TASK_2026-04-06

## Context
- 当前 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 在重读最新 effective config 时，会走 `codex-rs/core/src/codex.rs` 中的 `read_latest_provider_runtime_refresh(...)`。
- 该路径目前直接对 `merged_toml` 执行 `try_into()`，没有复用正常配置加载时的 base-path-aware 反序列化逻辑。
- 用户配置中的 `[agents.*].config_file` 当前允许使用相对路径，既有口径是“相对路径相对于定义它的 `config.toml`”。正常加载时该口径成立，但 refresh 路径会报 `AbsolutePathBuf deserialized without a base path in agents`。
- 该错误会沿着 `refreshAllLoaded -> failed_threads` 向上冒泡，最终让 Windows tray 把整个 app-server 实例记为失败；这属于下游表现，不是根因。
- 本次任务除了修复代码，还必须把该缺陷与验收口径归档到 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md` 的 `F10` 与“同步官方后的必查清单”中。
- 本任务全过程禁止编译、构建、测试执行、格式化、lint；所有判断、复核和查漏都只能基于源码阅读与静态链路核对。

## Goal
- 修复 provider runtime refresh 的配置读取路径，使其在 refresh 场景下也正确支持 `[agents.*].config_file` 相对路径。
- 修复后，合法的相对路径配置不再因为 `agents` 段解析失败而导致 `refreshAllLoaded` 产生 `failed_threads`。
- 保持修复范围收敛在 refresh 配置读取链路，不扩大到 tray 统计规则、named pipe、实例注册或 RPC 语义层。
- 保留真正的失败场景：例如 provider 已被删除、`config.toml` 本身语法损坏，这类情况仍应继续失败，不能被本修复误吞掉。
- 把本次缺陷补充归档到本地定制功能清单中，作为后续同步官方更新时的固定回归点。

## Checklist
- 核对 `codex-rs/core/src/codex.rs` 中 `read_latest_provider_runtime_refresh(...)` 的当前实现，确认根因是 `merged_toml.try_into()` 未携带 `config.toml` 所在目录作为反序列化基准。
- 修改 refresh 配置读取逻辑，复用与正常配置加载一致的 base-path-aware `ConfigToml` 反序列化约束，优先使用 `codex-rs/core/src/config/mod.rs` 中现有 helper 或等价复用点，而不是额外发明第二套路径解析规则。
- 反序列化基准目录固定取当前 user `config.toml` 所在目录，确保 `[agents.*].config_file = "agents/xxx.toml"` 与 `./agents/xxx.toml` 在 refresh 场景下都遵守既有口径。
- 不修改 `thread/providerRuntime/refresh`、`thread/providerRuntime/refreshAllLoaded` 的接口名、返回结构和 bulk refresh 成功判定规则；tray 仍按 `ok == true` 且 `failed_threads` 为空来统计实例成功或失败。
- 补源码级测试覆盖，但只允许写测试代码，不允许执行测试：
  - core 侧优先落到 `codex-rs/core/src/codex_tests.rs`，补“带相对 `agents.*.config_file` 的配置下 `refresh_provider_runtime()` 成功”的用例；
  - core 侧在同文件保留并补强“provider 缺失仍失败”“坏 `config.toml` 仍失败”的用例；
  - app-server 侧优先落到 `codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs`，补“`refreshAllLoaded` 在相同配置下不再返回 `failed_threads`”的用例。
- 更新 `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md`：
  - 只补 `F10`，不新增新的 `F*` 项；
  - 在 `F10` 的 `明确定义`、`当前代码迹象`、`后续验收口径` 中补充本次相对路径 refresh 缺陷与修复口径；
  - 在“同步官方后的必查清单”新增一条对应回归项，明确要求 future sync 后仍不因 `agents` 相对路径导致 refresh 失败。
- 修改完成后，必须使用 reviewer subagent 做一次只读静态复核，检查是否仍有遗漏、范围扩张或文档口径冲突；复核过程同样禁止编译、构建和测试执行。

## Acceptance
- `read_latest_provider_runtime_refresh(...)` 不再直接对 `merged_toml` 做裸 `try_into()`，而是明确走带 base path 的 `ConfigToml` 反序列化。
- 合法的 `[agents.*].config_file` 相对路径在 provider runtime refresh 场景下不再触发 `AbsolutePathBuf deserialized without a base path in agents`。
- `refreshAllLoaded` 的 bulk refresh 统计语义保持不变；本次只是消除错误来源，不改单实例成功/失败的判定口径。
- provider 缺失、坏 `config.toml` 这两类真正错误继续保留为失败路径，不被本修复误吞掉。
- `E:\vscodeProject\codex_github\codex\docs\local1-custom-feature-checklist-2026-03-28.md` 已补充 `F10` 与“同步官方后的必查清单”对应条目，且内容能独立说明本次缺陷与回归目标。
- 本次任务的代码复核、遗漏检查和最终结论全部基于代码阅读；文档、代码和复核要求中均不得出现编译、构建、测试执行要求。

## Notes
- 本次修复点是 refresh 配置读取路径，不是 provider 本身、tray UI 或实例统计逻辑。
- `windows_app_server_refresh_tray.py` 当前按实例统计成功/失败的逻辑不在本轮修改范围内；根因修复后它应自然恢复正确统计。
- 本轮允许新增或修改测试源码文件，但这些测试只作为静态保护代码写入，不运行。
- 归档文档只补 `F10` 及其必查清单，不新增独立 `F13`、`F14` 等编号，避免把同一条 provider runtime 热刷新链拆散。

## 用户/玩家视角直观变化清单
- 用户再次点击 Windows tray 的“刷新全部 app-server”时，不会再因为合法的 `agents` 相对路径配置而被误报为实例刷新失败。
- 当配置本身合法时，tray 的成功/失败统计会回到真实 refresh 结果，而不是被 `agents` 路径解析错误污染。
- 当配置本身仍然真的有问题，例如 provider 被删除或 `config.toml` 语法损坏，tray 仍会继续显示失败，不会被错误伪装成成功。

## 执行回写
- 已完成代码修复：`codex-rs/core/src/codex.rs`
  - 在 `read_latest_provider_runtime_refresh(...)` 中先取当前 user `config.toml` 父目录作为 `config_base_dir`
  - 先对最新 user layer 调用 `resolve_relative_paths_in_config_toml(...)`
  - 再对 merged effective config 调用 `deserialize_config_toml_with_base(...)`
  - 保持 `thread/providerRuntime/refresh` 与 `thread/providerRuntime/refreshAllLoaded` 的接口名、返回结构和 bulk refresh 成功判定语义不变
- 已完成测试源码补充：`codex-rs/core/src/codex_tests.rs`
  - 新增“带相对 `agents/reviewer.toml` 配置时 `refresh_provider_runtime()` 仍成功”的回归用例
  - 保留并补强“provider 缺失仍失败”的静态测试源码，使其在存在相对 `agents` 配置时仍验证真实失败路径
  - 保留“坏 `config.toml` 仍失败”的静态测试源码
- 已完成 app-server 侧测试源码补充：`codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs`
  - 新增 `refreshAllLoaded` 在 `./agents/reviewer.toml` 场景下仍保持 `failed_threads.is_empty()` 的回归用例
- 已完成归档：`docs/local1-custom-feature-checklist-2026-03-28.md`
  - 仅补充 `F10` 三列中与 `[agents.*].config_file` relative path refresh 回归直接相关的口径
  - 在“同步官方后的必查清单”新增 1 条对应回归项
- 本次任务未执行任何编译、构建、测试、格式化、lint；全部判断基于源码阅读、静态 diff 核对与 reviewer subagent 复核

## Subagent 正确性复核
- reviewer subagent：`019d5f84-b177-7a22-989e-13efbcdf2dc7`（`reviewer` 角色，跟随主配置 `gpt-5.4` / `xhigh`）
- 首次复核提示了 2 条 findings：
  - `codex-rs/core/src/codex.rs` 中还有与本任务无关的 runtime 行为 diff
  - `docs/local1-custom-feature-checklist-2026-03-28.md` 中还有与本任务无关的历史归档 diff
- 经补充边界说明后，reviewer 按 task-local 范围复判结论为：`no findings`
  - 根因修复点准确落在 `read_latest_provider_runtime_refresh(...)`
  - core 成功回归、core 真失败保留、app-server `refreshAllLoaded` 回归三类测试源码覆盖均已齐备
  - `F10` 与必查清单对应归档已齐备
- reviewer 额外指出的残余风险仅为可选项：若后续想做对称性硬化，可在 core 侧再补一个 `./agents/reviewer.toml` 成功变体；该项不是本次任务阻塞问题
