"""清单5.3：真实窗口标题及控制面切换，数字分配取关系证据并另行读图。"""
import json
from pathlib import Path
import sys
import threading
import time
from _local3_hidden_console import HiddenConsole
from verify_local3_hidden_tui import windows, is_user_request
from test_local3_runtime_refresh import RuntimeFixture


def verify(binary, root, gaps=False):
    root.mkdir(parents=True, exist_ok=True)
    home = root/'home'; home.mkdir(exist_ok=True)
    for path in (home/'auth.json', home/'accounts/alpha/auth.json'):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({'OPENAI_API_KEY': 'sk-local-title-fixture'}), encoding='utf-8')
    server = RuntimeFixture()
    worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    def configure(provider):
        config = f'''model = "gpt-6-sol"
model_provider = "{provider}"
cli_auth_credentials_store = "file"
[features]
remote_models = false
enable_request_compression = false
[tui]
notifications = false
'''
        for key in ('yunyi', 'wanfeng'):
            config += f'''[model_providers.{key}]
name = "{key}"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
'''
        config += f'[projects.{json.dumps(str(home))}]\ntrust_level = "trusted"\n'
        (home/'config.toml').write_text(config, encoding='utf-8')
    console = None
    observed = []
    try:
        for account in ('default', 'alpha'):
            configure('openai')
            args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox']
            if account != 'default':
                args += ['--account', account]
            registrations_before = set((home/'app_servers').glob('*.json'))
            console = HiddenConsole(args, home)
            console.wait_text('GPT-6-Sol')
            title = windows(console.desktop)[0][1]
            number = int(title.split(']', 1)[0][1:])
            if account == 'default':
                first_number = number
            else:
                registry = json.loads((home/'accounts/title-sequences.json').read_text())
                assert number == registry[account] == first_number + 1
            console.screenshot(root/f'{account}-official.png')
            observed.append(title)
            if account == 'alpha':
                # 空白 TUI 尚无 loaded thread。先在本机 provider 建立真实线程，
                # 再走同一正式控制面切换；官方身份仅检查标题，不发送假 key 到外网。
                console.close(); console = None
                configure('yunyi')
                registrations_before = set((home/'app_servers').glob('*.json'))
                console = HiddenConsole(args, home)
                console.wait_text('GPT-6-Sol')
                console.type('Reply briefly. LOCAL3_TITLE_yunyi'); time.sleep(.15); console.keys([(13, '\r', 0)])
                console.wait_text('WS_RECOVERY_OK')
                console.screenshot(root/'alpha-yunyi.png')
                assert windows(console.desktop)[0][1].startswith('[y] ')
                registration = next(path for path in (home/'app_servers').glob('*.json')
                                    if path not in registrations_before)
                endpoint = json.loads(registration.read_text())['control_endpoint']
                for provider in (('wanfeng', 'openai', 'yunyi') if gaps else ('wanfeng', 'openai')):
                    configure(provider)
                    with open(endpoint, 'r+b', buffering=0) as pipe:
                        pipe.write(b'{"op":"refresh_all_loaded_threads","scope":"console"}\n')
                        response = json.loads(pipe.read(65536))
                    assert response['ok'] and response['applied_thread_ids'] and not response['failed_threads'], response
                    prefix = f'[{number}] ' if provider == 'openai' else f'[{provider[0].lower()}] '
                    deadline = time.monotonic()+4
                    while time.monotonic() < deadline:
                        title = windows(console.desktop)[0][1]
                        if title.startswith(prefix):
                            break
                        time.sleep(.05)
                    assert title.startswith(prefix), title
                    if provider != 'openai':
                        marker = 'LOCAL3_TITLE_'+provider
                        console.type('Reply briefly. '+marker); time.sleep(.15); console.keys([(13, '\r', 0)])
                        deadline = time.monotonic()+4
                        while not any(is_user_request(r, marker) for r in server.requests) and time.monotonic() < deadline:
                            time.sleep(.05)
                        assert any(is_user_request(r, marker) for r in server.requests)
                        console.wait_text('WS_RECOVERY_OK')
                    console.screenshot(root/f'alpha-{provider}.png')
                    observed.append(windows(console.desktop)[0][1])
                if gaps:
                    # 当前真实线程在 yunyi 上；同窗官方→第三方已由上面的最后一次切换覆盖。
                    session_files = list((home/'sessions').rglob('*.jsonl'))
                    assert session_files
                    with session_files[0].open(encoding='utf-8') as stream:
                        session_id = json.loads(stream.readline())['payload']['id']
                    console.close(); console = None
                    for action in ('resume', 'fork'):
                        console = HiddenConsole([*args, action, session_id, '--no-alt-screen'], home)
                        console.wait_text('WS_RECOVERY_OK')
                        marker = 'LOCAL3_TITLE_'+action
                        before = len(server.requests)
                        console.type('Reply briefly. '+marker); time.sleep(.15); console.keys([(13, '\r', 0)])
                        deadline = time.monotonic()+4
                        while not any(is_user_request(r, marker) for r in server.requests[before:]) and time.monotonic()<deadline:
                            time.sleep(.03)
                        assert any(is_user_request(r, marker) for r in server.requests[before:])
                        time.sleep(.2)
                        title = windows(console.desktop)[0][1]
                        assert title.startswith('[y] ') and not title.startswith('[y] [y] ')
                        console.screenshot(root/f'alpha-{action}.png')
                        (root/f'{action}-screen.txt').write_text(console.text(), encoding='utf-8')
                        observed.append(title)
                        console.close(); console = None
            if console:
                console.close(); console = None
        (root/'result.json').write_text(json.dumps({'passed': True, 'titles': observed,
            'visual_status': 'pending actual inspection'}, ensure_ascii=False, indent=2), encoding='utf-8')
        print('PASS title identity and live provider changes')
    finally:
        if console:
            console.close()
        server.shutdown(); server.server_close(); worker.join(2)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve(), 'gaps' in sys.argv[3:])
