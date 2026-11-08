"""真实终端输入驱动：隐藏桌面、原样归还剪贴板；不调用编译器或前台输入。"""
import ctypes as C
from ctypes import wintypes as W
import os
from pathlib import Path
import subprocess
import time
import win32clipboard as clipboard
from verify_local3_hidden_tui import U, K, STARTUPINFO, PROCESSINFO, windows, capture, prepare_window


class COORD(C.Structure):
    _fields_ = [('x', W.SHORT), ('y', W.SHORT)]


class SMALL_RECT(C.Structure):
    _fields_ = [('left', W.SHORT), ('top', W.SHORT), ('right', W.SHORT), ('bottom', W.SHORT)]


class SCREEN(C.Structure):
    _fields_ = [('size', COORD), ('cursor', COORD), ('attributes', W.WORD),
                ('window', SMALL_RECT), ('maximum', COORD)]


class FONT(C.Structure):
    _fields_ = [('size', W.ULONG), ('index', W.DWORD), ('cell', COORD),
                ('family', W.UINT), ('weight', W.UINT), ('face', W.WCHAR * 32)]


class KEY(C.Structure):
    _fields_ = [('down', W.BOOL), ('repeat', W.WORD), ('vk', W.WORD),
                ('scan', W.WORD), ('char', W.WCHAR), ('control', W.DWORD)]


class EVENT(C.Union):
    _fields_ = [('key', KEY), ('padding', C.c_byte * 16)]


class INPUT(C.Structure):
    _fields_ = [('type', W.WORD), ('event', EVENT)]


class PROCESS_ENTRY(C.Structure):
    _fields_ = [('size', W.DWORD), ('usage', W.DWORD), ('pid', W.DWORD),
                ('heap', C.c_size_t), ('module', W.DWORD), ('threads', W.DWORD),
                ('parent', W.DWORD), ('priority', W.LONG), ('flags', W.DWORD),
                ('exe', W.WCHAR * 260)]


K.CreateToolhelp32Snapshot.argtypes = [W.DWORD, W.DWORD]; K.CreateToolhelp32Snapshot.restype = W.HANDLE
K.Process32FirstW.argtypes = [W.HANDLE, C.POINTER(PROCESS_ENTRY)]
K.Process32NextW.argtypes = [W.HANDLE, C.POINTER(PROCESS_ENTRY)]
K.AttachConsole.argtypes = [W.DWORD]
K.CreateFileW.argtypes = [W.LPCWSTR, W.DWORD, W.DWORD, C.c_void_p, W.DWORD, W.DWORD, W.HANDLE]
K.CreateFileW.restype = W.HANDLE
K.WriteConsoleInputW.argtypes = [W.HANDLE, C.POINTER(INPUT), W.DWORD, C.POINTER(W.DWORD)]
K.ReadConsoleOutputCharacterW.argtypes = [W.HANDLE, W.LPWSTR, W.DWORD, COORD, C.POINTER(W.DWORD)]
K.GetConsoleScreenBufferInfo.argtypes = [W.HANDLE, C.POINTER(SCREEN)]
K.SetConsoleWindowInfo.argtypes = [W.HANDLE, W.BOOL, C.POINTER(SMALL_RECT)]
K.SetConsoleScreenBufferSize.argtypes = [W.HANDLE, COORD]
K.GetCurrentConsoleFontEx.argtypes = [W.HANDLE, W.BOOL, C.POINTER(FONT)]
K.SetCurrentConsoleFontEx.argtypes = [W.HANDLE, W.BOOL, C.POINTER(FONT)]


class HiddenConsole:
    def __init__(self, args, home):
        self.process = PROCESSINFO()
        self.input = self.output = None
        self.previous_desktop = U.GetThreadDesktop(K.GetCurrentThreadId())
        name = f'Local3Question_{os.getpid()}_{time.monotonic_ns()}'
        self.clipboard_saved = None
        self.clipboard_sequence = None
        self.desktop = U.CreateDesktopW(name, None, None, 0, 0x1ff, None)
        if not self.desktop or not U.SetThreadDesktop(self.desktop):
            raise C.WinError(C.get_last_error())
        env = os.environ.copy(); env['CODEX_HOME'] = str(home)
        for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'CODEX_INTERNAL_AUTH_ACCOUNT'):
            env.pop(key, None)
        block = C.create_unicode_buffer('\0'.join(f'{k}={v}' for k, v in sorted(env.items()))+'\0\0')
        info = STARTUPINFO(); info.cb = C.sizeof(info); info.lpDesktop = name
        host = str(Path(os.environ['SystemRoot'])/'System32/conhost.exe')
        if not K.CreateProcessW(host, C.create_unicode_buffer(subprocess.list2cmdline([host, *args])),
                None, None, False, 0x10 | 0x4 | 0x400, block, str(home), C.byref(info), C.byref(self.process)):
            raise C.WinError(C.get_last_error())
        K.ResumeThread(self.process.hThread)
        K.FreeConsole()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            snapshot = K.CreateToolhelp32Snapshot(2, 0)
            entry = PROCESS_ENTRY(); entry.size = C.sizeof(entry)
            more = K.Process32FirstW(snapshot, C.byref(entry))
            found = []
            while more:
                if entry.parent == self.process.pid and entry.exe.lower() == 'codex.exe':
                    found.append(entry.pid)
                more = K.Process32NextW(snapshot, C.byref(entry))
            K.CloseHandle(snapshot)
            if found and K.AttachConsole(found[0]):
                break
            time.sleep(.05)
        else:
            raise AssertionError('cannot attach to owned hidden console')
        self.input = K.CreateFileW('CONIN$', 0xc0000000, 3, None, 3, 0, None)
        self.output = K.CreateFileW('CONOUT$', 0xc0000000, 3, None, 3, 0, None)
        font = FONT(); font.size = C.sizeof(font)
        if not K.GetCurrentConsoleFontEx(self.output, False, C.byref(font)):
            raise C.WinError(C.get_last_error())
        font.face = 'Consolas'; font.cell = COORD(0, 16); font.family = 54; font.weight = 400
        if not K.SetCurrentConsoleFontEx(self.output, False, C.byref(font)):
            raise C.WinError(C.get_last_error())
        while time.monotonic() < deadline:
            found = windows(self.desktop)
            if found:
                self.hwnd = found[0][0]
                break
            time.sleep(.05)
        else:
            raise AssertionError('owned terminal window missing')
        prepare_window(self.hwnd)

    def text(self):
        info = SCREEN()
        if not K.GetConsoleScreenBufferInfo(self.output, C.byref(info)):
            raise C.WinError(C.get_last_error())
        count = info.size.x * (info.window.bottom-info.window.top+1)
        buffer = C.create_unicode_buffer(count+1); read = W.DWORD()
        if not K.ReadConsoleOutputCharacterW(self.output, buffer, count,
                COORD(0, info.window.top), C.byref(read)):
            raise C.WinError(C.get_last_error())
        return buffer[:read.value]

    def wait_text(self, marker, timeout=8):
        deadline = time.monotonic()+timeout
        while time.monotonic() < deadline:
            text = self.text()
            if marker in text:
                return text
            if K.WaitForSingleObject(self.process.hProcess, 0) == 0:
                raise AssertionError('owned TUI exited')
            time.sleep(.05)
        raise AssertionError(f'console marker missing: {marker}; screen={self.text()}')

    def keys(self, keys):
        records = []
        for vk, char, modifiers in keys:
            for down in (True, False):
                records.append(INPUT(1, EVENT(key=KEY(down, 1, vk, 0, char, modifiers))))
        array = (INPUT * len(records))(*records); written = W.DWORD()
        if not K.WriteConsoleInputW(self.input, array, len(array), C.byref(written)):
            raise C.WinError(C.get_last_error())
        assert written.value == len(records)

    def type(self, text):
        self.keys([(0, char, 0) for char in text])

    def clipboard(self, text):
        clipboard.OpenClipboard(self.hwnd)
        try:
            if self.clipboard_saved is None:
                saved, kind = [], clipboard.EnumClipboardFormats(0)
                while kind:
                    value = clipboard.GetClipboardData(kind)
                    if not isinstance(value, (bytes, str)):
                        raise RuntimeError('clipboard has a non-copyable format; left unchanged')
                    saved.append((kind, value))
                    kind = clipboard.EnumClipboardFormats(kind)
                self.clipboard_saved = saved
            elif clipboard.GetClipboardSequenceNumber() != self.clipboard_sequence:
                raise RuntimeError('clipboard changed outside this test; left unchanged')
            clipboard.EmptyClipboard()
            clipboard.SetClipboardText(text, 13)
        finally:
            clipboard.CloseClipboard()
        self.clipboard_sequence = clipboard.GetClipboardSequenceNumber()

    def screenshot(self, path):
        U.RedrawWindow(self.hwnd, None, None, 0x1 | 0x80 | 0x100 | 0x400)
        time.sleep(.1)
        capture(self.hwnd, path)

    def close(self):
        if self.clipboard_saved is not None:
            clipboard.OpenClipboard(self.hwnd)
            try:
                if clipboard.GetClipboardSequenceNumber() == self.clipboard_sequence:
                    clipboard.EmptyClipboard()
                    for kind, value in self.clipboard_saved:
                        clipboard.SetClipboardData(kind, value)
            finally:
                clipboard.CloseClipboard()
        for handle in (self.input, self.output):
            if handle:
                K.CloseHandle(handle)
        K.FreeConsole()
        if self.process.hProcess:
            subprocess.run(['taskkill', '/PID', str(self.process.pid), '/T', '/F'],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NO_WINDOW, timeout=5)
            K.WaitForSingleObject(self.process.hProcess, 3000)
            K.CloseHandle(self.process.hProcess); K.CloseHandle(self.process.hThread)
        U.SetThreadDesktop(self.previous_desktop)
        U.CloseDesktop(self.desktop)
