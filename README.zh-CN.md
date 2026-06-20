# Codex Relay Edition / Codex 中转站魔改版

这是一个面向中转站使用场景的 Codex CLI 修改版，核心体验是：请求失败后自动持续重试，不用反复手动点击继续；遇到 relay provider、sub2api 网关、代理池或账号池临时抽风时，可以动态无感切换 provider，让 Codex 尽量自己绕开卡住的线路。

对经常使用 sub2api 等中转站的人来说，最烦的不是偶发 `503`，而是请求一直粘在已经出错的账号、路由或客户端指纹上，后面怎么点继续都继续失败。这个分支围绕这类场景做了自动重试、运行时刷新和粘连指纹切换，目标是尽量减少“无限 503 后只能手动处理”的情况，让 Codex 可以更长时间自己跑下去。

它基于 OpenAI Codex CLI 做适配，但不是 OpenAI 官方发布版。如果你只需要官方原版 Codex CLI，请以 [`openai/codex`](https://github.com/openai/codex) 为准。

## 主要改动

- 更适合 relay provider / sub2api 使用：可以把 Codex 指向自己的 `base_url`、bearer token、账号池或代理路由。
- 对临时上游错误更宽容：遇到 `429`、`503`、`server_is_overloaded`、`slow_down`、`select model` 满载等问题时，更偏向继续重试和展示当前状态，而不是很快中断。
- 连续失败后可以切换请求指纹或路由特征：减少同一账号、同一路由或同一客户端特征持续卡在同一个错误上的概率。
- 保留 local2 相关显示：CLI、TUI、版本号、历史会话和部分运行时行为会继续体现这是本地修改版，方便和官方原版区分。
- 提供 Codex Provider Refresh：在不关闭 `codex.exe` 的情况下，切换当前 provider 的 `base_url` 和 token。

## Codex Provider Refresh

Provider Refresh 是这个分支给 Windows 使用场景准备的小工具。它的作用很直接：当你已经开着 Codex，但想把当前 provider 的 `base_url` 或 token 换成另一个中转站配置时，不需要先关掉所有 `codex.exe` 再重开。

工具路径：

```text
scripts/windows_app_server_refresh_tray.py
```

<p align="center">
  <img src=".github/codex-provider-refresh-gui.png" alt="Codex Provider Refresh 真实 Python GUI 截图" width="96%" />
</p>

上图是从真实 Python Tk GUI 窗口截取的截图。截图使用临时演示配置，不读取用户真实配置；图里的 provider、base_url 和 token 都是演示值，token 已遮罩。

## 使用方式

1. 按你的中转站要求，在本地 Codex 配置里写好 provider、`base_url` 和 token。
2. 正常启动这个修改版 Codex。
3. 如果中转站临时满载、账号池需要切换、或者 token / 路由要调整，可以使用 Provider Refresh 工具刷新当前 provider。
4. 如果只是查看官方 Codex 的基础用法、命令说明或贡献流程，请继续参考官方文档和上游仓库。

## 和官方版的关系

这个仓库保留官方 Codex CLI 的基础能力，但默认展示重点放在中转站使用体验上。它适合需要 relay provider、sub2api、代理池、账号池、运行时刷新和更宽容重试策略的人；不适合想要完全无修改官方版本的人。

官方入口：

- 官方文档：<https://developers.openai.com/codex>
- 官方上游：<https://github.com/openai/codex>

## 注意事项

- 这是 fork / modified build，不是 OpenAI 官方发布渠道。
- README 截图只展示演示 provider 和遮罩 token，不展示真实 token、cookie、session 或账号信息。
- README 只负责介绍仓库定位和使用入口，不代表每次展示修改都会新增 runtime 逻辑。
- 本仓库继续遵循 [Apache-2.0 License](LICENSE)。
