"""Checklist 3.1/4.1: cancel and refresh interrupt a live server-advised wait.

L1 invariants compare request timing with the mock's own 30-second advice;
they do not infer expectations from the implementation.
"""
import json
from pathlib import Path
import sys
import threading
import time

from test_local3_retry_after import AdviceFixture
from test_local3_runtime_refresh import configure
from test_local3_lifecycle import Rpc


def verify(binary, root, action, ws):
    advised = 30
    fixture = AdviceFixture('ws' if ws else 'sse', str(advised), 'nested')
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        home = root / 'home'
        configure(home, fixture, ws=ws, budget=100)
        rpc = Rpc(binary, fixture, home, root, settings=[])
        tid = rpc.start(home)
        turn = rpc.call('turn/start', {'threadId': tid, 'input': [
            {'type': 'text', 'text': 'Reply briefly.'}]})['turn']['id']
        deadline = time.monotonic() + 15
        while not fixture.failures:
            if time.monotonic() >= deadline:
                raise TimeoutError('mock did not receive the first request')
            time.sleep(.05)
        # Give the client time to consume the failed event, without ending its wait.
        time.sleep(.5)
        before = time.monotonic()
        if action == 'refresh':
            configure(home, fixture, ws=ws, token='fixture-new', budget=100)
            rpc.call('thread/providerRuntime/refresh', {'threadId': tid})
        else:
            rpc.call('turn/interrupt', {'threadId': tid, 'turnId': turn})
        result = rpc.wait(lambda event: event.get('method') == 'turn/completed'
            and event['params']['turn']['id'] == turn, timeout=10)
        elapsed = time.monotonic() - before
        assert elapsed < advised / 3, elapsed
        expected = 'completed' if action == 'refresh' else 'interrupted'
        assert result['params']['turn']['status'] == expected, result
        assert len(fixture.requests) == (2 if action == 'refresh' else 1), fixture.requests
        if action == 'refresh':
            assert fixture.requests[-1]['at'] - fixture.failures[0] < advised, fixture.requests
        report = {'passed': True, 'action': action, 'ws': ws, 'seconds': elapsed,
                  'requests': len(fixture.requests)}
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf8')
        print(json.dumps(report), flush=True)
    finally:
        fixture.release.set()
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


if __name__ == '__main__':
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    root = Path(sys.argv[2]).resolve()
    for name, action, ws in [('cancel-http', 'cancel', False),
                              ('refresh-http', 'refresh', False), ('refresh-ws', 'refresh', True)]:
        if len(sys.argv) == 3 or name in sys.argv[3:]:
            verify(binary, root / name, action, ws)
