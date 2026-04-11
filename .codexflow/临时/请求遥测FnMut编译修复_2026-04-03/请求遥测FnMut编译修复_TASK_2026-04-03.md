# 请求遥测FnMut编译修复_TASK_2026-04-03

## Summary

- 项目根目录解析为 `E:\vscodeProject\codex_github\codex`。
- 本文定义的是一次以 release 构建链为验收门禁的编译回归修复任务，不定义新的用户可见功能。
- 当前任务已进入两阶段口径：
  - cycle1 已修复 [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs) 中 `run_with_request_telemetry(...)` 的 `FnMut` 绑定缺少 `mut`
  - build1 已证明该问题不再出现，并继续暴露出 cycle2 的当前唯一 blocker
- cycle2 当前唯一 blocker 位于 [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs)：
  - `stream_responses_api(...)` 原 receiver 为 `&self`
  - 函数体中存在 `std::mem::take(&mut self.pending_unauthorized_retry)`
  - build1 报错为 `cannot borrow data in a '&' reference as mutable`
- 当前允许的最小闭环修复固定为：
  - 保持 cycle1 的 telemetry 修复不回退
  - 在 cycle2 中只允许补 [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 这一处 `stream_responses_api(&mut self)` receiver mutability 修复
  - 保留现有 retry 语义、helper 契约和调用链结构
  - 不扩展到 retry 重构、接口收窄或长期文档归档

## Context

- 旧实现中，`make_request` 的实际调用发生在 [retry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-client/src/retry.rs) 的 `run_with_retry(...)` 内部；该 helper 自身声明的是 `mut make_req: impl FnMut() -> Request`，因此旧实现不会在 telemetry 包装层触发 `FnMut` 可变借用报错。
- 新实现把请求重试循环内联到了 [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs) 的 `run_with_request_telemetry(...)` 中，导致 `make_request` 的直接调用点迁移到了 telemetry helper 当前作用域。
- Rust 在当前作用域直接调用 `FnMut` 闭包时，需要对闭包值做可变借用，因此当前参数绑定本身必须声明为 `mut`。
- 当前 [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs) 中的 `execute_with(...)` 与 `stream_with(...)` 两条入口虽然都传入了实际上可满足 `Fn` 的 closure，但这不能反推 telemetry helper 应把契约从 `FnMut` 收窄为 `Fn`。

## Goal

- 在保持 cycle1 的 telemetry `FnMut` 修复不回退的前提下，用最小代码修改恢复 `cargo build --release -p codex-cli --bin codex` 这条 release 构建链在当前 cycle2 问题点上的可编译性；当前 cycle2 仅允许补 `stream_responses_api(&mut self)` 这一处 receiver mutability 修复，且不引入额外行为变化。

## Root Cause

- 根因必须写死为：
  - `make_request` 的调用位置已经从 retry helper 内部迁移到 telemetry helper 当前作用域。
  - 在该作用域中直接调用 `impl FnMut() -> Request` 时，参数绑定本身必须是 `mut`。
- 不允许把根因写成以下错误表述：
  - “因为重试循环会多次调用 closure，所以需要 `mut`”
  - “因为当前 call site 实际是 `Fn`，所以把签名改成 `Fn` 就行”
- 该问题的本质是 Rust 的 `FnMut` 绑定语义，而不是 retry 次数问题，也不是业务逻辑问题。

## Modification Boundary

- 初始唯一预期代码改动文件固定为 [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs)。
- 第一次 release build 之后，已新增一个经构建暴露、且仍属最小 mutability 回归的例外修复点：
  - [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 中 `stream_responses_api(...)` 的 receiver 从 `&self` 改为 `&mut self`
- 除以上 2 处外，其他代码文件仍不在本任务计划改动范围内。
- 以下文件只用于契约核对、影响链说明和验收，不作为计划改动文件：
  - [retry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-client/src/retry.rs)
  - [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs)
- 影响链必须明确覆盖：
  - `execute_with(...)`
  - `stream_with(...)`
- `telemetry.rs` 修复对应的 build1 已经执行并向前暴露出下一个 blocker；当前本任务统一按 cycle2 口径继续收口：
  - [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 的 receiver mutability 修复，是 build1 后本任务唯一允许的跟进代码修改
  - `execute_with(...)` / `stream_with(...)` call site 仍预计不需要修改
  - 除 `telemetry.rs` 与 `client.rs` 这两处外，不允许继续扩散到其他代码文件

## Detailed Code Modification Checklist

1. 在 [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs) 中定位 `run_with_request_telemetry<T, F, Fut>(...)` 的参数列表。
2. 将参数绑定从：

```rust
make_request: impl FnMut() -> Request,
```

改为：

```rust
mut make_request: impl FnMut() -> Request,
```

3. 保持 `make_request` 的类型约束不变，继续为 `impl FnMut() -> Request`。
4. 不允许把签名改成：
  - `impl Fn() -> Request`
  - `impl FnOnce() -> Request`
5. 不允许改动当前 retry loop 的以下逻辑：
  - `for attempt in 0..=policy.max_attempts`
  - `let req = make_request();`
  - `let req_for_retry = req.clone();`
  - `should_retry_request_error(&policy, &req_for_retry, err, attempt)`
  - `t.on_request(...)`
  - `t.on_request_retry(...)`
  - `sleep(backoff(policy.base_delay, attempt + 1)).await`
  - `Err(TransportError::RetryLimit)`
6. 不修改 [retry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-client/src/retry.rs) 的 `run_with_retry(...)` 签名或实现。
7. 不修改 [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs) 中 `execute_with(...)` / `stream_with(...)` 的 closure 构造方式。
8. 不新增注释、日志、测试桩、helper 抽取或其他“顺手清理”改动，除非编译器在同一问题点继续要求最小配套修复。
9. 若第一次 release build 暴露出 [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 中 `stream_responses_api(...)` 的 `&self` / `&mut self` mutability 回归，则允许唯一一处最小跟进修改：

```rust
async fn stream_responses_api(
    &mut self,
    ...
)
```

10. 除上述 `client.rs` receiver mutability 修复外，不允许继续在 `core/src/client.rs` 扩展到其他逻辑改造。

## Verification And Acceptance

- 验收命令优先固定为在 `E:\vscodeProject\codex_github\codex\codex-rs` 下执行：

```powershell
cargo build --release -p codex-cli --bin codex
```

- 不允许用以下方式替代本次回归验收：
  - `cargo check`
  - debug profile 构建
  - 与原失败 profile 不一致的轻量检查
  - 无关 package 的编译
- 验收标准固定为：
  - cycle1 的 telemetry `E0596 cannot borrow make_request as mutable` 不回归
  - cycle2 的 [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) `cannot borrow data in a '&' reference as mutable` 报错消失
  - [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs) 仍保持 `FnMut` 契约，只新增 `mut make_request`
  - [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 仅补 `stream_responses_api(&mut self)`，不扩展到其他逻辑改动
  - [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs) 的 `execute_with(...)` / `stream_with(...)` 调用方式不变
  - [retry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-client/src/retry.rs) 不变
  - 若 release 构建在本问题之后仍暴露新的、与本任务无关的错误，应将其记录为后续独立问题，而不是在本任务中继续扩展改动范围

## Non-Goals

- 不将 `FnMut` 收窄为 `Fn`
- 不将 `FnMut` 改成 `FnOnce`
- 不重构 `run_with_retry(...)`
- 不重构 telemetry retry loop
- 不重构 `stream_responses_api(...)` 逻辑
- 不改 provider 重试分类
- 不改 backoff、sleep、telemetry hook、retry budget 或终态语义
- 不改写 [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs) call site
- 不新增或修改 [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md)

## Review Closure

- 本文必须经过两轮 subagent 审核：
  - 第一轮：以找问题为主的批判审核
  - 第二轮：基于第一轮 findings 做“采纳 / 不采纳 / 降级为备注”裁决，并把采纳项并回主文档
- 两轮审核完成后，主文档必须直接整合采纳结论，不能把关键约束只留在审核文档里。
- 若某条 finding 被降级为备注或不采纳，必须在批判审核文档中写明原因。

## Notes

- 本任务是 release 编译回归修复任务，不是长期功能基线变更任务。
- 默认不归档到 `local1` 清单文档；原因不是单纯“本次修改不可见”，而是本次修复仅补 telemetry helper 的 `FnMut` 绑定，`local1` 清单中的 F5（请求重试范围增强）与 F7（单次重试等待时间上限为 `10s`）现有断言逐条保持为真，`local1` 基线语义未发生变化。只有当修复方案被迫改变这些长期真值或用户可见行为时，才重新评估是否需要单独归档。

## 用户/玩家视角直观变化清单

- 本次修改无用户/玩家可直接感知的直观变化。

## 实施回写

- cycle1 已实施最小代码修复：
  - 改动文件：[telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs)
  - 改动内容：将 `run_with_request_telemetry(...)` 的参数绑定改为 `mut make_request: impl FnMut() -> Request`
- 第一次 release build 验证结果：
  - 已执行一次 `cargo build --release -p codex-cli --bin codex`
  - 已知 `telemetry.rs` 的 `FnMut` 绑定报错不再出现
  - 构建继续前进后暴露出新的独立编译错误：
    - 文件：[client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs)
    - 位置：`core/src/client.rs:1119`
    - 错误：`cannot borrow data in a '&' reference as mutable`
    - 直接原因：`stream_responses_api(...)` 仍为 `&self`，但函数体内新增了 `std::mem::take(&mut self.pending_unauthorized_retry)`
- 针对上述新错误，当前已开启同一套“修复 -> 两轮 subagent 审核 -> 再 build”闭环：
  - 新增代码修复：[client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 中 `stream_responses_api(...)` 的 receiver 已改为 `&mut self`
  - 该新修复对应的两轮 subagent 审核与裁决回写现已完成
  - 第二次 release build 已执行并成功
- 本任务新增改动 vs 当前仓库已有脏改动：

| 路径 | 当前 git 状态 | 是否本任务引入 | 证据来源 | 是否影响本 TASK 边界判定 |
|---|---|---|---|---|
| [telemetry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/telemetry.rs) | `M` | 是 | 本轮 apply_patch 与逐文件 diff | 是；这是本任务唯一代码回写，且只补 `mut make_request` |
| [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) | `M` | 是 | 第一次 release build 报错定位 + 本轮 apply_patch 与逐文件 diff | 是；这是第一次 release build 暴露出的新独立 mutability 回归，当前只补 `&mut self` |
| [retry.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-client/src/retry.rs) | 未因本任务新增变化 | 否 | 主 TASK 边界约束与逐文件核对 | 是；仅作契约核对，不计入本任务写入 |
| [session.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/endpoint/session.rs) | 未因本任务新增变化 | 否 | 主 TASK 边界约束与逐文件核对 | 是；仅作调用链核对，不计入本任务写入 |
| [provider.rs](/E:/vscodeProject/codex_github/codex/codex-rs/codex-api/src/provider.rs) | `M` | 否 | `git status --short` | 是；当前工作树已有脏改动，不计入本任务 |
| [local1-custom-feature-checklist-2026-03-28.md](/E:/vscodeProject/codex_github/codex/docs/local1-custom-feature-checklist-2026-03-28.md) | `M` | 否 | `git status --short` | 是；当前工作树已有脏改动，不计入本任务，也不触发本轮归档 |
- 第二次 `cargo build --release -p codex-cli --bin codex` 最终验收结果：
  - 第一次执行 build2 时，命令等待在工具侧超时，但后台 `cargo` / `rustc` 进程继续运行，未出现即时编译报错。
  - 随后确认 `target\release\codex.exe` 已刷新为 `2026-04-03 23:20:54`，文件大小为 `174584832` 字节。
  - 为取得明确退出状态，再次执行同一条命令，增量构建以 `exit code 0` 返回，并输出 `Finished release profile [optimized] target(s) in 4.81s`。
  - 本次最终验收未再出现：
    - `telemetry.rs` 的 `cannot borrow make_request as mutable`
    - [client.rs](/E:/vscodeProject/codex_github/codex/codex-rs/core/src/client.rs) 的 `cannot borrow data in a '&' reference as mutable`
  - 构建过程中仅出现 `codex-app-server` 的 unused import / unused mut / dead_code warnings；这些告警不属于本任务 blocker，也未改变本任务最小修复边界。
