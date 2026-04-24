# 升级到 upstream `rust-v0.122.0` 并保留 `local1`_最终代码复核结论_2026-04-23

## 复核范围

本次最终复核聚焦于为完成 Windows release build 与 `local1` 保留而补回的关键修正：

- `codex-rs/core/src/compact.rs`
- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/session/session.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/app-server/src/message_processor.rs`

## Subagent 结论

- subagent：`Mill`
- agent id：`019db7ed-65d7-72f0-9e79-34cb71811130`
- 最终状态：`completed`
- 正式结论：`no findings`

subagent 明确复核并确认没有新增确定性问题的点包括：

1. `401 unauthorized recovery`
   - HTTP 与 WebSocket 两条 unauthorized 恢复分支已重新接回。
   - `PendingUnauthorizedRetry` 的恢复阶段与后续 telemetry 衔接正常。

2. `provider runtime refresh`
   - `pending_provider_runtime_refresh` 已补回 `Session` 结构并正确初始化。
   - 刷新后，当前会话配置、原始配置视图与 `ModelClient` live runtime provider 会同步到同一份新值。

3. `codex_home` 返回值修复
   - `self.config.codex_home.to_path_buf()` 是最小且正确的修复，满足 app-server 控制面要求的 `PathBuf` 类型，不引入额外行为变化。

## 主 agent 交叉核对

在不追加任何新编译的前提下，我对最终 release 产物做了最小运行时交叉核对：

- `I:\vscodeProject\codex\codex-rs\target\release\codex.exe --version`
  - `exit 0`
  - 输出：`codex-cli 0.122.0-local1`

- `I:\vscodeProject\codex\codex-rs\target\release\codex.exe --help`
  - `exit 0`
  - CLI usage 与主命令列表正常输出

## 残余缺口

- 本次未运行任何非 release tests，这是用户显式约束，不作为遗漏执行处理。
- 官方 `just build-for-release` / Bazel 路径在当前 Windows 机器仍有环境阻塞，这个限制已另行记录到 `release_build.log`。

## 最终结论

本次最终代码复核结论为 `no findings`。现有补回修正没有发现新的确定性问题；剩余缺口仅限未执行的非 release tests，以及与官方 Bazel 路径相关的环境阻塞记录。