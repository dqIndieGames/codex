<p align="center"><code>npm i -g @openai/codex</code><br />or <code>brew install --cask codex</code></p>
<p align="center"><strong>Codex CLI local1</strong> is a local1-maintained fork of OpenAI Codex CLI, synchronized with upstream <code>rust-v0.124.0</code> while preserving local1 workflow customizations.</p>
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## local1 Overview

This repository tracks the OpenAI Codex CLI codebase and carries a local1 customization layer. The current sync target is upstream release [`rust-v0.124.0`](https://github.com/openai/codex/releases/tag/rust-v0.124.0). This is not an official OpenAI release channel; the upstream project remains [`openai/codex`](https://github.com/openai/codex).

The local1 layer keeps these user-visible behaviors:

- Version surfaces show the local build identity as `<upstream version>-local1` across CLI help, TUI status surfaces, history cells, update prompts, and related snapshots.
- The `/responses` main request path keeps broader retry handling for transient HTTP failures, including local1 handling around `401` recovery and retry-chain log suppression.
- Provider runtime refresh keeps the current provider sticky while refreshing only `base_url` and `experimental_bearer_token`; Windows tray provider copy support remains connected to the same refresh path.
- Resume history discovery defaults to cross-provider visibility, while fork flows keep the current provider filter where that matters.
- `gpt-5.4` keeps the local1 priority fallback by default; setting `force_gpt54_priority_fallback = false` disables both the priority fallback and Fast passthrough for that model.
- Windows app-server and TUI startup logging stay quieter by default when `RUST_LOG` is not explicitly set.
- A brand-new or cleared regular thread whose first plain text input is exactly `你好` still injects the local1 checklist into the first visible assistant response; resumed, forked, subagent, reviewer, guardian, and non-matching first turns do not trigger it.

## local1 中文介绍

本仓库是 OpenAI Codex CLI 的 local1 维护分支，当前已同步到上游 [`rust-v0.124.0`](https://github.com/openai/codex/releases/tag/rust-v0.124.0)，同时继续保留 local1 定制功能。本仓库不是 OpenAI 官方发布渠道，官方上游仍是 [`openai/codex`](https://github.com/openai/codex)。

local1 分支保留的主要用户可见能力如下：

- CLI 帮助、TUI 状态区、历史单元、升级提示等版本展示继续显示 `<上游版本>-local1`，让使用者能直接看出当前是 local1 构建。
- `/responses` 主请求链继续保留更宽的临时失败重试能力，包括 local1 对 `401` 恢复与重试链日志降噪的处理。
- Provider runtime refresh 继续只刷新当前 provider 的 `base_url` 与 `experimental_bearer_token`，并保留 Windows tray 从配置复制 provider 字段后触发 refresh 的联动。
- Resume 历史列表默认可跨 provider 发现旧会话；Fork 场景仍保留当前 provider 过滤，避免把新分支接到错误来源上。
- `gpt-5.4` 默认继续走 local1 的 priority 兜底；顶层配置 `force_gpt54_priority_fallback = false` 会关闭该模型的 priority 兜底和 Fast 透传。
- 未显式设置 `RUST_LOG` 时，Windows app-server 与 TUI 默认保持日志降噪，减少普通使用时看到的后台噪声。
- brand-new 或 Clear 后的新普通线程，只有首个纯文本输入精确为 `你好` 时，才会在第一条可见 assistant 回复里注入 local1 清单；resume、fork、subagent、reviewer、guardian 以及不匹配的首轮输入都不会触发。

---

## Quickstart

### Installing and running Codex CLI

Install globally with your preferred package manager:

```shell
# Install using npm
npm install -g @openai/codex
```

```shell
# Install using Homebrew
brew install --cask codex
```

Then simply run `codex` to get started.

<details>
<summary>You can also review this fork's <a href="https://github.com/dqIndieGames/codex/releases/latest">local1 GitHub Release</a> for the current source release notes. Use the upstream OpenAI Releases page for official prebuilt binaries unless this fork's release page explicitly publishes GitHub-built assets.</summary>

The official OpenAI GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `codex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `codex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `codex-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `codex-x86_64-unknown-linux-musl`), so you likely want to rename it to `codex` after extracting it.

</details>

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
