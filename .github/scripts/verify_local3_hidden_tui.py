"""真实 TUI 隐藏桌面取证；仅 ctypes 调用现成 Win32 API，不编译辅助程序。"""
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time
from PIL import Image
from test_websocket_recovery import Fixture, command
from test_local3_runtime_refresh import RuntimeFixture
from _groundtruth.local3_retry_contract import WS_FAILURES_BEFORE_HTTP

U, K, G = C.WinDLL('user32', use_last_error=True), C.WinDLL('kernel32', use_last_error=True), C.WinDLL('gdi32', use_last_error=True)
class STARTUPINFO(C.Structure):
    _fields_ = [('cb', W.DWORD), ('lpReserved', W.LPWSTR), ('lpDesktop', W.LPWSTR), ('lpTitle', W.LPWSTR),
        ('dwX', W.DWORD), ('dwY', W.DWORD), ('dwXSize', W.DWORD), ('dwYSize', W.DWORD),
        ('dwXCountChars', W.DWORD), ('dwYCountChars', W.DWORD), ('dwFillAttribute', W.DWORD),
        ('dwFlags', W.DWORD), ('wShowWindow', W.WORD), ('cbReserved2', W.WORD), ('lpReserved2', C.c_void_p),
        ('hStdInput', W.HANDLE), ('hStdOutput', W.HANDLE), ('hStdError', W.HANDLE)]
class PROCESSINFO(C.Structure):
    _fields_ = [('hProcess', W.HANDLE), ('hThread', W.HANDLE), ('pid', W.DWORD), ('tid', W.DWORD)]
class BITMAPINFO(C.Structure):
    _fields_ = [('size', W.DWORD), ('width', W.LONG), ('height', W.LONG), ('planes', W.WORD), ('bits', W.WORD),
        ('compression', W.DWORD), ('imageSize', W.DWORD), ('xppm', W.LONG), ('yppm', W.LONG),
        ('used', W.DWORD), ('important', W.DWORD), ('colors', W.DWORD*3)]
CALLBACK = C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
U.CreateDesktopW.argtypes = [W.LPCWSTR, W.LPCWSTR, C.c_void_p, W.DWORD, W.DWORD, C.c_void_p]; U.CreateDesktopW.restype = W.HANDLE
U.GetThreadDesktop.argtypes = [W.DWORD]; U.GetThreadDesktop.restype = W.HANDLE
U.SetThreadDesktop.argtypes = [W.HANDLE]; U.SetThreadDesktop.restype = W.BOOL
U.CloseDesktop.argtypes = [W.HANDLE]
U.EnumDesktopWindows.argtypes = [W.HANDLE, CALLBACK, W.LPARAM]
U.GetWindowTextW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
U.IsWindowVisible.argtypes = [W.HWND]
U.GetWindowRect.argtypes = [W.HWND, C.POINTER(W.RECT)]
U.SetWindowPos.argtypes = [W.HWND, W.HWND, C.c_int, C.c_int, C.c_int, C.c_int, W.UINT]
U.GetDC.argtypes = [W.HWND]; U.GetDC.restype = W.HDC
U.ReleaseDC.argtypes = [W.HWND, W.HDC]
U.PrintWindow.argtypes = [W.HWND, W.HDC, W.UINT]; U.PrintWindow.restype = W.BOOL
U.RedrawWindow.argtypes = [W.HWND, C.c_void_p, W.HANDLE, W.UINT]
G.CreateCompatibleDC.argtypes = [W.HDC]; G.CreateCompatibleDC.restype = W.HDC
G.CreateDIBSection.argtypes = [W.HDC, C.POINTER(BITMAPINFO), W.UINT, C.POINTER(C.c_void_p), W.HANDLE, W.DWORD]; G.CreateDIBSection.restype = W.HBITMAP
G.SelectObject.argtypes = [W.HDC, W.HANDLE]; G.SelectObject.restype = W.HANDLE
G.DeleteObject.argtypes = [W.HANDLE]; G.DeleteDC.argtypes = [W.HDC]
K.CreateProcessW.argtypes = [W.LPCWSTR, W.LPWSTR, C.c_void_p, C.c_void_p, W.BOOL, W.DWORD, C.c_void_p, W.LPCWSTR, C.POINTER(STARTUPINFO), C.POINTER(PROCESSINFO)]
K.ResumeThread.argtypes = [W.HANDLE]
K.TerminateProcess.argtypes = [W.HANDLE, W.UINT]
K.WaitForSingleObject.argtypes = [W.HANDLE, W.DWORD]
K.CloseHandle.argtypes = [W.HANDLE]

def is_user_request(entry, marker):
    headers = {key.lower(): value for key, value in entry.get('headers', {}).items()}
    metadata = json.loads(headers.get('x-codex-turn-metadata', '{}'))
    return metadata.get('thread_source') == 'user' and marker in json.dumps(entry['payload'], ensure_ascii=False)


def prepare_window(hwnd):
    U.SetWindowPos(hwnd, None, 0, 0, 1200, 800, 0x14)
    # 先让终端处理尺寸变化，再要求真实窗口完成绘制；不拼接或重画截图。
    time.sleep(.3)
    U.RedrawWindow(hwnd, None, None, 0x1 | 0x4 | 0x80 | 0x100 | 0x400)
    time.sleep(.2)


class VisualFixture(RuntimeFixture):
    def __init__(self):
        super().__init__()
        self.failed, self.third_failure, self.completed = threading.Event(), threading.Event(), threading.Event()
        self.target_requests = []

    def events(self, payload, compact=False):
        entry = self.local.entry
        if not is_user_request(entry, 'LOCAL3_VISUAL_RECOVERY'):
            return super().events(payload, compact)
        self.target_requests.append(entry)
        if entry['transport'] == 'ws' and len(self.failures) < WS_FAILURES_BEFORE_HTTP:
            self.failures.append(time.monotonic())
            self.failed.set()
            if len(self.failures) == WS_FAILURES_BEFORE_HTTP:
                self.third_failure.set()
            return [{'type': 'response.failed', 'response': {'error': {
                'code': 'server_error', 'message': 'fixture user request failure',
                'headers': {'Retry-After': '2'}}}}]
        if entry['transport'] == 'http':
            self.completed.set()
        return super().events(payload, compact)

def windows(desktop):
    found = []
    @CALLBACK
    def callback(hwnd, _):
        title = C.create_unicode_buffer(2048)
        U.GetWindowTextW(hwnd, title, len(title))
        if title.value and U.IsWindowVisible(hwnd): found.append((hwnd, title.value))
        return True
    U.EnumDesktopWindows(desktop, callback, 0)
    return found

def capture(hwnd, target):
    rect = W.RECT(); U.GetWindowRect(hwnd, C.byref(rect))
    width, height = rect.right-rect.left, rect.bottom-rect.top
    source = U.GetDC(hwnd); dc = G.CreateCompatibleDC(source)
    bits = C.c_void_p()
    info = BITMAPINFO(); info.size = 40; info.width = width; info.height = -height; info.planes = 1; info.bits = 32
    bitmap = G.CreateDIBSection(dc, C.byref(info), 0, C.byref(bits), None, 0)
    previous = G.SelectObject(dc, bitmap)
    try:
        if not U.PrintWindow(hwnd, dc, 2): raise C.WinError(C.get_last_error())
        picture = Image.frombytes('RGB', (width, height), C.string_at(bits, width*height*4), 'raw', 'BGRX')
        picture.save(target)
        assert max(hi-lo for lo, hi in picture.getextrema()) > 0, 'blank terminal capture'
    finally:
        G.SelectObject(dc, previous); G.DeleteObject(bitmap); G.DeleteDC(dc); U.ReleaseDC(hwnd, source)

def verify(binary, root):
    from _local3_hidden_console import HiddenConsole, SCREEN
    root.mkdir(parents=True, exist_ok=True)
    server = VisualFixture(); worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    home = root/'home'; home.mkdir(exist_ok=True)
    (home/'config.toml').write_text('[notice.model_migrations]\n"gpt-5.5" = "gpt-6-sol"\n', encoding='utf-8')
    args = command(binary, server)
    settings = args[args.index('-c'):-1]
    args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox',
        '-m', 'gpt-5.5', *settings,
        '-c', 'tui.notifications=false',
        '-c', f'projects.{json.dumps(str(home))}.trust_level="trusted"', 'Reply briefly. LOCAL3_VISUAL_RECOVERY']
    console = None
    try:
        console = HiddenConsole(args, home)
        process, desktop = console.process, console.desktop
        deadline = time.monotonic() + 20
        while not server.failed.wait(.05) and time.monotonic() < deadline:
            if K.WaitForSingleObject(process.hProcess, 0) == 0:
                raise AssertionError('TUI process exited before the first request')
        if not server.failed.is_set():
            for index, (hwnd, title) in enumerate(windows(desktop)):
                capture(hwnd, root/f'tui-startup-failure-{index}.png')
                print(title, flush=True)
            raise AssertionError('TUI did not reach the real model request')
        found = windows(desktop)
        assert len(found) == 1, ('expected one owned terminal on hidden desktop', found)
        hwnd, title = found[0]
        prepare_window(hwnd)
        capture(hwnd, root/'tui-retry.png')
        if not server.third_failure.wait(25): raise AssertionError('TUI did not reach three failed WS attempts')
        time.sleep(.3)
        capture(hwnd, root/'tui-http-switch.png')
        if not server.completed.wait(15): raise AssertionError('TUI retry did not recover')
        time.sleep(.3)
        capture(hwnd, root/'tui-completed.png')
        assert [entry['transport'] for entry in server.target_requests] == ['ws'] * WS_FAILURES_BEFORE_HTTP + ['http']
        (root/'requests.json').write_text(json.dumps(server.target_requests, ensure_ascii=False, indent=2), encoding='utf-8')
        final_title = windows(desktop)[0][1]
        assert final_title.startswith('[f] '), ('effective fixture provider title', final_title)
        screen = SCREEN()
        K.GetConsoleScreenBufferInfo(console.output, C.byref(screen))
        (root/'result.json').write_text(json.dumps({'captured': True, 'visual_passed': False,
            'inspection': 'pending actual image review', 'mode': 'DEV-assisted',
            'columns': screen.size.x, 'viewport_columns': screen.window.right-screen.window.left+1,
            'no_switch_desktop': True, 'exe': binary, 'pid': process.pid, 'title_before': title, 'title_after': final_title}, indent=2), encoding='utf-8')
    finally:
        if console:
            console.close()
        server.shutdown(); server.server_close(); worker.join(2)

if __name__ == '__main__': verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
