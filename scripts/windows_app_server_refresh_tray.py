from __future__ import annotations

import ctypes
import datetime as dt
import json
import os
from ctypes import wintypes
from pathlib import Path
import threading
from typing import Any
from typing import Callable

APP_SERVERS_DIR_NAME = "app_servers"
DEFAULT_PIPE_TIMEOUT_MS = 2000
HEARTBEAT_STALE_SECONDS = 15
ERROR_BROKEN_PIPE = 109
GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
OPEN_EXISTING = 3
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
STILL_ACTIVE = 259
MB_OK = 0x00000000
MB_ICONINFORMATION = 0x00000040
REQUIRED_REGISTRATION_FIELDS = {
    "instance_id",
    "pid",
    "control_endpoint",
    "started_at",
    "heartbeat_at",
}

def configure_win32_prototypes(kernel32_dll: Any, user32_dll: Any) -> None:
    lp_dword = ctypes.POINTER(wintypes.DWORD)

    kernel32_dll.OpenProcess.argtypes = [
        wintypes.DWORD,
        wintypes.BOOL,
        wintypes.DWORD,
    ]
    kernel32_dll.OpenProcess.restype = wintypes.HANDLE
    kernel32_dll.GetExitCodeProcess.argtypes = [wintypes.HANDLE, lp_dword]
    kernel32_dll.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32_dll.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32_dll.CloseHandle.restype = wintypes.BOOL
    kernel32_dll.WaitNamedPipeW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD]
    kernel32_dll.WaitNamedPipeW.restype = wintypes.BOOL
    kernel32_dll.CreateFileW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        ctypes.c_void_p,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    ]
    kernel32_dll.CreateFileW.restype = wintypes.HANDLE
    kernel32_dll.ReadFile.argtypes = [
        wintypes.HANDLE,
        ctypes.c_void_p,
        wintypes.DWORD,
        lp_dword,
        ctypes.c_void_p,
    ]
    kernel32_dll.ReadFile.restype = wintypes.BOOL
    kernel32_dll.WriteFile.argtypes = [
        wintypes.HANDLE,
        ctypes.c_void_p,
        wintypes.DWORD,
        lp_dword,
        ctypes.c_void_p,
    ]
    kernel32_dll.WriteFile.restype = wintypes.BOOL
    user32_dll.MessageBoxW.argtypes = [
        wintypes.HWND,
        wintypes.LPCWSTR,
        wintypes.LPCWSTR,
        wintypes.UINT,
    ]
    user32_dll.MessageBoxW.restype = ctypes.c_int


if os.name == "nt":

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    configure_win32_prototypes(kernel32, user32)
    INVALID_HANDLE_VALUE = wintypes.HANDLE(-1).value
else:
    kernel32 = None
    user32 = None
    INVALID_HANDLE_VALUE = None


def default_codex_home() -> Path:
    configured = os.environ.get("CODEX_HOME")
    if configured:
        return Path(configured)
    return Path.home() / ".codex"


def app_servers_dir(codex_home: Path | None = None) -> Path:
    root = codex_home or default_codex_home()
    return root / APP_SERVERS_DIR_NAME


def parse_timestamp(raw_value: str) -> dt.datetime:
    normalized = raw_value.replace("Z", "+00:00")
    timestamp = dt.datetime.fromisoformat(normalized)
    if timestamp.tzinfo is None:
        timestamp = timestamp.replace(tzinfo=dt.timezone.utc)
    return timestamp.astimezone(dt.timezone.utc)


def remove_file_if_exists(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        return


def load_registration(path: Path) -> dict[str, Any] | None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        remove_file_if_exists(path)
        return None

    if not isinstance(payload, dict):
        remove_file_if_exists(path)
        return None

    if not REQUIRED_REGISTRATION_FIELDS.issubset(payload):
        remove_file_if_exists(path)
        return None

    if not isinstance(payload.get("pid"), int):
        remove_file_if_exists(path)
        return None

    return payload


def is_pid_alive(pid: int) -> bool:
    if os.name != "nt":
        return True

    process_handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not process_handle:
        return False

    try:
        exit_code = wintypes.DWORD()
        if not kernel32.GetExitCodeProcess(process_handle, ctypes.byref(exit_code)):
            return False
        return exit_code.value == STILL_ACTIVE
    finally:
        kernel32.CloseHandle(process_handle)


def open_named_pipe(endpoint: str, timeout_ms: int = DEFAULT_PIPE_TIMEOUT_MS) -> int:
    if os.name != "nt":
        raise RuntimeError("Windows named pipes are only available on Windows")

    if not kernel32.WaitNamedPipeW(endpoint, timeout_ms):
        raise OSError(ctypes.get_last_error(), f"WaitNamedPipeW failed for {endpoint}")

    handle = kernel32.CreateFileW(
        endpoint,
        GENERIC_READ | GENERIC_WRITE,
        0,
        None,
        OPEN_EXISTING,
        0,
        None,
    )
    if handle == INVALID_HANDLE_VALUE:
        raise OSError(ctypes.get_last_error(), f"CreateFileW failed for {endpoint}")
    return handle


def send_control_request(
    endpoint: str,
    payload: dict[str, Any],
    timeout_ms: int = DEFAULT_PIPE_TIMEOUT_MS,
) -> dict[str, Any]:
    if os.name != "nt":
        raise RuntimeError("The tray control pipe is only available on Windows")

    handle = open_named_pipe(endpoint, timeout_ms=timeout_ms)
    try:
        raw_request = json.dumps(payload, ensure_ascii=False).encode("utf-8") + b"\n"
        request_buffer = ctypes.create_string_buffer(raw_request)
        bytes_written = wintypes.DWORD()
        if not kernel32.WriteFile(
            handle,
            request_buffer,
            len(raw_request),
            ctypes.byref(bytes_written),
            None,
        ):
            raise OSError(ctypes.get_last_error(), f"WriteFile failed for {endpoint}")

        response_bytes = bytearray()
        while True:
            chunk = ctypes.create_string_buffer(4096)
            bytes_read = wintypes.DWORD()
            ok = kernel32.ReadFile(
                handle,
                chunk,
                len(chunk),
                ctypes.byref(bytes_read),
                None,
            )
            if not ok:
                error = ctypes.get_last_error()
                if error == ERROR_BROKEN_PIPE:
                    break
                raise OSError(error, f"ReadFile failed for {endpoint}")
            if bytes_read.value == 0:
                break
            response_bytes.extend(chunk.raw[: bytes_read.value])
            if b"\n" in response_bytes:
                break

        if not response_bytes:
            raise ValueError(f"empty control response from {endpoint}")

        first_line = response_bytes.splitlines()[0]
        response = json.loads(first_line.decode("utf-8"))
        if not isinstance(response, dict):
            raise ValueError(f"invalid control response shape from {endpoint}")
        return response
    finally:
        kernel32.CloseHandle(handle)


def ping_instance(endpoint: str) -> bool:
    response = send_control_request(endpoint, {"op": "ping"})
    return bool(response.get("ok"))


def heartbeat_is_stale(
    registration: dict[str, Any],
    now: dt.datetime | None = None,
) -> bool:
    current_time = now or dt.datetime.now(dt.timezone.utc)
    heartbeat_at = parse_timestamp(str(registration["heartbeat_at"]))
    return (current_time - heartbeat_at).total_seconds() > HEARTBEAT_STALE_SECONDS


def prune_stale_registration(
    path: Path,
    now: dt.datetime | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
) -> dict[str, Any] | None:
    registration = load_registration(path)
    if registration is None:
        return None

    if not pid_checker(int(registration["pid"])):
        remove_file_if_exists(path)
        return None

    try:
        stale = heartbeat_is_stale(registration, now=now)
    except (TypeError, ValueError):
        remove_file_if_exists(path)
        return None

    if stale:
        try:
            ping_ok = ping_checker(str(registration["control_endpoint"]))
        except Exception:
            ping_ok = False
        if not ping_ok:
            remove_file_if_exists(path)
            return None

    return registration


def enumerate_live_registrations(
    registry_dir: Path | None = None,
    now: dt.datetime | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
) -> list[dict[str, Any]]:
    directory = registry_dir or app_servers_dir()
    if not directory.exists():
        return []

    registrations: list[dict[str, Any]] = []
    for entry in sorted(directory.glob("*.json")):
        registration = prune_stale_registration(
            entry,
            now=now,
            pid_checker=pid_checker,
            ping_checker=ping_checker,
        )
        if registration is not None:
            registrations.append(registration)
    return registrations


def format_result_message(success_count: int, failure_count: int) -> str:
    return (
        "刷新全部 app-server 完成\n\n"
        f"成功实例：{success_count}\n"
        f"失败实例：{failure_count}"
    )


def show_result_message(success_count: int, failure_count: int) -> None:
    if os.name != "nt":
        print(format_result_message(success_count, failure_count))
        return

    user32.MessageBoxW(
        None,
        format_result_message(success_count, failure_count),
        "Codex App Server Refresh",
        MB_OK | MB_ICONINFORMATION,
    )


def refresh_all_instances(registry_dir: Path | None = None) -> tuple[int, int]:
    registrations = enumerate_live_registrations(registry_dir=registry_dir)
    success_count = 0
    failure_count = 0

    for registration in registrations:
        try:
            response = send_control_request(
                str(registration["control_endpoint"]),
                {"op": "refresh_all_loaded_threads"},
            )
            failed_threads = response.get("failed_threads")
            if response.get("ok") is True and isinstance(failed_threads, list) and not failed_threads:
                success_count += 1
            else:
                failure_count += 1
        except Exception:
            failure_count += 1

    return success_count, failure_count


def create_tray_icon():
    import pystray
    from PIL import Image
    from PIL import ImageDraw

    def build_image() -> Image.Image:
        image = Image.new("RGBA", (64, 64), (245, 247, 250, 255))
        draw = ImageDraw.Draw(image)
        draw.rounded_rectangle((8, 8, 56, 56), radius=12, fill=(27, 38, 59, 255))
        draw.rectangle((18, 22, 46, 28), fill=(255, 196, 61, 255))
        draw.rectangle((18, 32, 46, 38), fill=(119, 141, 169, 255))
        draw.rectangle((18, 42, 38, 48), fill=(224, 225, 221, 255))
        return image

    def handle_refresh(icon: pystray.Icon, _item: Any) -> None:
        def worker() -> None:
            success_count, failure_count = refresh_all_instances()
            show_result_message(success_count, failure_count)

        threading.Thread(target=worker, daemon=True).start()

    return pystray.Icon(
        "codex-app-server-refresh",
        build_image(),
        "Codex App Server Refresh",
        menu=pystray.Menu(pystray.MenuItem("刷新全部 app-server", handle_refresh)),
    )


def main() -> None:
    if os.name != "nt":
        raise SystemExit("windows_app_server_refresh_tray.py only supports Windows")

    icon = create_tray_icon()
    icon.run()


if __name__ == "__main__":
    main()
