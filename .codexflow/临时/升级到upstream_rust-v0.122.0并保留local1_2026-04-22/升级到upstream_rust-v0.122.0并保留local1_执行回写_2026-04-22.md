# 升级到 upstream `rust-v0.122.0` 并保留 `local1`_执行回写_2026-04-22

## 执行概览

本次任务已经按“先代码复核，后最终 release build，再做 release smoke”的顺序完成。没有插入 debug 构建，也没有补跑用户明确禁止的测试链。

## 约束执行情况

- 已遵守：不执行 `cargo test -p codex-core`。
- 已遵守：不执行任何非最终 release 的补充编译链。
- 已遵守：不打 debug 包。
- 已遵守：只在代码复核闭环后执行最终 release build。
- 已遵守：无法直接通过 release 包覆盖的内部逻辑，统一交给 subagent 做静态代码复核。

## 代码修正回写

本次为完成 `rust-v0.122.0` 升级并保留 `local1`，实际收口的关键文件如下：

1. `codex-rs/core/src/compact.rs`
   - 补回 `retry_delay_for_error` 的正常导入。
   - 删除未使用的 `PreviousTurnSettings` 导入。

2. `codex-rs/core/src/client.rs`
   - 修复 `build_live_api_auth` 的 provider/auth 取值错误，避免 runtime refresh 后继续拿错 auth。
   - 补回 `401 unauthorized recovery` 相关恢复链路与 telemetry 状态衔接。

3. `codex-rs/core/src/session/session.rs`
   - 恢复 `pending_provider_runtime_refresh` 字段，并在 `Session::new` 中初始化。

4. `codex-rs/core/src/session/mod.rs`
   - 恢复 provider runtime refresh 主链，包括读取 user config、解析 runtime refresh、应用 pending refresh、刷新 live runtime 等逻辑。

5. `codex-rs/app-server/src/message_processor.rs`
   - 修复 `codex_home()` 返回值类型，改为 `to_path_buf()`，让 app-server 控制面 release build 能通过。

## 审核链闭环

### 前置 findings 闭环

- 早期 reviewer findings 已经在本轮执行中完成修正，不再保留未闭环的高优先级问题进入最终 release 阶段。

### 最终 reviewer subagent

- subagent：`Mill`
- agent id：`019db7ed-65d7-72f0-9e79-34cb71811130`
- 最终状态：`completed`
- 最终结论：`no findings`

最终 reviewer 复核覆盖了三块关键修正：

- `401 unauthorized recovery` 恢复链是否完整；
- `provider runtime refresh` 字段与方法补回后，当前会话与下一轮 turn 是否读取同一份 live runtime；
- `codex_home.to_path_buf()` 是否是最小且正确的 app-server build 修复。

正式收口文档见：
- `升级到upstream_rust-v0.122.0并保留local1_最终代码复核结论_2026-04-23.md`

## Release build 回写

### 官方 Bazel 路径

- 原计划主路径：`just build-for-release`
- 实际对应：`bazel build //codex-rs/cli:release_binaries --config=remote`
- 当前状态：在这台 Windows 机器上仍受外部工具链 / 符号链接相关环境问题阻塞
- 处理方式：保留真实阻塞记录，不把这条路径伪装成成功

### 本次唯一成功的 release build

- 命令：`cargo build --release -p codex-cli --bin codex --locked`
- 工作目录：`I:\vscodeProject\codex\codex-rs`
- 结果：`exit 0`
- 日志摘要：`Finished release profile [optimized] target(s) in 52m 14s`
- 产物：`I:\vscodeProject\codex\codex-rs\target\release\codex.exe`
- 原始日志：
  - `I:\vscodeProject\codex\tmp\agent-snapshots\cargo-release\cargo_release.stdout.log`
  - `I:\vscodeProject\codex\tmp\agent-snapshots\cargo-release\cargo_release.stderr.log`

## Release smoke 回写

本次 release smoke 只针对最终 release 产物执行，没有引入任何额外编译：

1. `I:\vscodeProject\codex\codex-rs\target\release\codex.exe --version`
   - 结果：`exit 0`
   - 输出：`codex-cli 0.122.0-local1`

2. `I:\vscodeProject\codex\codex-rs\target\release\codex.exe --help`
   - 结果：`exit 0`
   - 结果要点：CLI usage、主命令列表、`--version`/`--help` 等入口正常显示

原始 smoke 记录位于：
- `I:\vscodeProject\codex\.codexflow\临时\升级到upstream_rust-v0.122.0并保留local1_2026-04-22\release_runtime_smoke.log`

## 残余限制与未做项

- 未执行任何非 release tests，这是用户明确要求，不属于遗漏执行。
- 官方 `just build-for-release` / Bazel 路径在当前 Windows 机器仍然受环境阻塞，这是环境限制，已明确记录，不做粉饰。
- 因为本轮必须遵守“只做最后 release build”的约束，所以没有再补任何 `cargo test`、`cargo check`、debug build 或中间验证编译。

## 最终结果

- `rust-v0.122.0` 升级所需的关键源码收口已完成。
- `local1` 定制能力保留结论已回写到最终状态矩阵。
- 最终 reviewer subagent 结论为 `no findings`。
- release 二进制已成功构建并完成最小 smoke。
- 官方 Bazel release 路径仍有环境阻塞，但该事实已被保留并与已成功的 cargo release 结果清晰区分。