"""Local downloaded-exe checks against checklist §§2.2, 3.6, 6, 7.

Uses real app-server RPC and RFC6455/SSE loopback requests, never compiles Rust.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import sys
import threading
import time

from test_websocket_recovery import Fixture, command


class Rpc:
    def __init__(self, binary, fixture, home, output, extra=()):
        home.mkdir(parents=True, exist_ok=True)
        output.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env['CODEX_HOME'] = str(home)
        for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL'):
            env.pop(key, None)
        args = command(binary, fixture)
        settings = args[args.index('-c'):-1]
        self.stderr = (output / 'app-server.stderr.txt').open('w', encoding='utf-8')
        self.log = (output / 'rpc.jsonl').open('w', encoding='utf-8')
        self.process = subprocess.Popen([binary, 'app-server', *settings, *extra],
            cwd=home, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            text=True, encoding='utf-8', creationflags=subprocess.CREATE_NO_WINDOW)
        self.queue = queue.Queue()
        self.pending = []
        self.sequence = 0
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()
        self.call('initialize', {'clientInfo': {'name': 'local3_verify', 'version': '1.0'},
                               'capabilities': {'experimentalApi': True}})
        self.send({'method': 'initialized'})

    def read(self):
        for line in self.process.stdout:
            try:
                self.queue.put(json.loads(line))
            except json.JSONDecodeError:
                pass
        self.queue.put({'eof': self.process.poll()})

    def send(self, value):
        self.process.stdin.write(json.dumps(value, ensure_ascii=False) + '\n')
        self.process.stdin.flush()

    def wait(self, predicate, timeout=100):
        deadline = time.monotonic() + timeout
        while True:
            for i, value in enumerate(self.pending):
                if predicate(value):
                    return self.pending.pop(i)
            value = self.queue.get(timeout=max(.01, deadline - time.monotonic()))
            self.log.write(json.dumps(value, ensure_ascii=False) + '\n')
            self.log.flush()
            if 'eof' in value:
                raise RuntimeError(f'app-server exited: {value}')
            if predicate(value):
                return value
            self.pending.append(value)

    def call(self, method, params):
        self.sequence += 1
        request_id = self.sequence
        self.send({'id': request_id, 'method': method, 'params': params})
        response = self.wait(lambda v: v.get('id') == request_id)
        assert 'error' not in response, response
        return response['result']

    def start(self, cwd):
        return self.call('thread/start', {'model': 'gpt-5.4', 'cwd': str(cwd),
            'approvalPolicy': 'never', 'sandbox': 'danger-full-access'})['thread']['id']

    def turn(self, thread_id, text, images=()):
        result = self.call('turn/start', {'threadId': thread_id, 'input': [
            {'type': 'text', 'text': text}, *images]})
        turn_id = result['turn']['id']
        completed = self.wait(lambda v: v.get('method') == 'turn/completed'
            and v['params']['turn']['id'] == turn_id)
        assert completed['params']['turn']['status'] == 'completed', completed
        return completed

    def stop(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait(timeout=10)
        self.reader.join(timeout=3)
        self.log.close()
        self.stderr.close()


def verify(binary, root):
    root.mkdir(parents=True, exist_ok=True)
    fixture = Fixture('healthy')
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    home = root / 'home'
    rpc = None
    try:
        rpc = Rpc(binary, fixture, home, root / 'first')
        thread_id = rpc.start(home)
        rpc.turn(thread_id, '你好')
        first = rpc.call('thread/read', {'threadId': thread_id, 'includeTurns': True})
        rpc.turn(thread_id, '你好')
        second = rpc.call('thread/read', {'threadId': thread_id, 'includeTurns': True})
        def texts(value):
            return [item['text'] for turn in value['thread']['turns'] for item in turn['items']
                    if item.get('type') == 'agentMessage']
        first_text = '\n'.join(texts(first))
        numbers = [int(x) for x in re.findall(r'(?m)^\s*(\d+)\. ', first_text)]
        assert numbers == list(range(1, 22)), ('first hello checklist', numbers)
        assert len(texts(second)) > len(texts(first)), 'second turn must complete'
        assert not re.search(r'(?m)^\s*1\. ', texts(second)[-1]), 'checklist must not repeat'
        rpc.stop()
        rpc = Rpc(binary, fixture, home, root / 'resume')
        rpc.call('thread/resume', {'threadId': thread_id})
        rpc.turn(thread_id, 'Continue with a short reply.')
        restored = rpc.call('thread/read', {'threadId': thread_id, 'includeTurns': True})
        assert len(texts(restored)) > len(texts(second)), 'resume must retain and extend history'
        assert not re.search(r'(?m)^\s*1\. ', texts(restored)[-1])
        assert not any('Reconnecting...' in t for t in texts(restored))
        (root / 'thread-id.txt').write_text(thread_id, encoding='utf-8')
        report = {'passed': True, 'first_hello_21': True, 'second_no_repeat': True,
                  'resume': True, 'thread_id': thread_id}
        (root / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report), flush=True)
    finally:
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(5)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
