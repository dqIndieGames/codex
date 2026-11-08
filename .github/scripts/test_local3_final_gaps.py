"""补验真实 exe 的迟到粘贴、终端粘贴及关闭隔离；不修改产品实现。"""
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import sys
import threading
import time
import win32clipboard as clip
import win32gui as gui
from _local3_hidden_console import HiddenConsole, U, K
from test_local3_question_paste import QuestionFixture, CTRL_V, ENTER
from test_websocket_recovery import command


class DelayedClipboard:
    """用 Windows 延迟呈现协议阻塞真实读取，事件确认后才推进 UI。"""
    def __init__(self, console, text):
        self.console, self.text = console, text
        self.started, self.release, self.done, self.ready = (threading.Event() for _ in range(4))
        self.error = None
        console.clipboard(text)
        if not hasattr(console, 'delayed_owners'):
            console.delayed_owners = []
        console.delayed_owners.append(self)
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()
        assert self.ready.wait(2), 'delayed clipboard owner not ready'
        if self.error:
            raise self.error

    def run(self):
        U.SetThreadDesktop(self.console.desktop)
        def procedure(hwnd, message, wp, lp):
            if message == 0x305:  # WM_RENDERFORMAT: consumer has entered GetClipboardData.
                self.started.set()
                if not self.release.wait(3):
                    self.error = AssertionError('delayed clipboard gate timed out')
                clip.SetClipboardText(self.text, 13)
                self.done.set()
                return 0
            if message == 0x10:
                gui.DestroyWindow(hwnd)
                return 0
            if message == 2:
                gui.PostQuitMessage(0)
                return 0
            return gui.DefWindowProc(hwnd, message, wp, lp)
        name = f'Local3DelayedClipboard_{os.getpid()}_{time.monotonic_ns()}'
        wc = gui.WNDCLASS(); wc.lpszClassName = name; wc.lpfnWndProc = procedure
        try:
            gui.RegisterClass(wc)
            self.hwnd = gui.CreateWindow(name, '', 0, 0, 0, 0, 0, 0, 0, 0, None)
            clip.OpenClipboard(self.hwnd)
            try:
                clip.EmptyClipboard()
                U.SetClipboardData.argtypes = [W.UINT, W.HANDLE]
                U.SetClipboardData.restype = W.HANDLE
                U.SetClipboardData(13, None)
            finally:
                clip.CloseClipboard()
            self.console.clipboard_sequence = clip.GetClipboardSequenceNumber()
            self.ready.set()
            gui.PumpMessages()
        except Exception as exc:
            self.error = exc
            self.ready.set(); self.done.set()
        finally:
            gui.UnregisterClass(name, None)

    def finish(self):
        self.release.set()
        assert self.done.wait(2), 'delayed clipboard was not consumed'
        if self.error:
            raise self.error
        # Delayed rendering is our own update. Do not adopt unrelated clipboard owners.
        assert clip.GetClipboardOwner() == self.hwnd, 'clipboard ownership changed'
        # GetClipboardData may synthesize formats before closing its handle.
        deadline = time.monotonic()+.5
        while True:
            try:
                clip.OpenClipboard(self.hwnd)
                clip.CloseClipboard()
                break
            except Exception:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(.01)
        assert clip.GetClipboardOwner() == self.hwnd
        self.console.clipboard_sequence = clip.GetClipboardSequenceNumber()


def terminal_paste(console, text):
    # Win32 clipboard CF_UNICODETEXT uses CRLF; conhost also consults its input code page.
    assert K.SetConsoleCP(65001) and K.SetConsoleOutputCP(65001)
    console.clipboard(text.replace('\n', '\r\n'))
    U.GetMenuStringW.argtypes = [W.HMENU, W.UINT, W.LPWSTR, C.c_int, W.UINT]
    def find(menu):
        for i in range(gui.GetMenuItemCount(menu)):
            label = C.create_unicode_buffer(256)
            U.GetMenuStringW(menu, i, label, len(label), 0x400)
            title = label.value
            if 'paste' in title.lower() or '粘贴' in title:
                return gui.GetMenuItemID(menu, i)
            submenu = gui.GetSubMenu(menu, i)
            if submenu:
                result = find(submenu)
                if result is not None:
                    return result
        return None
    item = find(gui.GetSystemMenu(console.hwnd, False))
    assert item is not None, 'terminal Paste menu unavailable'
    gui.PostMessage(console.hwnd, 0x112, item, 0)


def verify(binary, root, kinds, menu_paste=False):
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
        console = gate = None
        try:
            console = HiddenConsole(args, home)
            console.wait_text('GPT-6-Sol')
            if kind == 'blocking':
                console.type('/plan'); time.sleep(.15); console.keys([ENTER]); time.sleep(.15)
            console.type('Ask the two fixture questions. LOCAL3_QUESTION_CHECK')
            time.sleep(.3); console.keys([ENTER])
            assert server.answered.is_set() is False
            if kind == 'async':
                deadline = time.monotonic()+5
                while not server.sent and time.monotonic() < deadline:
                    time.sleep(.02)
                assert server.sent
                time.sleep(.2); console.keys([(37, '\0', 16)])
            console.wait_text('LOCAL3_QUESTION_A')
            console.keys([(40, '\0', 0), (40, '\0', 0)])
            if kind == 'blocking':
                console.keys([ENTER])
            gate = DelayedClipboard(console, 'LATE_FROM_A')
            console.keys([CTRL_V])
            assert gate.started.wait(2), 'real clipboard reader never entered delayed format'
            next_key = (34, '\0', 0) if kind == 'blocking' else (37, '\0', 16)
            console.keys([next_key]); console.wait_text('LOCAL3_QUESTION_B')
            gate.finish(); gate = None
            time.sleep(.15)
            assert 'LATE_FROM_A' not in console.text()
            console.keys([(40, '\0', 0), (40, '\0', 0)])
            if kind == 'blocking':
                console.keys([ENTER])
            expected = '终端粘贴第一行\nterminal-paste-B'
            if menu_paste:
                terminal_paste(console, expected)
            else:
                console.clipboard(expected)
                console.keys([CTRL_V])
            console.wait_text('terminal-paste-B')
            assert not server.answered.is_set(), 'terminal paste auto-submitted'
            console.screenshot(output/('terminal-paste.png' if menu_paste else 'answer-b.png'))
            # 已填 B，返回 A 填写独立内容；按实际提交结果检查不变量。
            previous = (33, '\0', 0) if kind == 'blocking' else (39, '\0', 16)
            console.keys([previous]); console.wait_text('LOCAL3_QUESTION_A')
            console.clipboard('answer-A'); console.keys([CTRL_V]); console.wait_text('answer-A')
            console.keys([ENTER]); time.sleep(.15)
            console.keys([ENTER]); server.release.set()
            assert server.answered.wait(4), 'real submitted answers missing'
            deadline = time.monotonic()+3
            while 'terminal-paste-B' not in '\n'.join(server.answers) and time.monotonic()<deadline:
                time.sleep(.03)
            if kind == 'blocking':
                submitted = json.loads(server.answers[-1])['answers']
                a = submitted['a']['answers'][-1].removeprefix('user_note: ')
                b = submitted['b']['answers'][-1].removeprefix('user_note: ')
            else:
                submitted = {}
                for response in server.answers:
                    body = response.split('<send_user_message_question_reply>', 1)[1].split('</send_user_message_question_reply>', 1)[0]
                    for answer in json.loads(body):
                        submitted[answer['question']] = answer['answer']
                a, b = submitted['LOCAL3_QUESTION_A'], submitted['LOCAL3_QUESTION_B']
            assert a == 'answer-A' and b == expected, (a, b)
            # 新问题用于在读取过程中关闭问答并切换线程。
            console.wait_text('WS_RECOVERY_OK')
            server.sent = False; server.release.clear()
            original_events = server.events
            def close_events(payload, compact=False):
                events = original_events(payload, compact)
                for event in events:
                    item = event.get('item')
                    if item and item.get('type') == 'function_call':
                        item['call_id'] = 'call_close_question'
                        item['arguments'] = item['arguments'].replace('LOCAL3_QUESTION_', 'CLOSE_QUESTION_')
                return events
            server.events = close_events
            console.type('Ask again. LOCAL3_QUESTION_CHECK'); time.sleep(.15); console.keys([ENTER])
            deadline=time.monotonic()+4
            while not server.sent and time.monotonic()<deadline: time.sleep(.02)
            assert server.sent, 'second question was not sent'
            if kind == 'async':
                time.sleep(.2); console.keys([(37, '\0', 16)])
            console.wait_text('CLOSE_QUESTION_A')
            console.keys([(40, '\0', 0), (40, '\0', 0)])
            if kind == 'blocking': console.keys([ENTER])
            gate = DelayedClipboard(console, 'LATE_FROM_CLOSED_QUESTION')
            console.keys([CTRL_V]); assert gate.started.wait(2)
            # Ctrl+C closes empty blocking editor; Escape interrupts the async turn.
            console.keys([(67, '\x03', 8)] if kind == 'blocking' else [(27, '\x1b', 0)])
            server.release.set()
            deadline=time.monotonic()+1.5
            while 'CLOSE_QUESTION_A' in console.text() and time.monotonic()<deadline: time.sleep(.02)
            console.type('/new'); time.sleep(.15); console.keys([ENTER]); time.sleep(.2)
            gate.finish(); gate = None
            console.type('AFTER_CLOSE_SENTINEL'); time.sleep(.2)
            screen=console.text()
            assert 'AFTER_CLOSE_SENTINEL' in screen and 'LATE_FROM_CLOSED_QUESTION' not in screen
            console.screenshot(output/'after-close-new.png')
            console.keys([ENTER])
            console.wait_text('WS_RECOVERY_OK')
            # 会话 ID 变化以真实 rollout 元数据证明，不能只看 /new 输入。
            ids=set()
            for path in (home/'sessions').rglob('*.jsonl'):
                with path.open(encoding='utf-8') as stream:
                    first=json.loads(stream.readline())
                if first.get('type')=='session_meta': ids.add(first['payload']['id'])
            assert len(ids)>=2, 'new thread was not created'
            (output/'answers.json').write_text(json.dumps(server.answers, ensure_ascii=False, indent=2), encoding='utf-8')
            results.append({'kind':kind,'delayed_read_confirmed':True,'terminal_paste_submitted':menu_paste,
                            'close_and_new_thread_isolated':True,'thread_ids':sorted(ids),'visual_status':'pending inspection'})
            print('PASS', kind, flush=True)
        except Exception:
            if console:
                console.screenshot(output/'failure.png')
                (output/'screen.txt').write_text(console.text(), encoding='utf-8')
            raise
        finally:
            if gate:
                gate.release.set()
                if gate.started.is_set(): gate.finish()
            server.release.set()
            if console:
                console.close()
                for owner in getattr(console, 'delayed_owners', []):
                    gui.PostMessage(owner.hwnd, 0x10, 0, 0)
                    owner.thread.join(2)
            server.shutdown(); server.server_close(); worker.join(2)
            (root/'result.json').write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__=='__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)),Path(sys.argv[2]).resolve(),
           [arg for arg in sys.argv[3:] if arg != 'menu-paste'] or ['blocking','async'], 'menu-paste' in sys.argv[3:])
