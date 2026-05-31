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

    def test_apply_provider_prefers_app_server_smart_apply(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            write_registration(
                registry_dir,
                "live.json",
                control_endpoint=r"\\.\pipe\codex-app-server-smart",
            )
            requests: list[dict[str, object]] = []

            def send_request(endpoint: str, payload: dict[str, object]) -> dict[str, object]:
                requests.append({"endpoint": endpoint, "payload": payload})
                self.assertEqual(endpoint, r"\\.\pipe\codex-app-server-smart")
                self.assertEqual(
                    payload,
                    {
                        "op": "apply_provider_runtime_from_effective_provider",
                        "source_provider_id": "saki",
                    },
                )
                return {
                    "ok": True,
                    "outcome": "success",
                    "source_provider_id": "saki",
                    "current_model_provider_id": "yunyi",
                    "total_threads": 2,
                    "applied_thread_ids": ["thread-a"],
                    "queued_thread_ids": ["thread-b"],
                    "failed_threads": [],
                }

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertTrue(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "app_server_smart_apply")
            self.assertEqual(summary["success_instances"], 1)
            self.assertEqual(summary["fallback_instances"], 0)
            self.assertEqual(summary["applied_threads"], 1)
            self.assertEqual(summary["queued_threads"], 1)
            self.assertEqual(len(requests), 1)

    def test_apply_provider_falls_back_when_every_instance_lacks_smart_apply(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            codex_home = Path(tempdir)
            registry_dir = codex_home / TRAY.APP_SERVERS_DIR_NAME
            registry_dir.mkdir()
            write_registration(
                registry_dir,
                "live.json",
                control_endpoint=r"\\.\pipe\codex-app-server-legacy",
            )
            (codex_home / TRAY.CONFIG_TOML_FILE_NAME).write_text(
                """
model_provider = "yunyi"

[model_providers.saki]
base_url = "https://api.saki.example/v1"
experimental_bearer_token = "new-token"

[model_providers.yunyi]
base_url = "https://api.old.example/v1"
experimental_bearer_token = "old-token"
""".strip()
                + "\n",
                encoding="utf-8",
            )
            requests: list[dict[str, object]] = []

            def send_request(endpoint: str, payload: dict[str, object]) -> dict[str, object]:
                requests.append({"endpoint": endpoint, "payload": payload})
                if payload.get("op") == "apply_provider_runtime_from_effective_provider":
                    return {
                        "ok": False,
                        "error": "unsupported control operation: apply_provider_runtime_from_effective_provider",
                    }
                if payload.get("op") == "refresh_all_loaded_threads":
                    return {
                        "ok": True,
                        "total_threads": 1,
                        "applied_thread_ids": ["thread-a"],
                        "queued_thread_ids": [],
                        "failed_threads": [],
                    }
                raise AssertionError(f"unexpected payload: {payload}")

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                codex_home=codex_home,
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertTrue(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "legacy_config_write")
            self.assertTrue(summary["config_changed"])
            self.assertEqual(summary["success_instances"], 1)
            self.assertEqual(summary["fallback_instances"], 1)
            self.assertEqual(
                [request["payload"]["op"] for request in requests],
                [
                    "apply_provider_runtime_from_effective_provider",
                    "refresh_all_loaded_threads",
                ],
            )
            updated = (codex_home / TRAY.CONFIG_TOML_FILE_NAME).read_text(encoding="utf-8")
            self.assertIn('base_url = "https://api.saki.example/v1"', updated)
            self.assertIn('experimental_bearer_token = "new-token"', updated)

    def test_apply_provider_does_not_treat_legacy_refresh_as_success_when_smart_apply_failed(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            write_registration(
                registry_dir,
                "smart-fails.json",
                control_endpoint=r"\\.\pipe\codex-app-server-smart-fails",
            )
            write_registration(
                registry_dir,
                "legacy.json",
                control_endpoint=r"\\.\pipe\codex-app-server-legacy",
            )
            requests: list[dict[str, object]] = []

            def send_request(endpoint: str, payload: dict[str, object]) -> dict[str, object]:
                requests.append({"endpoint": endpoint, "payload": payload})
                if endpoint == r"\\.\pipe\codex-app-server-smart-fails":
                    return {
                        "ok": False,
                        "outcome": "config_write_failed",
                        "message": "simulated write failure",
                        "applied_thread_ids": [],
                        "queued_thread_ids": [],
                        "failed_threads": [],
                    }
                if endpoint == r"\\.\pipe\codex-app-server-legacy":
                    return {
                        "ok": False,
                        "error": "unsupported control operation: apply_provider_runtime_from_effective_provider",
                    }
                raise AssertionError(f"unexpected endpoint: {endpoint}")

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertFalse(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "app_server_smart_apply")
            self.assertEqual(summary["success_instances"], 0)
            self.assertEqual(summary["fallback_instances"], 0)
            self.assertEqual(
                [request["payload"]["op"] for request in requests],
                [
                    "apply_provider_runtime_from_effective_provider",
                    "apply_provider_runtime_from_effective_provider",
                ],
            )

    def test_apply_provider_does_not_fall_back_when_smart_apply_request_raises(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            write_registration(
                registry_dir,
                "raises.json",
                control_endpoint=r"\\.\pipe\codex-app-server-raises",
            )

            def send_request(_endpoint: str, _payload: dict[str, object]) -> dict[str, object]:
                raise OSError("simulated pipe failure")

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertFalse(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "app_server_smart_apply")
            self.assertEqual(summary["failed_instances"], 1)
            self.assertEqual(summary["fallback_instances"], 0)

            _title, message, icon_flag = TRAY.format_apply_summary(summary)
            self.assertEqual(icon_flag, TRAY.MB_ICONERROR)
            self.assertIn("simulated pipe failure", message)

    def test_apply_provider_writes_config_but_reports_no_live_instances(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            codex_home = Path(tempdir)
            registry_dir = codex_home / TRAY.APP_SERVERS_DIR_NAME
            registry_dir.mkdir()
            (codex_home / TRAY.CONFIG_TOML_FILE_NAME).write_text(
                """
model_provider = "yunyi"

[model_providers.saki]
base_url = "https://api.saki.example/v1"
experimental_bearer_token = "new-token"

[model_providers.yunyi]
base_url = "https://api.old.example/v1"
experimental_bearer_token = "old-token"
""".strip()
                + "\n",
                encoding="utf-8",
            )

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                codex_home=codex_home,
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=lambda _endpoint, _payload: {},
            )

            self.assertTrue(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "legacy_config_write_no_live_instances")
            self.assertTrue(summary["config_changed"])
            self.assertEqual(summary["refresh_summary"]["total_instances"], 0)

            _title, message, icon_flag = TRAY.format_apply_summary(summary)
            self.assertEqual(icon_flag, TRAY.MB_ICONINFORMATION)
            self.assertIn("未刷新任何 live 实例", message)

    def test_apply_provider_refreshes_unsupported_instances_after_smart_apply_success(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            write_registration(
                registry_dir,
                "smart.json",
                control_endpoint=r"\\.\pipe\codex-app-server-smart",
            )
            write_registration(
                registry_dir,
                "legacy.json",
                control_endpoint=r"\\.\pipe\codex-app-server-legacy",
            )
            requests: list[dict[str, object]] = []

            def send_request(endpoint: str, payload: dict[str, object]) -> dict[str, object]:
                requests.append({"endpoint": endpoint, "payload": payload})
                if endpoint == r"\\.\pipe\codex-app-server-smart":
                    self.assertEqual(
                        payload["op"],
                        "apply_provider_runtime_from_effective_provider",
                    )
                    return {
                        "ok": True,
                        "outcome": "success",
                        "source_provider_id": "saki",
                        "current_model_provider_id": "yunyi",
                        "applied_thread_ids": ["thread-a"],
                        "queued_thread_ids": [],
                        "failed_threads": [],
                    }
                if endpoint == r"\\.\pipe\codex-app-server-legacy":
                    if payload["op"] == "apply_provider_runtime_from_effective_provider":
                        return {
                            "ok": False,
                            "error": "unsupported control operation: apply_provider_runtime_from_effective_provider",
                        }
                    if payload["op"] == "refresh_all_loaded_threads":
                        return {
                            "ok": True,
                            "applied_thread_ids": ["thread-b"],
                            "queued_thread_ids": [],
                            "failed_threads": [],
                        }
                raise AssertionError(f"unexpected request: {endpoint} {payload}")

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertTrue(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "app_server_smart_apply")
            self.assertEqual(summary["success_instances"], 2)
            self.assertEqual(summary["smart_apply_instances"], 1)
            self.assertEqual(summary["fallback_instances"], 1)
            self.assertEqual(
                [(request["endpoint"], request["payload"]["op"]) for request in requests],
                [
                    (
                        r"\\.\pipe\codex-app-server-legacy",
                        "apply_provider_runtime_from_effective_provider",
                    ),
                    (
                        r"\\.\pipe\codex-app-server-smart",
                        "apply_provider_runtime_from_effective_provider",
                    ),
                    (r"\\.\pipe\codex-app-server-legacy", "refresh_all_loaded_threads"),
                ],
            )

    def test_format_apply_summary_names_smart_apply_strategy(self) -> None:
        title, message, icon_flag = TRAY.format_apply_summary(
            {
                "ok": True,
                "source_provider_id": "saki",
                "current_model_provider_id": "yunyi",
                "config_path": r"C:\Users\Administrator\.codex\config.toml",
                "config_changed": True,
                "apply_strategy": "app_server_smart_apply",
                "refresh_summary": {
                    "total_instances": 1,
                    "success_instances": 1,
                    "failed_instances": 0,
                    "applied_threads": 1,
                    "queued_threads": 0,
                    "failed_threads": 0,
                    "smart_apply_instances": 1,
                    "fallback_instances": 0,
                    "details": [],
                },
            }
        )

        self.assertEqual(title, "Codex Provider Apply")
        self.assertEqual(icon_flag, TRAY.MB_ICONINFORMATION)
        self.assertIn("已通过当前 app-server 智能应用 provider", message)
        self.assertIn("应用方式: app-server 智能刷新", message)
        self.assertIn("智能 apply 实例: 1", message)

    def test_apply_provider_treats_smart_partial_failure_as_written_but_warns(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            registry_dir = Path(tempdir)
            write_registration(
                registry_dir,
                "partial.json",
                control_endpoint=r"\\.\pipe\codex-app-server-partial",
            )

            def send_request(_endpoint: str, payload: dict[str, object]) -> dict[str, object]:
                self.assertEqual(
                    payload,
                    {
                        "op": "apply_provider_runtime_from_effective_provider",
                        "source_provider_id": "saki",
                    },
                )
                return {
                    "ok": False,
                    "outcome": "partial_failure",
                    "source_provider_id": "saki",
                    "current_model_provider_id": "yunyi",
                    "total_threads": 2,
                    "applied_thread_ids": ["thread-a"],
                    "queued_thread_ids": [],
                    "failed_threads": [
                        {"thread_id": "thread-b", "message": "refresh failed"}
                    ],
                }

            summary = TRAY.apply_provider_runtime_smart_first(
                "saki",
                registry_dir=registry_dir,
                pid_checker=lambda _pid: True,
                ping_checker=lambda _endpoint: True,
                send_request=send_request,
            )

            self.assertTrue(summary["ok"])
            self.assertEqual(summary["apply_strategy"], "app_server_smart_apply")
            self.assertEqual(summary["success_instances"], 0)
            self.assertEqual(summary["failed_instances"], 1)
            self.assertEqual(summary["failed_threads"], 1)

            _title, message, icon_flag = TRAY.format_apply_summary(summary)
            self.assertEqual(icon_flag, TRAY.MB_ICONWARNING)
            self.assertIn("已通过当前 app-server 智能应用 provider", message)
            self.assertIn("failed_threads=1", message)

    def test_format_apply_summary_shows_smart_apply_failure_message(self) -> None:
        _title, message, icon_flag = TRAY.format_apply_summary(
            {
                "ok": False,
                "message": "app-server 智能 provider apply 未成功刷新任何实例",
                "source_provider_id": "saki",
                "current_model_provider_id": "yunyi",
                "config_path": r"C:\Users\Administrator\.codex\config.toml",
                "config_changed": False,
                "apply_strategy": "app_server_smart_apply",
                "refresh_summary": {
                    "total_instances": 1,
                    "success_instances": 0,
                    "failed_instances": 1,
                    "applied_threads": 0,
                    "queued_threads": 0,
                    "failed_threads": 0,
                    "smart_apply_instances": 1,
                    "fallback_instances": 0,
                    "details": [
                        {
                            "instance_id": "instance-1",
                            "ok": False,
                            "method": "smart_apply",
                            "response": {
                                "ok": False,
                                "outcome": "config_write_failed",
                                "message": "current model_provider is not backed by a writable user config entry",
                                "failed_threads": [],
                            },
                        }
                    ],
                },
            }
        )

        self.assertEqual(icon_flag, TRAY.MB_ICONERROR)
        self.assertIn("未成功刷新任何实例", message)
        self.assertIn("current model_provider is not backed", message)


if __name__ == "__main__":
    unittest.main()
