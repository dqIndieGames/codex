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
G.CreateCompatibleDC.argtypes = [W.HDC]; G.CreateCompatibleDC.restype = W.HDC
G.CreateDIBSection.argtypes = [W.HDC, C.POINTER(BITMAPINFO), W.UINT, C.POINTER(C.c_void_p), W.HANDLE, W.DWORD]; G.CreateDIBSection.restype = W.HBITMAP
G.SelectObject.argtypes = [W.HDC, W.HANDLE]; G.SelectObject.restype = W.HANDLE
G.DeleteObject.argtypes = [W.HANDLE]; G.DeleteDC.argtypes = [W.HDC]
K.CreateProcessW.argtypes = [W.LPCWSTR, W.LPWSTR, C.c_void_p, C.c_void_p, W.BOOL, W.DWORD, C.c_void_p, W.LPCWSTR, C.POINTER(STARTUPINFO), C.POINTER(PROCESSINFO)]
K.ResumeThread.argtypes = [W.HANDLE]
K.TerminateProcess.argtypes = [W.HANDLE, W.UINT]
K.WaitForSingleObject.argtypes = [W.HANDLE, W.DWORD]
K.CloseHandle.argtypes = [W.HANDLE]

class VisualFixture(Fixture):
    def __init__(self):
        super().__init__('close')
        self.failed, self.third_failure, self.completed = threading.Event(), threading.Event(), threading.Event()
    def record(self, transport, payload, user_agent=None):
        result = super().record(transport, payload, user_agent)
        if transport == 'ws':
            self.failed.set()
            if len(self.requests) == WS_FAILURES_BEFORE_HTTP:
                self.third_failure.set()
        if transport == 'http':
            self.completed.set()
        return result

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
    root.mkdir(parents=True, exist_ok=True)
    server = VisualFixture(); worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    home = root/'home'; home.mkdir(exist_ok=True)
    env = os.environ.copy(); env['CODEX_HOME'] = str(home)
    for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL'): env.pop(key, None)
    args = command(binary, server)
    settings = args[args.index('-c'):-1]
    args = [binary, '--dangerously-bypass-hook-trust', '--dangerously-bypass-approvals-and-sandbox',
        '--no-alt-screen', '-m', 'gpt-5.5', *settings,
        '-c', 'tui.notifications=false',
        '-c', f'projects.{json.dumps(str(home))}.trust_level="trusted"', '你好']
    desktop_name = f'DevVerify_local3_{os.getpid()}'
    desktop = U.CreateDesktopW(desktop_name, None, None, 0, 0x01ff, None)
    if not desktop: raise C.WinError(C.get_last_error())
    previous_desktop = U.GetThreadDesktop(K.GetCurrentThreadId())
    info, process = STARTUPINFO(), PROCESSINFO()
    info.cb, info.lpDesktop, info.lpTitle = C.sizeof(info), desktop_name, 'local3 verification'
    # CREATE_NEW_CONSOLE | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT.
    block = C.create_unicode_buffer('\0'.join(f'{k}={v}' for k, v in sorted(env.items()))+'\0\0')
    try:
        # 显式使用现成 conhost，避免默认终端委托把窗口转交用户当前桌面。
        console_host = str(Path(os.environ['SystemRoot'])/'System32'/'conhost.exe')
        if not K.CreateProcessW(console_host, C.create_unicode_buffer(subprocess.list2cmdline([console_host, *args])), None, None, False,
                0x10 | 0x4 | 0x400, block, str(home), C.byref(info), C.byref(process)):
            raise C.WinError(C.get_last_error())
        K.ResumeThread(process.hThread)
        if not U.SetThreadDesktop(desktop): raise C.WinError(C.get_last_error())
        if not server.failed.wait(20): raise AssertionError('TUI did not reach the real model request')
        found = windows(desktop)
        assert len(found) == 1, ('expected one owned terminal on hidden desktop', found)
        hwnd, title = found[0]
        U.SetWindowPos(hwnd, None, 0, 0, 1200, 800, 0x14)
        time.sleep(.2)
        capture(hwnd, root/'tui-retry.png')
        if not server.third_failure.wait(25): raise AssertionError('TUI did not reach three failed WS attempts')
        time.sleep(.3)
        capture(hwnd, root/'tui-http-switch.png')
        if not server.completed.wait(15): raise AssertionError('TUI retry did not recover')
        time.sleep(.3)
        capture(hwnd, root/'tui-completed.png')
        final_title = windows(desktop)[0][1]
        assert final_title.startswith('[f] '), ('effective fixture provider title', final_title)
        (root/'result.json').write_text(json.dumps({'captured': True, 'visual_passed': False,
            'inspection': 'pending actual image review', 'mode': 'DEV-assisted', 'desktop': desktop_name,
            'no_switch_desktop': True, 'exe': binary, 'pid': process.pid, 'title_before': title, 'title_after': final_title}, indent=2), encoding='utf-8')
    finally:
        if process.hProcess:
            K.TerminateProcess(process.hProcess, 0); K.WaitForSingleObject(process.hProcess, 3000)
            K.CloseHandle(process.hProcess); K.CloseHandle(process.hThread)
        U.SetThreadDesktop(previous_desktop); U.CloseDesktop(desktop)
        server.shutdown(); server.server_close(); worker.join(2)

if __name__ == '__main__': verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve())
