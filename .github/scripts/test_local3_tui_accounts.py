"""清单 5.3：真实 TUI 默认／alpha 的 HTTP、WS 请求一致性，L1 关系断言。

复用隐藏桌面与本机服务；只使用本轮 exe，不强制修改 TUI 后台选择。
动态身份作一一映射，保留字段及重复引用关系，不整块忽略 metadata。
"""
import base64
import json
from pathlib import Path
import re
import sys
import threading
import time

from test_local3_runtime_refresh import RuntimeFixture
from test_websocket_recovery import command
from _local3_hidden_console import HiddenConsole
from verify_local3_hidden_tui import K, windows, is_user_request


def normalized(entry, credential):
    ids = {}
    uuid_pattern = re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', re.I)
    def uuid_value(match):
        value = match.group().lower()
        return ids.setdefault(value, f'<identity-{len(ids)}>')
    def visit(value, key=''):
        if key in ('turn_started_at_unix_ms', 'x-codex-ws-stream-request-start-ms'):
            assert int(value) > 0
            return '<request-time>'
        if isinstance(value, dict):
            return {k: visit(v, k) for k, v in sorted(value.items())}
        if isinstance(value, list):
            return [visit(v) for v in value]
        if isinstance(value, str):
            if key == 'x-codex-turn-metadata':
                return visit(json.loads(value))
            return uuid_pattern.sub(uuid_value, value)
        return value
    headers = {k.lower(): v for k, v in entry['headers'].items()}
    assert headers['authorization'] == 'Bearer ' + credential, 'selected account credential'
    headers['authorization'] = '<selected-account>'
    if 'sec-websocket-key' in headers:
        assert base64.b64decode(headers['sec-websocket-key'], validate=True)
        headers['sec-websocket-key'] = '<handshake-nonce>'
    assert '-local3' not in headers['user-agent'], 'bare server version'
    return visit({'method': entry['method'], 'path': entry['path'],
                  'transport': entry['transport'], 'headers': headers, 'payload': entry['payload']})


def verify(binary, root):
    root.mkdir(parents=True, exist_ok=True)
    home = root / 'home'; home.mkdir(exist_ok=True)
    (home/'config.toml').write_text('[notice.model_migrations]\n"gpt-5.5" = "gpt-6-sol"\n', encoding='utf-8')
    for account, key in (('default', 'sk-default-tui-fixture'), ('alpha', 'sk-alpha-tui-fixture')):
        auth = home / ('auth.json' if account == 'default' else 'accounts/alpha/auth.json')
        auth.parent.mkdir(parents=True, exist_ok=True)
        auth.write_text(json.dumps({'OPENAI_API_KEY': key}), encoding='utf-8')
    server = RuntimeFixture()
    worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    results = {}
    try:
        for transport in ('http', 'ws'):
            for account in ('default', 'alpha'):
                label = f'{transport}-{account}'
                base = command(binary, server)
                settings = base[base.index('-c'):-1]
                args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox',
                        '--no-alt-screen', '-m', 'gpt-5.5', *settings,
                        '-c', 'tui.notifications=false', '-c', 'cli_auth_credentials_store="file"',
                        '-c', 'model_providers.fixture.requires_openai_auth=true',
                        '-c', f'model_providers.fixture.supports_websockets={str(transport == "ws").lower()}',
                        '-c', f'projects.{json.dumps(str(home))}.trust_level="trusted"']
                if account != 'default': args += ['--account', account]
                prompt = 'Reply with a short confirmation. LOCAL3_ACCOUNT_USER_REQUEST'
                args.append(prompt)
                start = len(server.requests)
                console = None
                try:
                    console = HiddenConsole(args, home)
                    deadline = time.monotonic() + 15
                    selected = []
                    while time.monotonic() < deadline:
                        selected = [entry for entry in server.requests[start:]
                                    if is_user_request(entry, prompt)]
                        if selected:
                            break
                        if K.WaitForSingleObject(console.process.hProcess, 0) == 0:
                            raise AssertionError(f'{label}: TUI exited before the user request')
                        time.sleep(.05)
                    assert len(selected) == 1, f'{label}: one target user request required'
                    time.sleep(.25)
                    found = windows(console.desktop)
                    assert len(found) == 1, (label, found)
                    hwnd, title = found[0]
                    console.screenshot(root / f'{label}.png')
                    request = selected[0]
                    assert request['transport'] == transport
                    credential = f'sk-{account}-tui-fixture'
                    results[label] = normalized(request, credential)
                    (root/f'{label}-normalized.json').write_text(json.dumps(results[label], ensure_ascii=False, indent=2), encoding='utf-8')
                    assert title.startswith('[f] '), title
                finally:
                    if console:
                        console.close()
            assert results[f'{transport}-default'] == results[f'{transport}-alpha'], f'{transport}: account changed non-auth request fields'
        (root/'result.json').write_text(json.dumps({'passed': True, 'real_tui': True,
            'startup': 'embedded: CLI -c overrides and bypass_hook_trust exclude daemon',
            'daemon_verified': False,
            'http_and_ws_account_requests_equal': True, 'visual_status': 'pending actual inspection'}, indent=2), encoding='utf-8')
        print('PASS real TUI account request equivalence', flush=True)
    finally:
        server.shutdown(); server.server_close(); worker.join(2)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
