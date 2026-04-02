from __future__ import annotations

import datetime as dt
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = (
    Path(__file__).resolve().parents[1] / "windows_app_server_refresh_tray.py"
)
SPEC = importlib.util.spec_from_file_location("windows_app_server_refresh_tray", MODULE_PATH)
TRAY = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(TRAY)


def write_registration(
    directory: Path,
    name: str,
    *,
    pid: int = 1234,
    control_endpoint: str = r"\\.\pipe\codex-app-server-test",
    heartbeat_at: str = "2026-04-02T00:00:00Z",
) -> Path:
    path = directory / name
    payload = {
        "instance_id": "instance-1",
        "pid": pid,
        "control_endpoint": control_endpoint,
        "started_at": "2026-04-02T00:00:00Z",
        "heartbeat_at": heartbeat_at,
    }
    path.write_text(json.dumps(payload), encoding="utf-8")
    return path


class WindowsAppServerRefreshTrayTests(unittest.TestCase):
    def test_configure_win32_prototypes_sets_handle_safe_signatures(self) -> None:
        class DummyFunction:
            def __init__(self) -> None:
                self.argtypes = None
                self.restype = None

        class DummyKernel32:
            def __init__(self) -> None:
                self.OpenProcess = DummyFunction()
                self.GetExitCodeProcess = DummyFunction()
                self.CloseHandle = DummyFunction()
                self.WaitNamedPipeW = DummyFunction()
                self.CreateFileW = DummyFunction()
                self.ReadFile = DummyFunction()
                self.WriteFile = DummyFunction()

        class DummyUser32:
            def __init__(self) -> None:
                self.MessageBoxW = DummyFunction()

        kernel32 = DummyKernel32()
        user32 = DummyUser32()
        TRAY.configure_win32_prototypes(kernel32, user32)

        self.assertEqual(
            kernel32.OpenProcess.argtypes,
            [TRAY.wintypes.DWORD, TRAY.wintypes.BOOL, TRAY.wintypes.DWORD],
        )
        self.assertIs(kernel32.OpenProcess.restype, TRAY.wintypes.HANDLE)
        self.assertIs(kernel32.CreateFileW.restype, TRAY.wintypes.HANDLE)
        self.assertIs(kernel32.ReadFile.restype, TRAY.wintypes.BOOL)
        self.assertIs(kernel32.WriteFile.restype, TRAY.wintypes.BOOL)
        self.assertIs(user32.MessageBoxW.restype, TRAY.ctypes.c_int)

    def test_invalid_json_registration_is_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            path = registry_dir / "broken.json"
            path.write_text("{not valid json", encoding="utf-8")

            registration = TRAY.prune_stale_registration(
                path,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
            )

            self.assertIsNone(registration)
            self.assertFalse(path.exists())

    def test_dead_pid_registration_is_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            path = write_registration(registry_dir, "dead.json", pid=9999)

            registration = TRAY.prune_stale_registration(
                path,
                pid_checker=lambda _pid: False,
                ping_checker=lambda _endpoint: True,
            )

            self.assertIsNone(registration)
            self.assertFalse(path.exists())

    def test_stale_registration_uses_ping_before_deciding_cleanup(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            stale_path = write_registration(
                registry_dir,
                "stale.json",
                heartbeat_at="2026-04-02T00:00:00Z",
            )
            now = dt.datetime(2026, 4, 2, 0, 0, 20, tzinfo=dt.timezone.utc)

            kept = TRAY.prune_stale_registration(
                stale_path,
                now=now,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
            )
            self.assertIsNotNone(kept)
            self.assertTrue(stale_path.exists())

            deleted = TRAY.prune_stale_registration(
                stale_path,
                now=now,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: False,
            )
            self.assertIsNone(deleted)
            self.assertFalse(stale_path.exists())

    def test_enumerate_live_registrations_reads_only_app_servers_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            codex_home = Path(tempdir)
            registry_dir = codex_home / TRAY.APP_SERVERS_DIR_NAME
            registry_dir.mkdir()
            ignored_dir = codex_home / "other"
            ignored_dir.mkdir()

            write_registration(registry_dir, "live.json")
            write_registration(ignored_dir, "ignored.json")

            registrations = TRAY.enumerate_live_registrations(
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
            )

            self.assertEqual(len(registrations), 1)
            self.assertEqual(registrations[0]["instance_id"], "instance-1")

    def test_format_result_message_reports_only_success_and_failure_counts(self) -> None:
        message = TRAY.format_result_message(3, 1)
        self.assertEqual(
            message,
            "刷新全部 app-server 完成\n\n成功实例：3\n失败实例：1",
        )


if __name__ == "__main__":
    unittest.main()
