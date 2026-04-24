# 升级到 upstream_rust-v0.122.0 并保留 local1 reviewer findings

## Findings

### High

1. `thread/providerRuntime/refresh` 仍会读 `OPENAI_BASE_URL`，与启动配置主路径和 README 口径不一致

- 文件与定位：
  - `codex-rs/core/src/codex.rs:4377-4402`，符号：`resolve_provider_runtime_refresh`
  - `codex-rs/core/src/config/mod.rs:1796`，启动配置主路径调用：`built_in_model_providers(openai_base_url)`
  - `codex-rs/app-server/README.md:160-161`
- 风险说明：
  - 启动配置主路径现在只把 `config.toml` 中的 `openai_base_url` 传给 `built_in_model_providers(...)`。
  - 但 provider runtime 热刷新路径在 `resolve_provider_runtime_refresh(...)` 中又把 deprecated `OPENAI_BASE_URL` 环境变量重新合回 built-in provider 表。
  - 这样会导致同一版本里“冷启动配置解析”和“热刷新配置解析”使用两套真值来源。对用户可见的结果是：用户改了 `config.toml`，或者通过 Windows tray 把 source provider 的 `base_url` / `experimental_bearer_token` 复制到当前 provider 后再执行 refresh，接口返回可能仍显示 refresh 成功，但 active thread 的后续自动 retry、新 turn 或后续请求仍可能继续命中旧的环境变量地址。
- 触发条件：
  - 当前线程使用 built-in `openai` provider；
  - 进程环境里存在非空 `OPENAI_BASE_URL`；
  - 调用 `thread/providerRuntime/refresh` 或 `thread/providerRuntime/refreshAllLoaded`。
- 为什么这是高信号问题：
  - 这是静态代码里已经成立的行为分叉，不是单纯“可能缺测试”。
  - README 明确写的是“reload the user config layer”且只刷新当前 provider runtime 的 `base_url` 与 `experimental_bearer_token`；当前实现会额外受旧环境变量影响，文档口径与实际实现不一致。
- 是否阻塞最后唯一一次 `just build-for-release`：
  - 不阻塞构建命令本身。
  - 但按本次“保留 local1 的 provider runtime refresh 真值”目标，建议在唯一一次 release build 前修复，否则 release 行为与文档和用户预期不一致。
- 建议验证点：
  - 预设 `OPENAI_BASE_URL=https://stale.example/v1`；
  - 用户配置写入新的 `openai_base_url` 或当前 provider 的 `base_url = https://new.example/v1`；
  - 对已加载 thread 调用 `thread/providerRuntime/refresh`；
  - 观察 runtime provider、后续 retry 和 next turn 是否仍落到 `stale.example`。

## Residual blind spots

- 本次仅做静态代码复核，没有运行任何编译、测试或构建命令。
- 未对最终 release 包做运行时验证；release 相关结论仍需后续唯一一次 `just build-for-release` 后再做实机复核。
