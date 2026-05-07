<p align="center"><code>npm i -g @openai/codex</code><br />or <code>brew install --cask codex</code></p>
<p align="center"><strong>Codex CLI local2</strong> is a local2-maintained fork of OpenAI Codex CLI, carrying local2 workflow customizations on top of the current <code>0.128.0</code> code line.</p>
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## local2 Overview

This repository tracks the OpenAI Codex CLI codebase and carries a local2 customization layer. The current package version in this repo is `0.128.0`, and user-visible local builds append the `-local2` suffix. This is not an official OpenAI release channel; the upstream project remains [`openai/codex`](https://github.com/openai/codex).

The local2 layer keeps these user-visible behaviors:

- Version surfaces show the local build identity as `<package version>-local2` across CLI help, TUI status surfaces, history cells, update prompts, and related snapshots. The CLI `--version` path also has a direct regression test guard.
- The `/responses` main request path keeps broader retry handling for transient HTTP failures, including direct retry for `401` and other remote HTTP failures, retry-chain log suppression, and retention of retry-visible status details.
- Provider runtime refresh keeps the current provider sticky while refreshing only `base_url` and `experimental_bearer_token`; Windows tray provider copy support remains connected to the same refresh path.
- Resume history discovery defaults to cross-provider visibility, while fork flows keep the current provider filter where that matters.
- The top-level `force_service_tier_priority` hook defaults to `true`; when enabled, every `/responses` request is forced to send `service_tier = "priority"` at the lowest request-construction layer, regardless of the configured `service_tier` value or model. Setting it to `false` restores the upstream mapping: `Fast -> priority`, `Flex -> flex`, and unset remains unset.
- Windows app/app-server and TUI startup logging stay quiet by default when `RUST_LOG` is not explicitly set, reducing log output, SQLite/file write overhead, and background I/O so normal startup and interactive use feel lighter and faster.
- A brand-new or cleared regular thread whose first plain text input is exactly the documented Chinese greeting `U+4F60 U+597D` still injects the local2 checklist into the first visible assistant response; resumed, forked, subagent, reviewer, guardian, and non-matching first turns do not trigger it.
- Runtime-load reductions stay opt-in: rollout batch flush, app-server high-frequency notification coalescing, and analytics / feedback / `log_db` load all default to off unless explicitly enabled in `config.toml`.

The long-term local2 preservation checklist lives in [docs/local2-custom-feature-checklist-2026-04-27.md](docs/local2-custom-feature-checklist-2026-04-27.md).

For the Chinese introduction, see [README.zh-CN.md](README.zh-CN.md).

---

## GitHub-Built Windows Release Exe

This fork is maintained with GitHub-built Windows release artifacts. Local compilation is intentionally not required for the local2 Windows handoff.

To build the minimal x64 Windows release package from the current `main` branch with GitHub Actions:

```shell
gh workflow run local2-minimal-windows-release.yml --repo dqIndieGames/codex --ref main -f tag=local2-windows-minimal-<date>-<sha> -f target=x86_64-pc-windows-msvc -f release_name="local2 Windows minimal <date> (<sha>)"
```

After the run completes successfully, download the Release asset or the tagged prerelease entry. Use a `run-id` whose `status` is completed and whose `conclusion` is success:

```shell
gh run list --repo dqIndieGames/codex --workflow local2-minimal-windows-release.yml --branch main --event workflow_dispatch --status success --limit 1
gh release download <tag> --repo dqIndieGames/codex -p codex-x86_64-pc-windows-msvc-minimal.zip -D dist/windows-x64
```

The downloaded zip contains exactly one runtime payload: `codex.exe`. The workflow builds `codex.exe`, packages only that file into `codex-x86_64-pc-windows-msvc-minimal.zip`, and publishes it as a GitHub prerelease asset.

The workflow is a packaging pipeline, not a full local2 business regression. In this fork, the broader local2 regression pass is expected to happen against the downloaded exe and the checklist in [docs/local2-custom-feature-checklist-2026-04-27.md](docs/local2-custom-feature-checklist-2026-04-27.md).

You can also smoke-test the downloaded exe on Windows after extracting the zip:

```powershell
Expand-Archive .\dist\windows-x64\codex-x86_64-pc-windows-msvc-minimal.zip -DestinationPath .\dist\windows-x64\minimal
.\dist\windows-x64\minimal\codex.exe --version
.\dist\windows-x64\minimal\codex.exe --help
```

Expected version output includes `<current package version>-local2`; for the current repo state, that means `0.128.0-local2`.

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
<summary>You can also review this fork's <a href="https://github.com/dqIndieGames/codex/releases/latest">local2 GitHub Release</a> for the current source release notes and GitHub-built minimal Windows assets. Use the upstream OpenAI Releases page for official prebuilt binaries unless this fork's release page explicitly publishes the artifact you need.</summary>

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
