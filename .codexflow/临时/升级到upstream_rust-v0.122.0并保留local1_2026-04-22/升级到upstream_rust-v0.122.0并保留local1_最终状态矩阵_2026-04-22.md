# 升级到 upstream `rust-v0.122.0` 并保留 `local1`_最终状态矩阵_2026-04-22

## 状态定义

- `通过（Release）`：由最终 release 产物直接验证。
- `通过（Mixed）`：既有 release 证据，也有静态代码复核证据。
- `通过（Static）`：无法直接靠 release 包穷尽，只能靠主 agent + subagent 的静态代码复核收口。
- `受阻（Blocked）`：真实存在的环境阻塞，已记录，不伪装成成功。

## 最终状态矩阵

| 项目 | 说明 | 验证方式 | 状态 | 证据 |
|---|---|---|---|---|
| F1 | `-local1` 版本显示链 | release + static | 通过（Mixed） | `codex.exe --version => codex-cli 0.122.0-local1`；CLI/TUI 版本显示链静态复核 |
| F2 | 状态卡片/状态区统一消费 `CODEX_CLI_DISPLAY_VERSION` | static | 通过（Static） | 代码复核收口，无最终 findings |
| F3 | 历史单元/升级提示保留 `local1` 版本口径 | static | 通过（Static） | 代码复核收口，无最终 findings |
| F4 | `local1` 快照与断言基线仍被保留 | static | 通过（Static） | 代码复核收口，无最终 findings |
| F5 | `/responses` 请求层重试范围增强 | static | 通过（Static） | `core/src/client.rs` 与相关链路复核通过 |
| F6 | retry UI/history/telemetry 边界不回退 | static | 通过（Static） | 静态复核通过，无新增 findings |
| F7 | 单次 retry 等待上限仍为 `10s` | static | 通过（Static） | 静态复核通过 |
| F8 | bounded/unbounded retry budget 语义不回退 | static | 通过（Static） | 静态复核通过 |
| F9 | retry classifier 与 route 透传保持统一 | static | 通过（Static） | 静态复核通过 |
| F10 | config-first provider runtime refresh + Windows tray 口径保留 | static | 通过（Static） | `session/mod.rs`、`session/session.rs` 等链路复核通过 |
| F11 | 默认历史发现不按 provider 切割，continue 仍用当前 provider | static | 通过（Static） | 静态复核通过 |
| F12 | `/responses` stream/websocket 的 `401` 继续走普通 retry | static | 通过（Static） | `core/src/client.rs` 相关恢复链补回并经最终 subagent 复核为 no findings |
| F13 | `force_gpt54_priority_fallback` 顶层开关保留 | static | 通过（Static） | 静态复核通过 |
| F14 | Windows app/app-server 默认 `warn` 日志降噪口径保留 | static | 通过（Static） | 静态复核通过 |
| F15 | TUI 默认 `warn` 日志降噪口径保留 | static | 通过（Static） | 静态复核通过 |
| A1 | 首轮固定 checklist 注入规则保留 | static | 通过（Static） | 静态复核通过 |
| A2 | `gpt-5.4 priority fallback` 归档规则保留 | static | 通过（Static） | 静态复核通过 |
| R1 | 本次唯一成功的 release build | release | 通过（Release） | `cargo build --release -p codex-cli --bin codex --locked` exit 0 |
| R2 | 最终 release smoke | release | 通过（Release） | `--version` / `--help` 均 exit 0 |
| R3 | 官方 `just build-for-release` / Bazel 路径 | environment | 受阻（Blocked） | 当前 Windows 机器仍受外部工具链 / 符号链接问题阻塞，已记录到 `release_build.log` |

## 结论

- 对 `local1` 冻结清单中的 `F1-F15` 与 `A1/A2`，本次升级收口结果均已达到“通过（Mixed）”或“通过（Static）”，没有遗留未闭环 findings。
- 对最终 release 产物，本次已经取得“通过（Release）”的构建与 smoke 证据。
- 唯一仍需如实保留的不是代码问题，而是官方 Bazel release 路径在当前 Windows 机器上的环境阻塞。