# Codex CLI local1 中文介绍

本仓库是 OpenAI Codex CLI 的 local1 维护分支，当前已同步到上游 `rust-v0.124.0`，同时继续保留 local1 定制功能。本仓库不是 OpenAI 官方发布渠道，官方上游仍是 `openai/codex`。

## 当前同步目标

- 上游版本：`rust-v0.124.0`
- local1 清单：`docs/local1-custom-feature-checklist-2026-03-28.md`
- local1 Release 说明：`docs/local1-release-rust-v0.124.0-2026-04-24.md`

## local1 保留能力

- CLI 帮助、TUI 状态区、历史单元、升级提示等版本展示继续显示 `<上游版本>-local1`，让使用者能直接看出当前是 local1 构建。
- `/responses` 主请求链继续保留更宽的临时失败重试能力，包括 local1 对 `401` 恢复与重试链日志降噪的处理。
- Provider runtime refresh 继续只刷新当前 provider 的 `base_url` 与 `experimental_bearer_token`，并保留 Windows tray 从配置复制 provider 字段后触发 refresh 的联动。
- Resume 历史列表默认可跨 provider 发现旧会话；Fork 场景仍保留当前 provider 过滤，避免把新分支接到错误来源上。
- 顶层 `force_service_tier_priority` hook 默认开启；开启后所有 `/responses` 请求都会在最底层请求构造处强制发送 `service_tier = "priority"`，不再区分原本配置的是普通、fast、flex 还是具体模型。显式设为 `false` 后恢复上游原始映射：`Fast -> priority`、`Flex -> flex`、未设置则继续不发送。
- 未显式设置 `RUST_LOG` 时，Windows app-server 与 TUI 默认保持日志降噪，减少日志输出、SQLite/文件写入占用和后台 I/O，让普通启动与交互更轻、更快。
- brand-new 或 Clear 后的新普通线程，只有首个纯文本输入精确为 `你好` 时，才会在第一条可见 assistant 回复里注入 local1 清单；resume、fork、subagent、reviewer、guardian 以及不匹配的首轮输入都不会触发。

## 发布说明

本次发布通过 GitHub 仓库与 GitHub Release 记录完成，不使用本机系统打包二进制产物。远端编译与检查以 GitHub Actions 结果为准。

## GitHub 编译 Windows release exe

本仓库的 Windows release exe 以 GitHub Actions 产物为准，本地不需要、也不建议为了发布交付执行编译。这里的 release exe 指发布类型的 Windows 可执行文件，不是固定文件名；实际 x64 文件名为 `codex-x86_64-pc-windows-msvc.exe`。GitHub Actions 是 GitHub 托管的自动化流水线；对使用者来说，它会在云端 Windows runner 上编译并上传 exe，避免把本机环境差异带进发布产物。

从当前 `main` 分支触发 x64 Windows release exe 编译：

```shell
gh workflow run rust-release-windows.yml --repo dqIndieGames/codex --ref main -f release-lto=fat -f target=x86_64-pc-windows-msvc
```

工作流成功完成后，下载 x64 Windows 产物。请选择 `status` 已完成且 `conclusion` 为成功的 `run-id`，否则可能下载不到产物或拿到错误构建：

```shell
gh run list --repo dqIndieGames/codex --workflow rust-release-windows.yml --branch main --event workflow_dispatch --status success --limit 1
gh run download <run-id> --repo dqIndieGames/codex -n x86_64-pc-windows-msvc -D dist/windows-x64
```

下载目录会包含 `codex-x86_64-pc-windows-msvc.exe` 和对应压缩包。工作流在上传前会对暂存的 `codex.exe` 执行冒烟测试：运行 `--version` 和 `--help`，并断言版本输出包含 `0.124.0-local1`。冒烟测试的意思是先确认 exe 能启动、能输出版本和帮助；它不等于完整业务回归。

在 Windows 本机下载后，也可以只运行 exe 做冒烟验证，不涉及本地编译：

```powershell
.\dist\windows-x64\codex-x86_64-pc-windows-msvc.exe --version
.\dist\windows-x64\codex-x86_64-pc-windows-msvc.exe --help
```

预期版本输出应包含 `0.124.0-local1`。
