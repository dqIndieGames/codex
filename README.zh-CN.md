# Codex CLI local2 中文介绍

本仓库是 OpenAI Codex CLI 的 local2 维护分支，当前代码线的包版本为 `0.128.0`，同时继续保留 local2 定制功能。本仓库不是 OpenAI 官方发布渠道，官方上游仍是 `openai/codex`。

## 当前同步目标

- 当前包版本：`0.128.0`
- local2 清单：`docs/local2-custom-feature-checklist-2026-04-27.md`

## local2 保留能力

- CLI 帮助、TUI 状态区、历史单元、升级提示等版本展示继续显示 `<包版本>-local2`，让使用者能直接看出当前是 local2 构建；CLI `--version` 也有直接回归保护。
- `/responses` 主请求链继续保留更宽的临时失败重试能力，包括 `401` 与其他远端 HTTP 错误直接进入普通 retry、重试链日志降噪，以及保留可见重试提示。
- Provider runtime refresh 继续只刷新当前 provider 的 `base_url` 与 `experimental_bearer_token`，并保留 Windows tray 从配置复制 provider 字段后触发 refresh 的联动。
- Resume 历史列表默认可跨 provider 发现旧会话；Fork 场景仍保留当前 provider 过滤，避免把新分支接到错误来源上。
- 顶层 `force_service_tier_priority` hook 默认开启；开启后所有 `/responses` 请求都会在最底层请求构造处强制发送 `service_tier = "priority"`，不再区分原本配置的是普通、fast、flex 还是具体模型。显式设为 `false` 后恢复上游原始映射：`Fast -> priority`、`Flex -> flex`、未设置则继续不发送。
- 未显式设置 `RUST_LOG` 时，Windows app-server 与 TUI 默认保持日志降噪，减少日志输出、SQLite/文件写入占用和后台 I/O，让普通启动与交互更轻、更快。
- brand-new 或 Clear 后的新普通线程，只有首个纯文本输入精确为 `你好` 时，才会在第一条可见 assistant 回复里注入 local2 清单；resume、fork、subagent、reviewer、guardian 以及不匹配的首轮输入都不会触发。
- rollout 批量 flush、app-server 高频通知合并、analytics / feedback / `log_db` 这些 runtime 负担优化默认关闭，只有在 `config.toml` 显式开启时才生效。

## 发布说明

本次发布通过 GitHub 仓库与 GitHub Release 记录完成，不使用本机系统打包二进制产物。远端编译与检查以 GitHub Actions 结果为准。

## GitHub 远端打包最小 Windows release

本仓库的 Windows release 以 GitHub Actions 产物为准，本地不需要、也不建议为了发布交付执行编译。这里的“最小 Windows release”指 GitHub 在云端 Windows runner 上编译并发布一个只包含 `codex.exe` 的 zip，避免把本机环境差异带进最终交付。

从当前 `main` 分支触发最小 x64 Windows release：

```shell
gh workflow run local2-minimal-windows-release.yml --repo dqIndieGames/codex --ref main -f tag=local2-windows-minimal-<日期>-<sha> -f target=x86_64-pc-windows-msvc -f release_name="local2 Windows minimal <日期> (<sha>)"
```

工作流成功完成后，下载 Release 资产。请选择 `status` 已完成且 `conclusion` 为成功的 `run-id`，否则可能下载不到产物或拿到错误构建：

```shell
gh run list --repo dqIndieGames/codex --workflow local2-minimal-windows-release.yml --branch main --event workflow_dispatch --status success --limit 1
gh release download <tag> --repo dqIndieGames/codex -p codex-x86_64-pc-windows-msvc-minimal.zip -D dist/windows-x64
```

下载得到的 zip 名称为 `codex-x86_64-pc-windows-msvc-minimal.zip`，zip 内只包含一个运行时文件：`codex.exe`。这个工作流本身是打包流水线，不等于完整 local2 业务回归；完整的 local2 行为验收应结合下载后的 exe 与 `docs/local2-custom-feature-checklist-2026-04-27.md` 一起执行。

在 Windows 本机下载后，也可以只运行 exe 做冒烟验证，不涉及本地编译：

```powershell
Expand-Archive .\dist\windows-x64\codex-x86_64-pc-windows-msvc-minimal.zip -DestinationPath .\dist\windows-x64\minimal
.\dist\windows-x64\minimal\codex.exe --version
.\dist\windows-x64\minimal\codex.exe --help
```

当前仓库状态下，预期版本输出应包含 `0.128.0-local2`。
