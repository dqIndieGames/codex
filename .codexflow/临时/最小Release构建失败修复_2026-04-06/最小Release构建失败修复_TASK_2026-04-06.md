# 最小Release构建失败修复_TASK_2026-04-06

## Summary

- 项目根目录解析为 `E:\vscodeProject\codex_github\codex`。
- 本文已重写为“纯静态执行版”任务真值，当前任务目标是修复已知 `E0308` 借用错误，但全程禁止任何编译、测试、lint、fmt、codegen 或其他会触发编译的命令。
- 本轮只允许一项源码修改：
  - 在 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 的两处 helper 调用给 `config_base_dir` 补 `&`
- 本轮完成口径固定为：
  - 已完成针对已知 `E0308` 的静态最小修复
  - 已完成 helper 契约核对与 pre/post diff 审计
  - 已完成任务回写与 reviewer subagent 两轮复核
  - `release` 构建状态因用户禁止编译而未验证

## Context

- 历史上，本专题已经执行过一次最小 release 构建，命令为：

```powershell
$env:CARGO_TARGET_DIR = 'E:\vscodeProject\codex_github\codex\codex-rs\target\release\2026-04-06'
cargo build --release -p codex-cli --bin codex
```

- 上述命令的历史结果为失败，退出码 `101`，日志位于：
  - `E:\vscodeProject\codex_github\codex\codex-rs\target\release\2026-04-06\build.log`
- 这份失败日志现在只作为历史证据使用，本轮不得重新执行任何 build 验证。
- 当前首个 blocking 错误稳定定位在 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)：
  - `core/src/codex.rs:4341`
  - `core/src/codex.rs:4351`
- 当前工作区里 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 还混有与本任务无关的其他脏改；本任务必须以“执行前当前工作树状态”为 diff 基线，而不是以 `HEAD` 为基线。

## Historical Failure Evidence

- 历史失败日志中的首个 blocking 错误片段为：

```text
error[E0308]: mismatched types
   --> core\src\codex.rs:4341:78
4341 | let user_config = resolve_relative_paths_in_config_toml(user_config, config_base_dir)
     |                                                                    ^^^^^^^^^^^^^^^ expected `&Path`, found `AbsolutePathBuf`
help: consider borrowing here
4341 | let user_config = resolve_relative_paths_in_config_toml(user_config, &config_base_dir)

error[E0308]: mismatched types
   --> core\src\codex.rs:4351:78
4351 | let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, config_base_dir)
     |                                                                      ^^^^^^^^^^^^^^^ expected `&Path`, found `AbsolutePathBuf`
help: consider borrowing here
4351 | let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, &config_base_dir)
```

- 该历史失败证据仅用于支撑当前静态修复方向，不得在本轮被重新编译验证。

## Root Cause

- 根因固定写为：
  - [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 在 `read_latest_provider_runtime_refresh(...)` 中新增了 `config_base_dir` 局部变量。
  - `config_toml_path.parent()` 当前返回 `AbsolutePathBuf`。
  - 后续两个 helper 的函数签名明确要求 `&Path`，但当前代码把 `config_base_dir` 按值传入，触发 Rust 类型不匹配。
  - 编译器历史报错已经直接给出最小修复建议：在两个调用点传 `&config_base_dir`。
- 本轮静态契约证据必须固定引用：
  - [absolute-path/src/lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/utils/absolute-path/src/lib.rs)
  - [config_loader/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config_loader/mod.rs)
  - [config/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config/mod.rs)
- 不允许把根因写成以下错误表述：
  - “relative path 逻辑本身设计错了”
  - “需要改 helper 签名去接受 `AbsolutePathBuf`”
  - “需要重构 refresh 配置读取流程”
  - “需要回退整段 provider runtime refresh 修复”

## Execution Constraints

- 本轮禁止执行以下所有动作：
  - `cargo build`
  - `cargo check`
  - `cargo test`
  - 任何其他 `cargo` 子命令
  - `rustc`
  - `fmt` / `cargo fmt`
  - `clippy`
  - 任何测试、lint、codegen、snapshot 更新、或会隐式触发编译的命令
- 本轮允许的动作仅限于：
  - 源码阅读
  - 文本文件编辑
  - pre-edit / post-edit 基线快照
  - 精确 diff 审计
  - 任务回写
  - reviewer subagent 静态复核
- 本轮禁止创建分支、切换分支、创建 worktree；保持当前工作区执行。

## Modification Boundary

- 当前唯一允许改动的源码文件固定为 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)。
- 允许的最小源码改动固定只有 2 处 token 级变化：
  - `resolve_relative_paths_in_config_toml(user_config, &config_base_dir)`
  - `deserialize_config_toml_with_base(merged_toml, &config_base_dir)`
- 以下内容明确不在本任务改动范围内：
  - [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 中与 stream unauthorized / retry details / websocket retry 可见性相关的其他 diff
  - [codex_tests.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex_tests.rs) 与 [thread_provider_runtime_refresh.rs](/E:/vscodeProject/codex_github/codex/codex-rs/app-server/tests/suite/v2/thread_provider_runtime_refresh.rs) 的测试源码调整
  - [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) 的归档内容
  - 任何导入重排、错误文案修改、格式化、行尾统一或周边链路改造
- 以下文件只作为契约核对来源，不作为计划改动文件：
  - [config_loader/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config_loader/mod.rs)
  - [config/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config/mod.rs)
  - [absolute-path/src/lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/utils/absolute-path/src/lib.rs)

## 需要修改的模块与职责

| 模块 | 文件 | 需要修改的内容 | 技术目的 | 完成口径 |
|---|---|---|---|---|
| core | [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) | 仅在 2 个 helper 调用点补 `&config_base_dir` 借用 | 让调用点与 `&Path` 契约静态对齐 | 已知 `E0308` 调用点静态修正完成，未验证编译结果 |
| core | [config_loader/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config_loader/mod.rs) | 不改，仅核对 `base_dir: &Path` 契约 | 证明 callee 契约未变 | 静态证据完整 |
| core | [config/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config/mod.rs) | 不改，仅核对 `config_base_dir: &Path` 契约 | 证明 callee 契约未变 | 静态证据完整 |
| utils | [absolute-path/src/lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/utils/absolute-path/src/lib.rs) | 不改，仅核对 `parent()` 返回类型 | 证明 caller 侧确实拿到 `AbsolutePathBuf` | 静态证据完整 |

## Detailed Code Modification Checklist

1. 在修改任何源码前，先冻结 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 的执行前基线。
2. 基线快照必须写到：
  - `E:\vscodeProject\codex_github\tmp\agent-snapshots\`
3. 基线至少包含：
  - 目标片段快照
  - 整文件快照
  - 整文件哈希
  - 执行前 `git diff -- codex-rs/core/src/codex.rs` 快照
4. 打开 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)，定位 `read_latest_provider_runtime_refresh(...)`。
5. 保留 `config_base_dir` 的现有求值逻辑与错误信息，不修改：

```rust
let config_base_dir = config_toml_path.parent().ok_or_else(|| {
    CodexErr::InvalidRequest(format!(
        "failed to resolve base directory for user config `{}`",
        config_toml_path.display()
    ))
})?;
```

6. 将第一处 helper 调用从：

```rust
let user_config = resolve_relative_paths_in_config_toml(user_config, config_base_dir)
```

改为：

```rust
let user_config = resolve_relative_paths_in_config_toml(user_config, &config_base_dir)
```

7. 将第二处 helper 调用从：

```rust
let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, config_base_dir)
```

改为：

```rust
let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, &config_base_dir)
```

8. 不允许进一步改成以下任一方案：
  - 改 helper 签名去接受 `AbsolutePathBuf`
  - 把 `config_base_dir` 重新绑定成别的类型
  - 抽新 helper 包装借用
  - 回退 `config_base_dir` 与 relative path 相关逻辑
  - 顺手修改同文件其他无关 diff
  - 顺手做导入清理、注释补充、格式化或行尾变更
9. 修改完成后，必须生成 post-edit 快照并相对 pre-edit 基线做精确审计。
10. 本轮不允许以任何编译型手段验证，只允许静态核对与 diff 审计。

## Static Verification And Acceptance

- 本轮唯一允许的验证方式固定为：
  - 调用点源码核对
  - callee 签名核对
  - `AbsolutePathBuf::parent()` 返回类型核对
  - pre/post 基线对比
  - reviewer subagent 静态复核
- 静态验收标准固定为：
  - [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 的两处目标调用已经改为借用传参
  - [config_loader/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config_loader/mod.rs) 与 [config/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config/mod.rs) 的 `&Path` 契约未改
  - [absolute-path/src/lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/utils/absolute-path/src/lib.rs) 仍显示 `parent()` 返回 `AbsolutePathBuf`
  - 相对执行前当前工作树状态，`codex.rs` 新增差异仅包含两处 `&` 插入
  - 不存在新增导入变化、错误文案变化、无关逻辑变化、格式化变化、行尾污染
  - 任务回写中明确声明“未执行任何编译，release 状态未验证”
- 本轮明确的未验证项固定为：
  - `cargo build --release -p codex-cli --bin codex`
  - 任何 release / debug / check / test 级编译结果
- 结论口径只能写成：
  - `已完成静态最小修复，编译状态未验证`
- 禁止写成：
  - `build 已修复`
  - `release 已通过`
  - `阻塞已被编译验证消除`

## Implementation Writeback Requirements

- 执行完成后，必须回写本主文档，至少包含以下章节：
  - `实施摘要`
  - `静态验证证据`
  - `差异审计结果`
  - `未执行项/未验证项`
  - `subagent 审核结论`
  - `后续交接`
- `实施摘要` 必须写清：
  - 实际改动文件
  - 实际改动点数量
  - 是否严格保持两处 token 级修改
- `静态验证证据` 必须写清：
  - 目标调用点最终行文
  - callee 签名来源
  - `parent()` 返回类型来源
- `差异审计结果` 必须写清：
  - pre-edit 快照路径
  - post-edit 快照路径
  - 相对执行前状态的新增差异摘要
- `未执行项/未验证项` 必须明确写出：
  - 本轮未执行任何编译
  - release 构建状态仍未知
- `后续交接` 必须明确写出：
  - 如果后续用户允许编译，应以独立阶段重新执行 build 验证
  - 该 build 结果不得回填成“本轮已验证”

## Review Closure

- 本轮必须开启 reviewer subagent 做两轮静态复核。
- 第一轮 reviewer 审核对象固定为：
  - 当前重写后的任务文档
  - helper 契约来源
  - pre-edit 基线快照
- 第一轮 reviewer 审核目标固定为：
  - 任务边界是否足够严格
  - 静态验收口径是否与“禁止编译”一致
  - pre-edit 基线是否足够证明后续只改两处
- 第二轮 reviewer 审核对象固定为：
  - 已修改的 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)
  - post-edit 差异审计结果
  - 回写后的主文档与复核记录
- 第二轮 reviewer 审核目标固定为：
  - 是否真的只改了两处目标借用
  - 是否有无关改动混入
  - 回写章节是否完整
  - “未执行任何编译”声明是否明确
- 若 reviewer 任何一轮返回 findings，主 agent 必须先修正文档或代码，再继续同类 reviewer 复审。
- 若 `wait_agent` 只是超时，则继续等待同一个 reviewer；不得仅因超时而重开或降档。

## Non-Goals

- 不运行任何编译、测试、lint、fmt、codegen
- 不修改 helper 签名
- 不回退 provider runtime refresh 相对路径修复
- 不重构 `read_latest_provider_runtime_refresh(...)`
- 不新增测试或归档到其他长期功能清单
- 不处理 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 中与本次 `E0308` 无关的其他脏改
- 不处理当前工作区里的其他未提交改动

## Notes

- 本任务是“无编译静态最小修复”任务，不是 build 验证任务。
- 当前历史失败日志已经足够给出最小修复方向，但本轮不会重新证明编译结果。
- 本轮的主要质量门不再是 build，而是静态契约核对、pre/post diff 审计和两轮 reviewer subagent 复核。

## 用户/玩家视角直观变化清单

- 本次修改无用户/玩家可直接感知的直观变化。

## 实施摘要

- 本轮实际源码改动文件：
  - [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)
- 本轮实际源码改动点数量：
  - 2 处
- 本轮实际改动内容：
  - `resolve_relative_paths_in_config_toml(user_config, &config_base_dir)`
  - `deserialize_config_toml_with_base(merged_toml, &config_base_dir)`
- 本轮严格保持为两处 token 级修改：
  - 只新增了 2 个 `&`
  - 未改 helper 签名
  - 未改导入
  - 未改错误文案
  - 未改同文件其他 unrelated diff

## 静态验证证据

- 目标调用点当前源码位于 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)：
  - `core/src/codex.rs:4341` 已为 `resolve_relative_paths_in_config_toml(user_config, &config_base_dir)`
  - `core/src/codex.rs:4351` 已为 `deserialize_config_toml_with_base(merged_toml, &config_base_dir)`
- callee 签名静态证据：
  - [config_loader/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config_loader/mod.rs) 的 `resolve_relative_paths_in_config_toml(..., base_dir: &Path)`
  - [config/mod.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/config/mod.rs) 的 `deserialize_config_toml_with_base(..., config_base_dir: &Path)`
- caller 返回类型静态证据：
  - [absolute-path/src/lib.rs](/E:/vscodeProject/codex_github/codex/codex-rs/utils/absolute-path/src/lib.rs) 中 `AbsolutePathBuf::parent() -> Option<Self>`
- 改前基线快照：
  - 目标片段：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_pre_snippet.txt`
  - 整文件快照：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_pre_full.rs`
  - 整文件哈希：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_pre_hash.txt`
  - 执行前 git diff：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_pre_gitdiff.txt`
- 改后基线快照：
  - 目标片段：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_post_snippet.txt`
  - 整文件快照：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_post_full.rs`
  - 整文件哈希：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_post_hash.txt`
  - pre/post 审计：`E:\vscodeProject\codex_github\tmp\agent-snapshots\codex_rs_core_src_codex_rs_2026-04-06_static_fix_pre_post_audit.txt`

## 差异审计结果

- 执行前整文件 SHA-256：
  - `19686642DD0122CDC1C50B3086A9F5B408F10B1A5C1EA4EB19124BE08D3182B0`
- 执行后整文件 SHA-256：
  - `622FFE8A4CBEC180D9B0176C310682B93C1EF5F0ED57DCD848992D69E9B93DCC`
- 相对执行前当前工作树状态，pre/post 审计结果仅包含以下 4 行差异：
  - `=>        let user_config = resolve_relative_paths_in_config_toml(user_config, &config_base_dir)`
  - `=>        let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, &config_base_dir)`
  - `<=        let user_config = resolve_relative_paths_in_config_toml(user_config, config_base_dir)`
  - `<=        let cfg: ConfigToml = deserialize_config_toml_with_base(merged_toml, config_base_dir)`
- 审计结论：
  - 相对执行前状态，仅新增两处 `&`
  - 未发现额外文本差异
  - 由于 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs) 在执行前已含其他脏改，本轮完成判断不以相对 `HEAD` 的 `git diff` 为依据，而以 pre/post 快照对比为准

## 未执行项/未验证项

- 本轮未执行任何编译
- 本轮未执行任何 `cargo` 子命令
- 本轮未执行任何测试、lint、fmt、codegen
- `cargo build --release -p codex-cli --bin codex` 仍未重新验证
- 当前不能声称：
  - `build 已修复`
  - `release 已通过`
  - `阻塞已被编译验证消除`

## subagent 审核结论

- 第一轮 reviewer（改前复核）：
  - reviewer subagent：`019d62fd-f5cf-78e3-8cd1-6ec4bfc02935`
  - 结论：`no findings`
  - 审核对象：
    - 重写后的本任务文档
    - helper 契约来源
    - pre-edit 基线快照
- 第二轮 reviewer（改后复核）：
  - reviewer subagent：`019d62fd-f5cf-78e3-8cd1-6ec4bfc02935`
  - 结论：`no findings`
  - 审核对象：
    - 已修改的 [codex.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/codex.rs)
    - pre/post 差异审计快照
    - 回写后的主任务文档与复核记录
  - 复核结论：
    - 当前源码两处目标调用都已改为 `&config_base_dir`
    - `pre_full` 与 `post_full` 的整文件比较仅包含两处 `&` 插入对应的 4 行替换
    - 主任务文档已明确写出“本轮未执行任何编译”“release 构建状态仍未知”
    - 复核记录已准确记录改前 reviewer 结论、本轮源码修改和 pre/post 差异审计

## 后续交接

- 本轮完成口径固定为：
  - `已完成静态最小修复，编译状态未验证`
- 如果后续用户允许编译，应在独立阶段重新执行 build 验证。
- 后续 build 结果不得回填成“本轮已验证”；它只能作为下一阶段新增验证结果记录。
