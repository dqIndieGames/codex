"""清单5.5：真实问答输入及提交契约；L1比较提交内容与输入，真实截图另行读图。"""
import json
from pathlib import Path
import sys
import threading
import time
from _local3_hidden_console import HiddenConsole
from test_local3_runtime_refresh import RuntimeFixture
from test_websocket_recovery import command
from verify_local3_hidden_tui import is_user_request

ENTER = (13, '\r', 0)
CTRL_V = (86, '\x16', 8)
SHIFT_INSERT = (45, '\0', 16)


class QuestionFixture(RuntimeFixture):
    def __init__(self, kind):
        super().__init__()
        self.kind = kind
        self.sent = False
        self.answers = []
        self.answered = threading.Event()

    def events(self, payload, compact=False):
        entry = self.local.entry
        if not is_user_request(entry, 'LOCAL3_QUESTION_CHECK'):
            return super().events(payload, compact)
        if not self.sent:
            self.sent = True
            name = 'request_user_input_async' if self.kind == 'async' else 'request_user_input'
            serialized_tools = json.dumps([payload.get('tools', []),
                *[item.get('tools', []) for item in payload.get('input', [])
                  if item.get('type') == 'additional_tools']])
            assert name in serialized_tools, f'{name} missing from real tool capabilities'
            if self.kind == 'async':
                questions = [{'title': f'LOCAL3_QUESTION_{i}', 'options': ['First', 'Second']} for i in ('A', 'B')]
            else:
                questions = [{'id': i.lower(), 'header': i, 'question': f'LOCAL3_QUESTION_{i}',
                    'options': [{'label': 'First', 'description': 'First option'},
                                {'label': 'Second', 'description': 'Second option'}]} for i in ('A', 'B')]
            output = [{'type': 'function_call', 'name': name, 'call_id': 'call_local3_question',
                       'arguments': json.dumps({'questions': questions})}]
            return [{'type': 'response.created', 'response': {'id': 'question-response'}},
                    {'type': 'response.output_item.done', 'output_index': 0, 'item': output[0]},
                    {'type': 'response.completed', 'response': {'id': 'question-response', 'output': output,
                     'status': 'completed', 'usage': {'input_tokens': 10, 'output_tokens': 10, 'total_tokens': 20}}}]
        if self.kind == 'async' and not self.release.is_set():
            # 保持发问回合运行，避免真实产品在回合结束时回收尚未回答的问题。
            # 回答经真实队列提交后由驱动释放本次响应，不改变产品状态。
            self.release.wait(20)
        for item in payload.get('input', []):
            if self.kind == 'blocking' and item.get('type') == 'function_call_output' and item.get('call_id') == 'call_local3_question':
                self.answers.append(item['output'])
                self.answered.set()
            if self.kind == 'async' and item.get('role') == 'user':
                for part in item.get('content', []):
                    if '<send_user_message_question_reply>' in part.get('text', ''):
                        self.answers.append(part['text'])
                        self.answered.set()
        return super().events(payload, compact)


def verify(binary, root, kinds):
    root.mkdir(parents=True, exist_ok=True)
    results = []
    for kind in kinds:
        output = root/kind; output.mkdir(exist_ok=True)
        home = output/'home'; home.mkdir(exist_ok=True)
        (home/'config.toml').write_text('model = "gpt-6-sol"\n', encoding='utf-8')
        server = QuestionFixture(kind)
        worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
        base = command(binary, server); settings = base[base.index('-c'):-1]
        args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox',
                '--no-alt-screen', '-m', 'gpt-6-sol', *settings,
                '-c', 'tui.notifications=false', '-c', 'model_providers.fixture.supports_websockets=false',
                '-c', f'projects.{json.dumps(str(home))}.trust_level="trusted"']
        console = None
        try:
            console = HiddenConsole(args, home)
            console.wait_text('GPT-6-Sol')
            if kind == 'blocking':
                console.type('/plan'); time.sleep(.2); console.keys([ENTER]); time.sleep(.2)
            console.type('Ask the two fixture questions. LOCAL3_QUESTION_CHECK')
            time.sleep(.2); console.keys([ENTER])
            if kind == 'async':
                # 展开异步回答面板，使用产品默认 Shift+Left。
                deadline = time.monotonic()+8
                while not server.sent and time.monotonic() < deadline:
                    time.sleep(.05)
                assert server.sent, 'async tool call was not sent'
                time.sleep(.3); console.keys([(37, '\0', 16)])
            console.wait_text('LOCAL3_QUESTION_A')
            # 先打开 Other。输入与紧随的粘贴验证缓冲文字顺序。
            console.keys([(40, '\0', 0), (40, '\0', 0)])
            if kind == 'blocking':
                console.keys([ENTER])
            console.type('prefix-')
            first = '中文第一行\nsecond line'
            console.clipboard(first); console.keys([CTRL_V])
            console.wait_text('second line')
            console.clipboard('-suffix'); console.keys([SHIFT_INSERT])
            console.wait_text('-suffix')
            assert not server.answered.is_set(), 'pasting must not submit answers'
            console.screenshot(output/'answer-a.png')
            # 同批注入粘贴和切题。读取可能先完成并合法写入 A，也可能被取消；
            # 两种时序都不得把 A 的粘贴写入 B，最终按题核对提交内容。
            console.clipboard('STALE_CLIPBOARD_MUST_NOT_APPEAR')
            next_key = (34, '\0', 0) if kind == 'blocking' else (37, '\0', 16)
            console.keys([CTRL_V, next_key])
            console.wait_text('LOCAL3_QUESTION_B')
            time.sleep(.2)
            assert 'STALE_CLIPBOARD_MUST_NOT_APPEAR' not in console.text()
            console.keys([(40, '\0', 0), (40, '\0', 0)])
            if kind == 'blocking':
                console.keys([ENTER])
            console.clipboard('answer-B'); console.keys([CTRL_V])
            console.wait_text('answer-B')
            console.screenshot(output/'answer-b.png')
            previous_key = (33, '\0', 0) if kind == 'blocking' else (39, '\0', 16)
            console.keys([previous_key]); time.sleep(.15)
            console.keys([ENTER]); time.sleep(.2)
            console.keys([ENTER])
            server.release.set()
            assert server.answered.wait(5), 'answer was not delivered to the model request'
            if kind == 'async':
                deadline = time.monotonic()+5
                while 'answer-B' not in '\n'.join(server.answers) and time.monotonic() < deadline:
                    time.sleep(.05)
            if kind == 'blocking':
                submitted = json.loads(server.answers[-1])['answers']
                answer_a = submitted['a']['answers'][-1].removeprefix('user_note: ')
                answer_b = submitted['b']['answers'][-1].removeprefix('user_note: ')
            else:
                submitted = {}
                for response in server.answers:
                    content = response.split('<send_user_message_question_reply>', 1)[1].split('</send_user_message_question_reply>', 1)[0]
                    for answer in json.loads(content):
                        submitted[answer['question']] = answer['answer']
                answer_a = submitted['LOCAL3_QUESTION_A']
                answer_b = submitted['LOCAL3_QUESTION_B']
            expected_a = 'prefix-'+first+'-suffix'
            assert answer_a in (expected_a, expected_a+'STALE_CLIPBOARD_MUST_NOT_APPEAR'), 'A paste content/order changed'
            assert answer_b == 'answer-B', 'A paste leaked into B or B answer changed'
            (output/'answers.json').write_text(json.dumps(server.answers, ensure_ascii=False, indent=2), encoding='utf-8')
            results.append({'kind': kind, 'passed': True, 'clipboard_restored_unless_externally_changed': True,
                            'paste_completed_before_navigation': answer_a != expected_a,
                            'visual_status': 'pending actual inspection'})
        except Exception:
            if console:
                console.screenshot(output/'failure.png')
                (output/'screen.txt').write_text(console.text(), encoding='utf-8')
            raise
        finally:
            server.release.set()
            if console:
                console.close()
            server.shutdown(); server.server_close(); worker.join(2)
            (output/'requests.json').write_text(json.dumps(server.requests, ensure_ascii=False, indent=2), encoding='utf-8')
    (root/'result.json').write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding='utf-8')
    print('PASS question clipboard input and submitted answers', flush=True)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve(), sys.argv[3:] or ['blocking', 'async'])
