"""Black-box contract: local3 checklist 3.1/3.4, using the supplied cloud artifact.

L1: each request must respect the independent mock response deadline exactly once.
Default without advice comes from the existing product-contract constant.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime, timezone, timedelta
from email.utils import format_datetime

from test_websocket_recovery import Fixture, Handler, command
from _groundtruth.local3_retry_contract import RETRY_SECONDS


class AdviceHandler(Handler):
    def do_POST(self):
        payload = json.loads(self.read_exact(int(self.headers['Content-Length'])))
        self.server.record('http', payload, self.headers.get('User-Agent'))
        event = self.server.failure()
        if event is not None and self.server.mode == 'http':
            body = json.dumps({'error': event['response']['error']}).encode()
            self.send_response(429)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            if self.server.advice is not None:
                self.send_header('Retry-After', self.server.advice)
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()
            return
        events = [event] if event else super(AdviceFixture, self.server).events(payload)
        body = ''.join(f'data: {json.dumps(e)}\n\n' for e in events).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        # A failed event must terminate the attempt even while the transport stays open.
        self.send_header('Transfer-Encoding', 'chunked')
        self.end_headers()
        self.wfile.write(f'{len(body):x}\r\n'.encode() + body + b'\r\n')
        self.wfile.flush()
        if event:
            self.server.release.wait(90)
        else:
            self.wfile.write(b'0\r\n\r\n')
            self.wfile.flush()


class AdviceFixture(Fixture):
    def __init__(self, mode, advice, location, failures=1, message=None, code='turn_rate_limit_exceeded'):
        super().__init__('healthy')
        self.RequestHandlerClass = AdviceHandler
        self.mode, self.advice, self.location = mode, advice, location
        self.limit = failures
        self.message = message or 'Codex is at capacity, please try again later'
        self.code = code
        self.release = threading.Event()
        self.deadlines = []
        self.tool_sent = True

    def failure(self):
        if len(self.failures) >= self.limit:
            return None
        now = time.monotonic()
        self.failures.append(now)
        error = {'code': self.code, 'message': self.message}
        event = {'type': 'response.failed', 'response': {'id': 'resp_failed', 'error': error}}
        if self.location.startswith('error'):
            event = {'type': 'error', 'error': error, 'status': 429}
        if self.advice is not None:
            advice = self.advice
            if advice == 'future-date':
                target = (datetime.now(timezone.utc) + timedelta(seconds=3)).replace(microsecond=0)
                self.deadlines.append((target - datetime.now(timezone.utc)).total_seconds())
                advice = format_datetime(target, usegmt=True)
            headers = {'rEtRy-AfTeR': advice}
            if self.location in ('nested', 'error-nested'):
                error['headers'] = headers
            elif self.location == 'response':
                event['response']['headers'] = headers
            else:
                event['headers'] = headers
            if self.location == 'priority':
                error['headers'] = {'Retry-After': 'invalid'}
                event['response']['headers'] = headers
                event['headers'] = {'Retry-After': '60'}
        return event

    def events(self, payload, compact=False):
        event = self.failure()
        return [event] if event else super().events(payload, compact)


def verify(binary, output, name, mode, advice, location, expected, failures=1, message=None, code='turn_rate_limit_exceeded'):
    fixture = AdviceFixture(mode, advice, location, failures, message, code)
    thread = threading.Thread(target=fixture.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix='local3-advice-') as home:
            env = os.environ.copy()
            env['CODEX_HOME'] = home
            env['CODEX_INTERNAL_RETRY_MODE'] = 'unbounded'
            for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'CODEX_TUI_COMPLETION_DIAGNOSTICS_DIR'):
                env.pop(key, None)
            args = command(binary, fixture)
            args[-1:-1] = ['-c', f'model_providers.fixture.supports_websockets={str(mode == "ws").lower()}',
                          '-c', 'model_providers.fixture.request_max_retries=100000']
            result = subprocess.run(args, cwd=home, env=env, capture_output=True, text=True,
                encoding='utf8', errors='replace', timeout=120, creationflags=subprocess.CREATE_NO_WINDOW)
        (output / f'{name}.stdout.txt').write_text(result.stdout, encoding='utf8')
        (output / f'{name}.stderr.txt').write_text(result.stderr, encoding='utf8')
        assert result.returncode == 0 and 'WS_RECOVERY_OK' in result.stdout, (name, result.stderr[-2500:], result.stdout[-1500:])
        waits = []
        assert len(fixture.failures) == failures, (name, fixture.failures)
        for index, failed in enumerate(fixture.failures):
            next_at = min(r['at'] for r in fixture.requests if r['at'] > failed)
            wait = next_at - failed
            required = fixture.deadlines[index] if expected is None else expected
            assert required - .1 <= wait <= required + 4, (name, required, wait)
            waits.append(round(wait, 3))
        report = {'case': name, 'passed': True, 'waits': waits,
                  'requests': [{k: v for k, v in r.items() if k != 'payload'} for r in fixture.requests]}
        (output / f'{name}.json').write_text(json.dumps(report, indent=2), encoding='utf8')
        print(json.dumps(report), flush=True)
    finally:
        fixture.release.set()
        fixture.shutdown()
        fixture.server_close()
        thread.join(5)


if __name__ == '__main__':
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    output = Path(sys.argv[2]).resolve()
    output.mkdir(parents=True, exist_ok=True)
    cases = [
        ('http-short', 'http', '2', 'nested', 2),
        ('http-long', 'http', '30', 'nested', 30),
        ('sse-ten', 'sse', '10', 'nested', 10),
        ('sse-response', 'sse', '2', 'response', 2),
        ('sse-top', 'sse', '2', 'top', 2),
        ('sse-priority', 'sse', '2', 'priority', 2),
        ('sse-error-nested', 'sse', '2', 'error-nested', 2),
        ('sse-error-top', 'sse', '2', 'error-top', 2),
        ('ws-failed', 'ws', '2', 'nested', 2),
        ('ws-error', 'ws', '2', 'error-nested', 2),
        ('sse-zero', 'sse', '0', 'nested', 0),
        ('sse-expired', 'sse', 'Wed, 21 Oct 2015 07:28:00 GMT', 'top', 0),
        ('sse-date', 'sse', 'future-date', 'top', None),
        ('sse-invalid', 'sse', '-1', 'top', RETRY_SECONDS),
        ('sse-missing', 'sse', None, 'top', RETRY_SECONDS),
        ('http-recovery', 'http', '0', 'nested', 0, 6),
        ('ws-fallback', 'ws', '2', 'nested', 2, 4),
        ('sse-text', 'sse', 'invalid', 'nested', 2, 1, 'Please try again in 2s.'),
        ('ws-text', 'ws', 'invalid', 'error-nested', 2, 1, 'Please try again in 2s.'),
        ('sse-context', 'sse', '2', 'nested', 2, 1, None, 'context_length_exceeded'),
    ]
    requested = set(sys.argv[3:])
    for case in cases:
        if not requested or case[0] in requested:
            verify(binary, output, *case)
