"""Manual compaction/image recovery using the checklist §§3.6 and 6 as truth.

Checks original-image preservation on disk, image changes on the next request,
and a new WS compaction after the earlier compaction used HTTP.
"""
import hashlib
import json
import struct
import zlib
from pathlib import Path
import sys
import threading
import time

from test_local3_lifecycle import Rpc
from test_websocket_recovery import Fixture


class SamplingSizeFixture(Fixture):
    def events(self, payload, compact=False):
        # Checklist §6.1: unrelated failures break a run of consecutive sampling overflows.
        sequence = ('context_length_exceeded', 'server_error', 'context_length_exceeded',
                    'context_length_exceeded', 'context_length_exceeded')
        if len(self.failures) < len(sequence):
            code = sequence[len(self.failures)]
            self.failures.append(time.monotonic())
            return [{'type': 'response.failed', 'response': {'id': 'failed-fixture',
                     'status': 'failed', 'error': {'code': code, 'message': 'fixture failure'}}}]
        return super().events(payload, compact)


def images(payload):
    result = []
    def walk(value):
        if isinstance(value, dict):
            if value.get('type') == 'input_image':
                result.append((value.get('image_url'), value.get('detail')))
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
    walk(payload.get('input', []))
    return result


def verify(binary, root, scenario):
    root.mkdir(parents=True, exist_ok=True)
    fixture = SamplingSizeFixture(scenario) if scenario == 'sampling-context' else Fixture(scenario)
    # Manual compaction: keep the setup turn small and avoid an unrelated tool.
    fixture.tool_sent = True
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        home = root / 'home'
        http = 'http413' in scenario
        extra = ['-c', 'model_providers.fixture.supports_websockets=false',
                 '-c', 'model_providers.fixture.request_max_retries=100000'] if http else []
        rpc = Rpc(binary, fixture, home, root, extra)
        tid = rpc.start(home)
        def chunk(kind, data):
            return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))
        png = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!IIBBBBB', 2, 2, 8, 2, 0, 0, 0))
               + chunk(b'IDAT', zlib.compress((b'\0' + bytes([40, 120, 200]) * 2) * 2)) + chunk(b'IEND', b''))
        image_file = root / 'original.png'
        image_file.write_bytes(png)
        original_hash = hashlib.sha256(png).hexdigest()
        inputs = [{'type': 'localImage', 'path': str(image_file), 'detail': 'original'} for _ in range(7)]
        rpc.turn(tid, 'Remember these images.', inputs)
        if scenario == 'sampling-context':
            attempts = fixture.requests
            (root / 'requests.json').write_text(json.dumps(attempts, indent=2), encoding='utf-8')
            original = images(attempts[0]['payload'])
            assert len(attempts) == 6 and all(r['transport'] == 'ws' for r in attempts)
            assert all(images(r['payload']) == original for r in attempts[:-1]), 'do not shrink before three consecutive overflows'
            assert images(attempts[-1]['payload']) != original, 'shrink after three consecutive overflows'
            report = {'passed': True, 'scenario': scenario, 'consecutive_overflow_counter': True}
            (root / 'result.json').write_text(json.dumps(report), encoding='utf-8')
            print(json.dumps(report), flush=True)
            return
        initial_images = images(fixture.requests[-1]['payload'])
        start = len(fixture.requests)
        rpc.call('thread/compact/start', {'threadId': tid})
        completed = rpc.wait(lambda v: v.get('method') == 'turn/completed', timeout=180)
        assert completed['params']['turn']['status'] == 'completed', completed
        attempts = [r for r in fixture.requests[start:] if r['compact']]
        (root / 'requests.json').write_text(json.dumps(fixture.requests, indent=2), encoding='utf-8')
        assert len(attempts) >= 2, 'must exercise retry'
        original, shrunk = images(attempts[0]['payload']), images(attempts[3 if http else 1]['payload'])
        if not original and attempts[0]['payload'].get('previous_response_id'):
            # An incremental request refers to the images already sent in the setup turn.
            original = initial_images
        assert len(original) == len(shrunk) == len(inputs), 'retain ordered image slots'
        assert original != shrunk, 'next attempt after threshold must use changed images'
        if http:
            assert len(attempts) == 13, 'four tiers trigger after size failures 3/6/9/12'
            assert all(r['transport'] == 'http' for r in attempts)
            for threshold in (3, 6, 9, 12):
                assert images(attempts[threshold - 1]['payload']) != images(attempts[threshold]['payload'])
        elif 'fallback' in scenario:
            assert attempts[1]['transport'] == 'ws', 'one WS retry after first 1009'
            assert attempts[-1]['transport'] == 'http', 'failed WS retry must fall back'
            assert images(attempts[-1]['payload']) == shrunk, 'HTTP must not restore original images'
        else:
            assert all(r['transport'] == 'ws' for r in attempts), 'successful recovery stays WS'
        rpc.turn(tid, 'Continue with a short reply.')
        configured_transport = 'http' if http else 'ws'
        assert fixture.requests[-1]['transport'] == configured_transport
        start = len(fixture.requests)
        rpc.call('thread/compact/start', {'threadId': tid})
        rpc.wait(lambda v: v.get('method') == 'turn/completed')
        assert fixture.requests[start]['transport'] == configured_transport, 'next compaction uses configuration'
        assert hashlib.sha256(image_file.read_bytes()).hexdigest() == original_hash, 'disk image must stay unchanged'
        shrunk_events = []
        for file in home.glob('sessions/**/*.jsonl'):
            for line in file.read_text(encoding='utf-8').splitlines():
                record = json.loads(line)
                if record.get('type') == 'images_shrunk':
                    shrunk_events.append(record)
        assert shrunk_events, 'image change must be persisted for resume/fork'
        report = {'passed': True, 'scenario': scenario, 'transports': [r['transport'] for r in attempts],
                  'images_changed': True, 'persisted': True, 'disk_original_unchanged': True,
                  'next_compaction_transport': configured_transport}
        (root / 'requests.json').write_text(json.dumps(fixture.requests, indent=2), encoding='utf-8')
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report), flush=True)
    finally:
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve(), sys.argv[3])
