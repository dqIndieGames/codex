"""Downloaded-exe contracts: checklist §§3.1, 3.6, 4.1, 4.4 and 8.4.

Only loopback traffic and isolated fake credentials are used. No Rust build.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

from test_local3_lifecycle import Rpc
from test_websocket_recovery import Fixture, Handler, command
from _groundtruth.local3_retry_contract import RETRY_SECONDS


class RecordingHandler(Handler):
    def do_GET(self):
        self.server.local.headers = dict(self.headers)
        self.server.local.method = 'GET'
        self.server.local.path = self.path
        super().do_GET()

    def do_POST(self):
        self.server.local.headers = dict(self.headers)
        self.server.local.method = 'POST'
        self.server.local.path = self.path
        super().do_POST()


class RuntimeFixture(Fixture):
    def __init__(self):
        super().__init__('healthy')
        self.RequestHandlerClass = RecordingHandler
        self.local = threading.local()
        self.release = threading.Event()
        self.mode = 'healthy'
        self.tool_sent = True

    def record(self, transport, payload, user_agent=None):
        compact = super().record(transport, payload, user_agent)
        self.requests[-1]['headers'] = dict(getattr(self.local, 'headers', {}))
        self.requests[-1]['method'] = getattr(self.local, 'method', None)
        self.requests[-1]['path'] = getattr(self.local, 'path', None)
        return compact

    def events(self, payload, compact=False):
        if self.mode == 'stall':
            self.release.wait(100)
        if compact and self.mode == 'partial':
            return [
                {'type': 'response.output_item.done', 'output_index': 0,
                 'item': {'id': 'partial', 'type': 'message', 'role': 'assistant',
                          'content': [{'type': 'output_text', 'text': 'INVALID_PARTIAL_SUMMARY'}]}},
                {'type': 'response.failed', 'response': {'id': 'failed',
                 'error': {'code': 'server_error', 'message': 'fixture failure'}}},
            ]
        return super().events(payload, compact)


def configure(home, fixture, ws=True, token='fixture-old', tier='default', budget=0):
    home.mkdir(parents=True, exist_ok=True)
    (home / 'config.toml').write_text(f'''
model = "gpt-5.5"
model_provider = "fixture"
approval_policy = "never"
sandbox_mode = "danger-full-access"
force_service_tier_priority = false
service_tier = {json.dumps(tier)}
compact_prompt = "COMPACT_FIXTURE"
model_auto_compact_token_limit = 200000
[features]
remote_models = false
enable_request_compression = false
code_mode = false
[model_providers.fixture]
name = "Runtime verification"
base_url = "http://127.0.0.1:{fixture.server_port}/v1"
wire_api = "responses"
supports_websockets = {str(ws).lower()}
requires_openai_auth = false
experimental_bearer_token = {json.dumps(token)}
request_max_retries = 0
stream_max_retries = {budget}
''', encoding='utf-8')


def await_requests(fixture, count, timeout=30):
    deadline = time.monotonic() + timeout
    while len(fixture.requests) < count:
        if time.monotonic() >= deadline:
            raise AssertionError(f'expected {count} requests, got {len(fixture.requests)}')
        time.sleep(.05)


def exercise_refresh(binary, root, ws):
    fixture = RuntimeFixture()
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        home = root / 'home'
        configure(home, fixture, ws)
        rpc = Rpc(binary, fixture, home, root, settings=[])
        tid = rpc.start(home)
        fixture.mode = 'stall'
        turn = rpc.call('turn/start', {'threadId': tid, 'input': [
            {'type': 'text', 'text': 'Reply briefly.'}]})['turn']['id']
        await_requests(fixture, 1)
        # The original socket/header wait stays stalled. Only refresh can unblock the turn.
        fixture.mode = 'healthy'
        configure(home, fixture, ws, token='fixture-new', tier='flex')
        started = time.monotonic()
        rpc.call('thread/providerRuntime/refresh', {'threadId': tid})
        done = rpc.wait(lambda v: v.get('method') == 'turn/completed'
                        and v['params']['turn']['id'] == turn, timeout=30)
        assert done['params']['turn']['status'] == 'completed', done
        elapsed = time.monotonic() - started
        assert elapsed < RETRY_SECONDS, 'refresh must not add the remote-error retry delay'
        assert len(fixture.requests) == 2, 'one old request and one refreshed request'
        first, second = fixture.requests
        def auth(entry):
            return {k.lower(): v for k, v in entry['headers'].items()}.get('authorization')
        assert auth(first) == 'Bearer fixture-old'
        assert auth(second) == 'Bearer fixture-new'
        assert second['payload']['service_tier'] == 'flex', 'same active turn uses refreshed tier'
        assert all(r['transport'] == ('ws' if ws else 'http') for r in fixture.requests)
        assert all('-local3' not in r['user_agent'] for r in fixture.requests)
        return {'passed': True, 'case': 'refresh-ws' if ws else 'refresh-http',
                'seconds': round(elapsed, 3), 'same_url_token_and_active_tier': True}
    finally:
        fixture.release.set()
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


def exercise_history(binary, root):
    fixture = RuntimeFixture()
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        home = root / 'home'
        configure(home, fixture)
        rpc = Rpc(binary, fixture, home, root, settings=[])
        tid = rpc.start(home)
        rpc.turn(tid, 'Remember the history marker.')
        fixture.mode = 'partial'
        rpc.call('thread/compact/start', {'threadId': tid})
        failed = rpc.wait(lambda v: v.get('method') == 'turn/completed', timeout=30)
        assert failed['params']['turn']['status'] != 'completed', failed
        current = rpc.call('thread/read', {'threadId': tid, 'includeTurns': True})
        assert 'INVALID_PARTIAL_SUMMARY' not in json.dumps(current)
        rpc.stop()
        rpc = None
        for file in home.glob('sessions/**/*.jsonl'):
            assert 'INVALID_PARTIAL_SUMMARY' not in file.read_text(encoding='utf-8')
        fixture.mode = 'healthy'
        rpc = Rpc(binary, fixture, home, root / 'resume', settings=[])
        rpc.call('thread/resume', {'threadId': tid})
        rpc.turn(tid, 'Continue with a short reply.')
        current = rpc.call('thread/read', {'threadId': tid, 'includeTurns': True})
        assert 'INVALID_PARTIAL_SUMMARY' not in json.dumps(current)
        return {'passed': True, 'case': 'failed-summary-history-resume'}
    finally:
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


def exercise_budget(binary, root, budget):
    fixture = Fixture('close')
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    root.mkdir(parents=True, exist_ok=True)
    try:
        env = os.environ.copy()
        env['CODEX_HOME'] = str(root)
        env['CODEX_INTERNAL_RETRY_MODE'] = 'bounded'
        for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'CODEX_INTERNAL_AUTH_ACCOUNT'):
            env.pop(key, None)
        args = command(binary, fixture)
        args[-1:-1] = ['-c', f'model_providers.fixture.stream_max_retries={budget}']
        result = subprocess.run(args, cwd=root, env=env, capture_output=True, text=True,
                                encoding='utf-8', timeout=60, creationflags=subprocess.CREATE_NO_WINDOW)
        assert result.returncode != 0, 'budget exhaustion must terminate without fake success'
        assert len(fixture.requests) == budget + 1, (budget, len(fixture.requests))
        assert len(fixture.failures) == budget + 1
        return {'passed': True, 'case': f'budget-{budget}', 'attempts': len(fixture.requests)}
    finally:
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


if __name__ == '__main__':
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    root = Path(sys.argv[2]).resolve()
    root.mkdir(parents=True, exist_ok=True)
    results = []
    for name, fn in [
        ('refresh-ws', lambda r: exercise_refresh(binary, r, True)),
        ('refresh-http', lambda r: exercise_refresh(binary, r, False)),
        ('history', lambda r: exercise_history(binary, r)),
        ('zero', lambda r: exercise_budget(binary, r, 0)),
        ('bounded', lambda r: exercise_budget(binary, r, 1)),
    ]:
        if len(sys.argv) > 3 and name not in sys.argv[3:]:
            continue
        result = fn(root / name)
        results.append(result)
        print(json.dumps(result), flush=True)
        (root / 'results.json').write_text(json.dumps(results, indent=2), encoding='utf-8')
