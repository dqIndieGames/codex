# 升级到 upstream `rust-v0.122.0` 并保留 `local1`_TASK_2026-04-22

## 输出位置说明

- 项目根目录解析为 `I:\vscodeProject\codex`。
- 本轮用户明确点名 `$codexflow-temp-output-writer`，所以正式任务产物统一回写到：
  - `I:\vscodeProject\codex\.codexflow\临时\升级到upstream_rust-v0.122.0并保留local1_2026-04-22\`
- 本文是本次升级任务的执行依据与最终回写版本，已经根据实际执行结果完成 checklist 回写。

## Context

- 本次目标是把当前仓库升级到官方 `rust-v0.122.0`，同时保留 `docs/local1-custom-feature-checklist-2026-03-28.md` 中冻结的 `local1` 定制能力。
- 本次必须遵守的用户约束已经执行到底：
  - 不跑 `cargo test -p codex-core`
  - 不跑任何非最终 release 的编译或测试链
  - 不打 debug 包
  - 只在所有代码复核收口后执行最后一次 release build
- 本次执行中，官方 `just build-for-release` / Bazel 路径在当前 Windows 机器上仍然受外部工具链与符号链接相关环境问题阻塞；这个事实已保留在同目录 `release_build.log`，没有被伪装成成功。
- 在该阻塞前提下，本次真正完成并用于最终 smoke 的唯一成功 release 构建为：
  - `cargo build --release -p codex-cli --bin codex --locked`
  - 工作目录：`I:\vscodeProject\codex\codex-rs`

## Goal

- 完成 `rust-v0.122.0` 升级所需的源代码收口。
- 保留 `local1` 冻结清单中的关键能力，至少覆盖 `F1-F15` 与 `A1/A2` 的语义不回退。
- 在最终 release build 前完成 subagent 代码复核，并根据复核结果修正代码与文档。
- 在最终 release build 后完成 release 产物 smoke，给出可追溯证据。
- 把结果完整回写到 `.codexflow` 目录，形成之后可复查的任务闭环。

## Checklist

- [x] 冻结本次升级真值来源：官方 `rust-v0.122.0`，以及 `docs/local1-custom-feature-checklist-2026-03-28.md`。
- [x] 对照 upstream `122` 变更与本地 `local1` 风险面，收敛需要保留的关键链路。
- [x] 先执行 subagent 代码审核，再做最后的 release build，未把 debug 产物当成验收依据。
- [x] 根据前置 reviewer findings 闭环修正代码与文档，避免把已知问题带入最终 release。
- [x] 修复 release build 关键缺口，实际落点包括：
  - `codex-rs/core/src/compact.rs`
  - `codex-rs/core/src/client.rs`
  - `codex-rs/core/src/session/session.rs`
  - `codex-rs/core/src/session/mod.rs`
  - `codex-rs/app-server/src/message_processor.rs`
- [x] 在代码修正后执行最终 reviewer subagent 复核，最终结论为 `no findings`。
- [x] 执行本次唯一成功的 release build：`cargo build --release -p codex-cli --bin codex --locked`。
- [x] 对 release 产物执行最小 smoke：`codex.exe --version`、`codex.exe --help` 均通过。
- [x] 回写执行记录、最终状态矩阵、最终复核结论和 release 构建记录。
- [x] 显式记录官方 Bazel `build-for-release` 路径在当前 Windows 机器仍受环境阻塞，不把该阻塞隐藏掉。

## Acceptance

- [x] 版本结果已经提升到 `0.122.0-local1`，并能从 release 二进制的 `--version` 直接看到。
- [x] 最终 release 二进制的 `--help` 可以正常输出，说明最基本的 CLI 入口可运行。
- [x] `local1` 关键能力的保留结论已经回写到最终状态矩阵，未遗漏 `F1-F15` 与 `A1/A2`。
- [x] subagent 审核已形成正式结论文件，且最终收口为 `no findings`。
- [x] 本次没有执行 `cargo test -p codex-core`，也没有额外插入非最终 release 的编译链。
- [x] 官方 Bazel 路径的环境阻塞已被保留为真实限制，不把它伪装成成功。

## Notes

- 本次“成功完成”的含义是：源代码升级与 `local1` 保留已完成、最终 subagent 复核已无 findings、唯一成功的 release build 已完成、release smoke 已完成。
- 本次“仍有残余限制”的含义是：官方 `just build-for-release` / Bazel 路径在当前 Windows 机器依然受环境阻塞；这个限制已记录，但不影响本次已经完成的 `cargo` release 二进制验证事实。
- 本次严格遵守用户限制，没有补跑非 release 测试，因此剩余测试缺口只限“未执行非 release tests”，而不是 release 结果本身未验证。
- 详细执行过程见：
  - `升级到upstream_rust-v0.122.0并保留local1_执行回写_2026-04-22.md`
  - `升级到upstream_rust-v0.122.0并保留local1_最终状态矩阵_2026-04-22.md`
  - `升级到upstream_rust-v0.122.0并保留local1_最终代码复核结论_2026-04-23.md`
  - `release_build.log`

## 用户/玩家视角直观变化清单

- 本次 TASK 文档本身无用户/玩家可直接感知的直观变化。
- 当本 TASK 对应的代码结果被用户运行时，最终使用者会直接感知到：
  1. release 二进制版本号已经提升到 `0.122.0-local1`；
  2. CLI 基本入口可正常显示帮助信息；
  3. `local1` 版本显示、provider runtime refresh、`401` 重试链路等定制能力没有在这次升级中被 upstream `122` 冲掉。