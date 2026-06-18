# Codex Relay Edition / Codex 中转站魔改版

Codex Relay Edition is a relay-focused modified build of OpenAI Codex CLI for users who run Codex through relay providers, sub2api gateways, and proxy pools. It is a forked/modified build for relay operations, not an official OpenAI release channel.

**What users see first:** this repository is positioned for relay provider operation, persistent recovery from overloaded upstreams, and quick provider switching while `codex.exe` keeps running.

## Relay-Focused Highlights

- Relay provider and sub2api first: point Codex at your own `base_url`, bearer token, account pool, or proxy route.
- Continuous retry display path for transient upstream failures, including `429`, `503`, `203`, `server_is_overloaded`, `slow_down`, and `select model` capacity errors.
- Route recovery after repeated failures: after 3 consecutive failures, the client-facing route fingerprint can rotate so one sticky route does not keep blocking the session.
- Codex Provider Refresh: switch the active `base_url` and token without closing `codex.exe`.
- Real Provider Refresh screenshot: the README image is captured from the actual Python Tk GUI with a temporary demo `CODEX_HOME` and masked tokens.

## Codex Provider Refresh

Provider means the API supplier or relay entry Codex sends requests to. For users, this decides which relay service, account, token, and route Codex is using.

Base URL means the API server address. For users, changing it moves Codex to another relay endpoint.

Token means the credential used by the relay or API service. This README only shows demo or masked values.

Provider Refresh tool path:

```text
E:\vscodeProject\codex_github\codex\scripts\windows_app_server_refresh_tray.py
```

<p align="center">
  <img src=".github/codex-provider-refresh-gui.png" alt="Real Codex Provider Refresh Python GUI screenshot" width="96%" />
</p>

The screenshot above is a real window from `scripts/windows_app_server_refresh_tray.py`, captured with a temporary demo `CODEX_HOME` so the supported relay-provider state can be shown without reading the user's real config. It uses values such as `sub2api_relay`, `https://relay.example/v1`, and masked tokens. Do not place real tokens, cookies, sessions, or account credentials in README screenshots.

## Quickstart

Run the Codex binary from this modified build and configure your relay provider in the local Codex config. Keep the official upstream documentation nearby for baseline CLI usage, then use this repository's README and local2 notes for relay-specific behavior.

Useful local entry points:

- Relay/provider display overview: this README.
- Chinese overview: [`README.zh-CN.md`](./README.zh-CN.md)
- Provider Refresh tray helper: `scripts/windows_app_server_refresh_tray.py`
- local2 retained behavior notes: `docs/local2-custom-feature-checklist-2026-04-27.md`

## Safety Notes

- This is a modified fork/build, not the official OpenAI Codex project.
- Screenshots should show only demo providers and masked tokens.
- README display changes do not implement new runtime logic by themselves.
- This repository remains under the [Apache-2.0 License](LICENSE).

## Upstream And Docs

- Official Codex documentation: <https://developers.openai.com/codex>
- Official upstream project: <https://github.com/openai/codex>
- Contributing guide: [`docs/contributing.md`](./docs/contributing.md)
- Installing and building notes: [`docs/install.md`](./docs/install.md)
