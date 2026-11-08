"""清单 5.3：真实 TUI 默认／alpha 的 HTTP、WS 请求一致性，L1 关系断言。

复用隐藏桌面与本机服务；只使用本轮 exe，不强制修改 TUI 后台选择。
动态身份作一一映射，保留字段及重复引用关系，不整块忽略 metadata。
"""
import base64
import ctypes as C
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time

from test_local3_runtime_refresh import RuntimeFixture
from test_websocket_recovery import command
from verify_local3_hidden_tui import U, K, STARTUPINFO, PROCESSINFO, windows, capture


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
                env = os.environ.copy(); env['CODEX_HOME'] = str(home)
                for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'CODEX_INTERNAL_AUTH_ACCOUNT'):
                    env.pop(key, None)
                base = command(binary, server)
                settings = base[base.index('-c'):-1]
                args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox',
                        '--no-alt-screen', '-m', 'gpt-5.5', *settings,
                        '-c', 'tui.notifications=false', '-c', 'cli_auth_credentials_store="file"',
                        '-c', 'model_providers.fixture.requires_openai_auth=true',
                        '-c', f'model_providers.fixture.supports_websockets={str(transport == "ws").lower()}',
                        '-c', f'projects.{json.dumps(str(home))}.trust_level="trusted"']
                if account != 'default': args += ['--account', account]
                args.append('Reply with a short confirmation.')
                desktop_name = f'Local3Accounts_{os.getpid()}_{label}'
                desktop = U.CreateDesktopW(desktop_name, None, None, 0, 0x01ff, None)
                if not desktop: raise C.WinError(C.get_last_error())
                previous = U.GetThreadDesktop(K.GetCurrentThreadId())
                info, process = STARTUPINFO(), PROCESSINFO()
                info.cb, info.lpDesktop = C.sizeof(info), desktop_name
                block = C.create_unicode_buffer('\0'.join(f'{k}={v}' for k, v in sorted(env.items()))+'\0\0')
                host = str(Path(os.environ['SystemRoot'])/'System32/conhost.exe')
                start = len(server.requests)
                try:
                    if not K.CreateProcessW(host, C.create_unicode_buffer(subprocess.list2cmdline([host, *args])),
                            None, None, False, 0x10 | 0x4 | 0x400, block, str(home), C.byref(info), C.byref(process)):
                        raise C.WinError(C.get_last_error())
                    K.ResumeThread(process.hThread)
                    if not U.SetThreadDesktop(desktop): raise C.WinError(C.get_last_error())
                    deadline = time.monotonic() + 15
                    while len(server.requests) == start and time.monotonic() < deadline: time.sleep(.05)
                    assert len(server.requests) == start + 1, f'{label}: one successful request required'
                    time.sleep(.25)
                    found = windows(desktop)
                    assert len(found) == 1, (label, found)
                    hwnd, title = found[0]
                    U.SetWindowPos(hwnd, None, 0, 0, 1200, 800, 0x14)
                    capture(hwnd, root / f'{label}.png')
                    request = server.requests[-1]
                    assert request['transport'] == transport
                    credential = f'sk-{account}-tui-fixture'
                    results[label] = normalized(request, credential)
                    (root/f'{label}-normalized.json').write_text(json.dumps(results[label], ensure_ascii=False, indent=2), encoding='utf-8')
                    assert title.startswith('[f] '), title
                finally:
                    if process.hProcess:
                        subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                            creationflags=subprocess.CREATE_NO_WINDOW, timeout=5)
                        K.WaitForSingleObject(process.hProcess, 3000)
                        K.CloseHandle(process.hProcess); K.CloseHandle(process.hThread)
                    U.SetThreadDesktop(previous); U.CloseDesktop(desktop)
            assert results[f'{transport}-default'] == results[f'{transport}-alpha'], f'{transport}: account changed non-auth request fields'
        (root/'result.json').write_text(json.dumps({'passed': True, 'real_tui': True,
            'http_and_ws_account_requests_equal': True, 'visual_status': 'pending actual inspection'}, indent=2), encoding='utf-8')
        print('PASS real TUI account request equivalence', flush=True)
    finally:
        server.shutdown(); server.server_close(); worker.join(2)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
