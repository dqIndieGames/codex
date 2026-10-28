"""Checklist 3.1/3.6: independent local/V2 compaction transport deadlines."""
import json
from pathlib import Path
import sys
import threading
import time

from test_websocket_recovery import Fixture
from test_local3_lifecycle import Rpc
from test_local3_runtime_refresh import configure


class CompactFixture(Fixture):
    def __init__(self):
        super().__init__('healthy')
        self.tool_sent = True

    def events(self, payload, compact=False):
        if compact and not self.failures:
            self.failures.append(time.monotonic())
            return [{'type': 'response.failed', 'response': {'id': 'failed_compact', 'error': {
                'code': 'turn_rate_limit_exceeded', 'message': 'Codex is at capacity, please try again later',
                'headers': {'Retry-After': '2'}}}}]
        return super().events(payload, compact)


def verify(binary, root, v2, ws):
    fixture = CompactFixture()
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        home = root / 'home'
        configure(home, fixture, ws=ws, budget=100)
        config = (home / 'config.toml').read_text(encoding='utf8')
        if v2:
            config = config.replace('name = "Runtime verification"', 'name = "OpenAI"')
        (home / 'config.toml').write_text(config, encoding='utf8')
        rpc = Rpc(binary, fixture, home, root, settings=[])
        tid = rpc.start(home)
        rpc.turn(tid, 'Remember this short message.')
        rpc.call('thread/compact/start', {'threadId': tid})
        complete = rpc.wait(lambda event: event.get('method') == 'turn/completed', timeout=45)
        assert complete['params']['turn']['status'] == 'completed', complete
        requests = [r for r in fixture.requests if r['compact']]
        assert len(fixture.failures) == 1 and len(requests) == 2, fixture.requests
        assert all(r['transport'] == ('ws' if ws else 'http') for r in requests)
        wait = requests[1]['at'] - fixture.failures[0]
        assert 1.9 <= wait <= 6, wait
        rpc.turn(tid, 'Continue briefly after compaction.')
        report = {'passed': True, 'v2': v2, 'ws': ws, 'wait': wait,
                  'requests': fixture.requests}
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf8')
        print(json.dumps({k: v for k, v in report.items() if k != 'requests'}), flush=True)
    finally:
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


if __name__ == '__main__':
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    root = Path(sys.argv[2]).resolve()
    for name, v2, ws in [('local-http', False, False), ('local-ws', False, True),
                          ('v2-http', True, False), ('v2-ws', True, True)]:
        if len(sys.argv) == 3 or name in sys.argv[3:]:
            verify(binary, root / name, v2, ws)
