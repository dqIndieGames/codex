"""L0 checklist 6.1/6.4; L1 originals unchanged, strict reduction, exact resume/fork.
Only run via the cumulative release budget, using a downloaded exe.
"""
import base64
import hashlib
import io
import json
import random
import threading
import time
import uuid
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from test_local3_lifecycle import Rpc
from test_websocket_recovery import Fixture
from _groundtruth.local3_retry_contract import IMAGE_FAILURES_PER_ROUND as COUNT
from _groundtruth.local3_retry_contract import IMAGE_KEEP_RECENT, IMAGE_LONG_EDGES, IMAGE_PLACEHOLDER_SIZE

def images(payload):
    result = []
    def walk(value):
        if isinstance(value, dict):
            if value.get('type') == 'input_image': result.append((value.get('image_url'), value.get('detail')))
            for child in value.values(): walk(child)
        elif isinstance(value, list):
            for child in value: walk(child)
    walk(payload.get('input', []))
    return result

class ImageFixture(Fixture):
    def __init__(self, root):
        super().__init__('images-v2')
        self.root, self.phase = root, 'ready'
        self.remaining, self.failure_index = 0, 0
        self.last_images = []
        self.snapshots, self.tool_sent = set(), True
        self.failure_plan = []

    def record(self, transport, payload, user_agent=None):
        compact = 'COMPACT_FIXTURE' in json.dumps(payload) or any(i.get('type') == 'compaction_trigger' for i in payload.get('input', []))
        summaries = []
        urls = images(payload)
        for index, (url, _) in enumerate(urls):
            assert url and url.startswith('data:'), 'no omitted image or unexpected file reference'
            data = base64.b64decode(url.split(',', 1)[1])
            picture = Image.open(io.BytesIO(data)); picture.load()
            sha = hashlib.sha256(data).hexdigest()
            alpha = picture.getchannel('A').getextrema() if 'A' in picture.getbands() else None
            summaries.append({'sha256': sha, 'bytes': len(data), 'size': list(picture.size), 'format': picture.format,
                              'alpha': alpha})
            if index in (0, 1, len(urls)-1) and sha not in self.snapshots:
                (self.root / f'sent-{index}-{sha[:12]}.{picture.format.lower()}').write_bytes(data)
                self.snapshots.add(sha)
        self.last_images = summaries
        self.requests.append({'transport': transport, 'compact': compact, 'at': time.monotonic(),
            'json_bytes': len(json.dumps(payload).encode()), 'images': summaries,
            'user_agent': user_agent, 'phase': self.phase, 'failure_index': self.failure_index})
        return compact

    def http_error(self, payload, compact):
        if self.failure_plan and 'http' in self.failure_plan[0]:
            failure = self.failure_plan.pop(0)
            return failure['http'], {'error': failure['error']}
        if self.phase == 'fail' and self.remaining and self.failure_index % COUNT == 0:
            self.failure_index += 1; self.remaining -= 1
            return 413, {'error': {'message': 'Payload Too Large'}}
        return None

    def events(self, payload, compact=False):
        if self.failure_plan:
            return [self.failure_plan.pop(0)]
        if self.phase == 'fail' and self.remaining:
            self.failure_index += 1; self.remaining -= 1
            # 同一六档序列覆盖已解析、未知、缺失和异常 error，真值来自清单 6.1。
            causes = [None, {'code': 42}, {'code': 'invalid_api_key'}, {'code': 'insufficient_quota'},
                      {'code': 'turn_rate_limit_exceeded'}, {'code': 'server_is_overloaded'},
                      {'code': 'invalid_prompt'}, {'code': 'cyber_policy'}]
            response = {'status': 'failed', 'error': causes[(self.failure_index - 1) % len(causes)]}
            return [{'type': 'response.failed', 'response': response, 'headers': {'Retry-After': '0'}}]
        return super().events(payload, compact)

    def arm(self, failures):
        self.phase, self.remaining, self.failure_index = 'fail', failures, 0

def verify(binary, root):
    root.mkdir(parents=True, exist_ok=True)
    server = ImageFixture(root)
    worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    rpc = None
    try:
        originals = []
        font = ImageFont.truetype(r'C:\Windows\Fonts\consola.ttf', 40)
        for index in range(IMAGE_KEEP_RECENT + 3):
            picture = Image.frombytes('RGB', (1440, 900), random.Random(index).randbytes(1440*900*3))
            draw = ImageDraw.Draw(picture)
            draw.rectangle((30, 30, 1410, 380), fill=(245, 245, 245))
            draw.text((60, 60), f'Image {index+1} | Code: x = 1280 / 768\nValue: 1234567890   q95 q85 q75\nFine lines and comparison text', font=font, fill=(20, 20, 20))
            draw.line((60, 330, 1350, 330), fill=(20, 20, 20), width=1)
            draw.rectangle((100, 430, 670, 820), fill=(80, 160, 220))
            draw.ellipse((530, 460, 610, 540), fill=(255, 220, 90))
            draw.polygon([(100, 800), (310, 520), (440, 680), (570, 560), (670, 800)], fill=(35, 100, 60))
            path = root / f'original-{index}.png'; picture.save(path, compress_level=0)
            originals.append((path, hashlib.sha256(path.read_bytes()).hexdigest()))
        home = root / 'home'
        extra = ['-c', 'model_providers.fixture.supports_websockets=false', '-c', 'model_providers.fixture.request_max_retries=5']
        rpc = Rpc(binary, server, home, root / 'rpc', extra)
        tid = rpc.start(home)
        rpc.turn(tid, 'Analyze these images in arrival order.', [
            {'type': 'localImage', 'path': str(path), 'detail': 'original'} for path, _ in originals])
        initial_images = server.last_images
        boundary_thread = rpc.call('thread/fork', {'threadId': tid})['thread']['id']
        def failed(code='server_error'):
            return {'type': 'response.failed', 'response': {'error': {
                'code': code, 'message': 'fixture', 'headers': {'Retry-After': '0'}}}}
        ordinary = {'type': 'error', 'error': {'code': 'rate_limit_exceeded',
                    'message': 'fixture', 'headers': {'Retry-After': '0'}}, 'status': 429}
        sequence = [failed(), failed(), ordinary, failed(), failed()]
        server.failure_plan = sequence.copy()
        start = len(server.requests)
        rpc.turn(boundary_thread, 'Check the failure counting boundary.')
        assert len(server.requests[start:]) == len(sequence) + 1
        assert all(r['images'] == initial_images for r in server.requests[start:]), 'ordinary error resets image streak'
        for code in ('request_body_too_large', 'image_dimensions_exceeded'):
            image_thread = rpc.call('thread/fork', {'threadId': tid})['thread']['id']
            server.failure_plan = [{'http': 400, 'error': {'code': code, 'message': 'fixture'}} for _ in range(COUNT)]
            start = len(server.requests)
            rpc.turn(image_thread, 'Continue after the explicit image limit.')
            attempts = server.requests[start:]
            assert len(attempts) == COUNT + 1
            assert all(r['images'] == initial_images for r in attempts[:COUNT])
            assert sum(x['bytes'] for x in attempts[-1]['images']) < sum(x['bytes'] for x in initial_images)
            if code == 'image_dimensions_exceeded':
                assert all(max(x['size']) <= IMAGE_LONG_EDGES[3] for x in attempts[-1]['images'])
        # 两种上下文封装都应交回官方，不增加尝试以凑三次，不改变图片。
        for event in (
            {'http': 400, 'error': {'code': 'context_length_exceeded', 'message': 'context limit'}},
            failed('context_length_exceeded'),
        ):
            context_thread = rpc.call('thread/fork', {'threadId': tid})['thread']['id']
            server.failure_plan = [event]
            start = len(server.requests)
            turn = rpc.call('turn/start', {'threadId': context_thread, 'input': [{'type': 'text', 'text': 'Continue.'}]})['turn']['id']
            completed = rpc.wait(lambda e: e.get('method') == 'turn/completed' and e['params']['turn']['id'] == turn)
            assert completed['params']['turn']['status'] != 'completed'
            assert len(server.requests[start:]) == 1 and server.last_images == initial_images
        server.phase = 'ready'
        server.arm(COUNT*6); start = len(server.requests)
        rpc.turn(tid, 'Continue using the images in arrival order.')
        attempts = server.requests[start:]
        assert len(attempts) == COUNT*6+1, 'one count per remote failure'
        for round_index in range(6):
            before = attempts[round_index*COUNT]['images']
            assert all(x['images'] == before for x in attempts[round_index*COUNT:(round_index+1)*COUNT]), 'no early downgrade'
            after = attempts[(round_index+1)*COUNT]['images']
            assert sum(x['bytes'] for x in after) < sum(x['bytes'] for x in before)
            assert len(after) == len(before)
        for tier, edge in IMAGE_LONG_EDGES.items():
            assert all(max(x['size']) <= edge for x in attempts[tier*COUNT]['images'])
        assert all(x['format'] == 'PNG' for x in attempts[COUNT]['images'])
        assert all(x['format'] == 'JPEG' for x in attempts[2*COUNT]['images'])
        assert all(x['size'] == initial_images[i]['size'] for i, x in enumerate(attempts[2*COUNT]['images']))
        fifth = attempts[5*COUNT]['images']
        retained = {i for i, x in enumerate(fifth) if tuple(x['size']) != IMAGE_PLACEHOLDER_SIZE}
        expected = set(range(len(originals)-IMAGE_KEEP_RECENT, len(originals)))
        assert retained == expected, (retained, expected)
        sixth = attempts[6*COUNT]['images']
        assert sum(tuple(x['size']) == IMAGE_PLACEHOLDER_SIZE for x in sixth) == len(originals)-len(expected)+1
        assert {i for i, x in enumerate(sixth) if tuple(x['size']) != IMAGE_PLACEHOLDER_SIZE} == expected - {min(expected)}, 'stage six evicts the oldest remaining image'
        rpc.stop()
        # 删除本测试的发送缓存，保留原始内容；resume 必须精确重建，不能恢复更大原图。
        cache = home / 'cache' / 'local3-images-v2'
        for path in cache.glob('*'):
            if path.name.endswith('.image'):
                path.unlink()
        source_keys, result_keys = set(), set()
        for path in home.rglob('*.jsonl'):
            for line in path.read_text(encoding='utf-8').splitlines():
                entry = json.loads(line)
                if entry.get('type') == 'images_shrunk':
                    for metadata in entry['payload'].get('images', []):
                        source_keys.add(metadata['source_digest'])
                        result_keys.add(metadata['result_digest'])
        assert result_keys, 'recorded image references must exist before cache rebuild'
        for key in result_keys - source_keys:
            (cache / key).unlink(missing_ok=True)
        rpc = Rpc(binary, server, home, root / 'resumed', extra)
        server.phase = 'ready'
        rpc.call('thread/resume', {'threadId': tid}); rpc.turn(tid, 'Continue with the same image context.')
        assert server.last_images == sixth, 'resume exact image state'
        fork = rpc.call('thread/fork', {'threadId': tid})['thread']['id']
        rpc.turn(fork, 'Continue without restoring degraded images.')
        assert server.last_images == sixth, 'fork exact image state'
        # 独立旧四档契约：旧第四档只留最后一张；之后加入的图片不能被重算淘汰。
        legacy_id = str(uuid.uuid4())
        stamp = '2026-11-07T00:00:00Z'
        url = 'data:image/png;base64,' + base64.b64encode(originals[0][0].read_bytes()).decode()
        legacy_items = [
            {'type': 'session_meta', 'payload': {'id': legacy_id, 'session_id': legacy_id,
                'timestamp': stamp, 'cwd': str(home), 'originator': 'local3_verify',
                'cli_version': '0.165.0-local3', 'model_provider': 'fixture', 'base_instructions': None}},
            {'type': 'response_item', 'payload': {'type': 'message', 'role': 'user', 'content': [
                {'type': 'input_image', 'image_url': url, 'detail': 'original'} for _ in originals]}},
            {'type': 'images_shrunk', 'payload': {'tier': 4, 'changed': len(originals)-1}},
            {'type': 'response_item', 'payload': {'type': 'message', 'role': 'user', 'content': [
                {'type': 'input_image', 'image_url': url, 'detail': 'original'}]}},
        ]
        legacy_path = root / 'legacy-four-stage.jsonl'
        legacy_path.write_text('\n'.join(json.dumps({'timestamp': stamp, **item}) for item in legacy_items)+'\n', encoding='utf-8')
        resumed = rpc.call('thread/resume', {'threadId': legacy_id, 'path': str(legacy_path)})['thread']['id']
        rpc.turn(resumed, 'Continue the legacy image history.')
        assert len(server.last_images) == len(originals)+1
        assert [tuple(x['size']) == IMAGE_PLACEHOLDER_SIZE for x in server.last_images] == [True]*(len(originals)-1)+[False, False]
        transparent = Image.new('RGBA', (1440, 900), (0, 0, 0, 0))
        drawing = ImageDraw.Draw(transparent)
        drawing.rectangle((100, 100, 1300, 800), fill=(40, 160, 220, 128))
        drawing.text((150, 200), 'Alpha 128 | Reference 12345', font=font, fill=(255, 255, 255, 255))
        alpha_path = root / 'original-transparent.png'
        transparent.save(alpha_path, compress_level=0)
        originals.append((alpha_path, hashlib.sha256(alpha_path.read_bytes()).hexdigest()))
        server.phase = 'ready'
        rpc.turn(tid, 'Analyze this new batch in arrival order.', [
            {'type': 'localImage', 'path': str(originals[0][0]), 'detail': 'original'},
            {'type': 'localImage', 'path': str(alpha_path), 'detail': 'original'}])
        new_before = server.last_images[-1]
        server.arm(COUNT); rpc.turn(tid, 'Continue after adding the new batch.')
        assert server.last_images[-1]['bytes'] < new_before['bytes']
        assert tuple(server.last_images[-1]['size']) != IMAGE_PLACEHOLDER_SIZE, 'new image cannot jump to placeholder'
        assert server.last_images[-1]['alpha'] == new_before['alpha'], 'lossless stage preserves alpha range'
        # 共享六档只跑一次；两个独立摘要入口各取三次失败后的真实请求证据。
        prior = server.last_images
        compact_evidence = []
        for provider_name in ('Local image fixture', 'OpenAI'):
            rpc.stop()
            rpc = Rpc(binary, server, home, root / ('compact-local' if provider_name != 'OpenAI' else 'compact-v2'),
                      [*extra, '-c', f'model_providers.fixture.name={json.dumps(provider_name)}'])
            compact_thread = rpc.call('thread/fork', {'threadId': tid})['thread']['id']
            server.arm(COUNT)
            start = len(server.requests)
            rpc.call('thread/compact/start', {'threadId': compact_thread})
            completed = rpc.wait(lambda event: event.get('method') == 'turn/completed')
            assert completed['params']['turn']['status'] == 'completed', completed
            attempts = server.requests[start:]
            assert len(attempts) == COUNT + 1 and all(r['compact'] for r in attempts)
            assert attempts[0]['images'] == prior, 'compact must start with the persisted processed images'
            assert all(r['images'] == prior for r in attempts[:COUNT])
            assert sum(x['bytes'] for x in attempts[-1]['images']) < sum(x['bytes'] for x in prior)
            compact_evidence.append(provider_name)
        assert all(hashlib.sha256(path.read_bytes()).hexdigest() == digest for path, digest in originals)
        markers = []
        for path in home.rglob('rollout-*.jsonl'):
            for line in path.read_text(encoding='utf-8').splitlines():
                item = json.loads(line)
                if item.get('type') == 'images_shrunk':
                    assert 'base64,' not in line
                    markers.append(item)
        assert markers, 'durable image changes'
        report = {'passed': True, 'six_stages': True, 'chronological_retention': True, 'resume': True, 'fork': True,
                  'cache_rebuilt': True, 'legacy_four_stage': True,
                  'new_image_after_top': True, 'originals_unchanged': True, 'compact_entries': compact_evidence,
                  'visual_status': 'pending actual inspection'}
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report), flush=True)
    finally:
        (root / 'requests.json').write_text(json.dumps(server.requests, indent=2), encoding='utf-8')
        if rpc: rpc.stop()
        server.shutdown(); server.server_close(); worker.join(2)

if __name__ == '__main__':
    import sys
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
