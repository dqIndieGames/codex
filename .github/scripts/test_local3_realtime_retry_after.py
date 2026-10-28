"""Checklist 3.1: real artifact, independent loopback HTTP/WS retry deadlines."""
import base64
import hashlib
import json
from pathlib import Path
import sys
import threading
import time

from test_websocket_recovery import Fixture, Handler
from test_local3_lifecycle import Rpc
from test_local3_runtime_refresh import configure


class RealtimeHandler(Handler):
    def reply(self, status, body, headers=()):
        self.send_response(status)
        self.send_header('Content-Length', str(len(body)))
        for key, value in headers:
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(body)
        self.wfile.flush()

    def failure(self, kind):
        now = time.monotonic()
        self.server.attempts.append({'kind': kind, 'at': now, 'path': self.path})
        previous = [r for r in self.server.attempts if r['kind'] == kind]
        if len(previous) == 1:
            self.reply(429, b'{"error":{"code":"turn_rate_limit_exceeded"}}', [('Retry-After', '2')])
            return True
        return False

    def do_POST(self):
        self.read_exact(int(self.headers.get('Content-Length', '0')))
        if not self.failure('call'):
            self.reply(201, b'v=0\r\ns=local3-fixture\r\n', [
                ('Content-Type', 'application/sdp'), ('Location', '/v1/realtime/calls/rtc_fixture')])

    def do_GET(self):
        # Responses warmup is separate and must not consume realtime failures.
        if self.path.startswith('/v1/responses'):
            return super().do_GET()
        if self.failure('sideband' if 'call_id' in self.path else 'websocket'):
            return
        accept = base64.b64encode(hashlib.sha1((self.headers['Sec-WebSocket-Key'] +
            '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
        self.send_response(101)
        for key, value in [('Upgrade', 'websocket'), ('Connection', 'Upgrade'), ('Sec-WebSocket-Accept', accept)]:
            self.send_header(key, value)
        self.end_headers()
        self.wfile.flush()
        self.close_connection = True
        self.frame(1, json.dumps({'type': 'session.updated', 'session': {'id': 'sess_fixture'}}).encode())
        self.server.release.wait(30)


def verify(binary, root, kind):
    fixture = Fixture('healthy')
    fixture.RequestHandlerClass = RealtimeHandler
    fixture.attempts = []
    fixture.release = threading.Event()
    thread = threading.Thread(target=fixture.serve_forever, daemon=True)
    thread.start()
    rpc = None
    try:
        home = root / 'home'
        configure(home, fixture, budget=100)
        text = (home / 'config.toml').read_text(encoding='utf8').replace('request_max_retries = 0', 'request_max_retries = 100')
        text = (f'experimental_realtime_ws_base_url = "http://127.0.0.1:{fixture.server_port}/v1"\n'
                f'experimental_realtime_webrtc_call_base_url = "http://127.0.0.1:{fixture.server_port}/v1"\n' + text)
        text = text.replace('[features]', '[features]\nrealtime_conversation = true')
        (home / 'config.toml').write_text(text, encoding='utf8')
        rpc = Rpc(binary, fixture, home, root, settings=[])
        tid = rpc.start(home)
        transport = {'type': 'websocket'} if kind == 'websocket' else {'type': 'webrtc', 'sdp': 'v=0\r\ns=local3-fixture\r\n'}
        # Official WebRTC AVAS accepts v1/v3 audio; v2 text is WS-only.
        rpc.call('thread/realtime/start', {'threadId': tid,
            'outputModality': 'text' if kind == 'websocket' else 'audio',
            'includeStartupContext': False, 'version': 'v2' if kind == 'websocket' else 'v1',
            'transport': transport})
        rpc.wait(lambda v: v.get('method') == 'thread/realtime/started', timeout=30)
        wanted = ['websocket'] if kind == 'websocket' else ['call', 'sideband']
        deadline = time.monotonic() + 30
        while not all(sum(r['kind'] == k for r in fixture.attempts) >= 2 for k in wanted):
            if time.monotonic() >= deadline:
                raise TimeoutError(fixture.attempts)
            time.sleep(.05)
        waits = {}
        for channel in wanted:
            attempts = [r for r in fixture.attempts if r['kind'] == channel]
            wait = attempts[1]['at'] - attempts[0]['at']
            assert 1.9 <= wait <= 6, (channel, wait)
            waits[channel] = wait
        rpc.call('thread/realtime/stop', {'threadId': tid})
        report = {'passed': True, 'case': kind, 'waits': waits, 'attempts': fixture.attempts}
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf8')
        print(json.dumps(report), flush=True)
    finally:
        root.mkdir(parents=True, exist_ok=True)
        (root / 'attempts.json').write_text(json.dumps(fixture.attempts, indent=2), encoding='utf8')
        fixture.release.set()
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        thread.join(5)


if __name__ == '__main__':
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    root = Path(sys.argv[2]).resolve()
    for kind in sys.argv[3:] or ['websocket', 'webrtc']:
        verify(binary, root / kind, kind)
