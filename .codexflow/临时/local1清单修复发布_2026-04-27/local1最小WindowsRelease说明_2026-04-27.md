# local1 最小 Windows Release 说明_2026-04-27

## 内容

- 源码提交：本 release 关联本次 `local1` 清单修复提交。
- Windows 资产：`codex-windows-x86_64-0.124.0-local1.zip`
- zip 内容：单个 `codex.exe`
- exe 来源：现成 `codex-rs/target/release/codex.exe`
- exe 版本：`codex-cli 0.124.0-local1`
- SHA256：`29F0A42564788E101682CCA6223B00C12B18E726A314D145D619B564312D18B2`

## 重要边界

- 本轮没有执行任何编译、构建或测试。
- 该 zip 是从本机已有 exe 打包而来，未重新编译。
- 因此，该 exe 可能不包含本次提交中的源码文本修正；源码修正会在后续重新构建后进入新的二进制。

## 本次源码/文档变化

- `docs/local1-custom-feature-checklist-2026-03-28.md`：补齐 0.124 当前状态、A1/A2 主表、A1 完整可见文案、当前承载面、推荐验证矩阵和后续同步索引。
- `docs/local1-release-rust-v0.124.0-2026-04-24.md`：修正 Windows exe asset 说明，并收敛 F11/provider provenance 口径。
- `codex-rs/core/src/stream_events_utils.rs`：把 A1 首段清单可见文案同步为“不承诺 `thread/list` 历史 provider provenance 保真”。

## 验证

- `git diff --check`：通过；仅输出 Git 行尾提示。
- UTF-8 回读：通过；无 BOM、无替代字符。
- Subagent 复核：第二轮无 findings。
- 未运行编译、构建、测试、lint。
