from __future__ import annotations

import ctypes
import datetime as dt
import json
import os
import threading
from ctypes import wintypes
from pathlib import Path
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
MB_ICONWARNING = 0x00000030
MB_ICONERROR = 0x00000010
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


class TrayState:
    def __init__(self) -> None:
        self._lock = threading.RLock()
        self.providers: list[dict[str, Any]] = []
        self.selected_provider_id: str | None = None
        self.current_model_provider_id: str | None = None
        self.current_model_provider_writable = False
        self.catalog_error: str | None = None

    def snapshot(self) -> dict[str, Any]:
        with self._lock:
            return {
                "providers": [provider.copy() for provider in self.providers],
                "selected_provider_id": self.selected_provider_id,
                "current_model_provider_id": self.current_model_provider_id,
                "current_model_provider_writable": self.current_model_provider_writable,
                "catalog_error": self.catalog_error,
            }

    def set_catalog(self, response: dict[str, Any]) -> None:
        providers = response.get("providers")
        if not isinstance(providers, list):
            providers = []

        normalized: list[dict[str, Any]] = []
        for provider in providers:
            if not isinstance(provider, dict):
                continue
            provider_id = provider.get("provider_id")
            display_name = provider.get("display_name")
            if not isinstance(provider_id, str) or not provider_id:
                continue
            normalized.append(
                {
                    "provider_id": provider_id,
                    "display_name": display_name if isinstance(display_name, str) and display_name else provider_id,
                    "has_base_url": bool(provider.get("has_base_url")),
                    "has_experimental_bearer_token": bool(
                        provider.get("has_experimental_bearer_token")
                    ),
                }
            )

        current_model_provider_id = response.get("current_model_provider_id")
        if not isinstance(current_model_provider_id, str):
            current_model_provider_id = None

        with self._lock:
            self.providers = normalized
            self.current_model_provider_id = current_model_provider_id
            self.current_model_provider_writable = bool(
                response.get("current_model_provider_writable")
            )
            self.catalog_error = None
            valid_ids = {provider["provider_id"] for provider in normalized}
            if self.selected_provider_id not in valid_ids:
                if current_model_provider_id in valid_ids:
                    self.selected_provider_id = current_model_provider_id
                else:
                    self.selected_provider_id = normalized[0]["provider_id"] if normalized else None

    def clear_catalog(self, error_message: str | None = None) -> None:
        with self._lock:
            self.providers = []
            self.selected_provider_id = None
            self.current_model_provider_id = None
            self.current_model_provider_writable = False
            self.catalog_error = error_message

    def set_selected_provider(self, provider_id: str) -> None:
        with self._lock:
            valid_ids = {provider["provider_id"] for provider in self.providers}
            if provider_id in valid_ids:
                self.selected_provider_id = provider_id


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
        return timestamp.replace(tzinfo=dt.timezone.utc)
    return timestamp.astimezone(dt.timezone.utc)


def load_registration(path: Path) -> dict[str, Any] | None:
    try:
        raw_text = path.read_text(encoding="utf-8")
    except OSError:
        return None
    try:
        payload = json.loads(raw_text)
    except json.JSONDecodeError:
        remove_file_if_exists(path)
        return None

    if not isinstance(payload, dict):
        remove_file_if_exists(path)
        return None

    if not REQUIRED_REGISTRATION_FIELDS.issubset(payload):
        remove_file_if_exists(path)
        return None

    return payload


def remove_file_if_exists(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        return
    except OSError:
        return


def is_pid_alive(pid: int) -> bool:
    if os.name != "nt":
        return False
    process_handle = kernel32.OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        False,
        pid,
    )
    if not process_handle:
        return False
    try:
        exit_code = wintypes.DWORD()
        if not kernel32.GetExitCodeProcess(process_handle, ctypes.byref(exit_code)):
            return False
        return exit_code.value == STILL_ACTIVE
    finally:
        kernel32.CloseHandle(process_handle)


def send_control_request(
    endpoint: str,
    payload: dict[str, Any],
    timeout_ms: int = DEFAULT_PIPE_TIMEOUT_MS,
) -> dict[str, Any]:
    if os.name != "nt":
        raise RuntimeError("named pipe control is only available on Windows")

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

    try:
        raw = (json.dumps(payload) + "\n").encode("utf-8")
        bytes_written = wintypes.DWORD()
        if not kernel32.WriteFile(
            handle,
            raw,
            len(raw),
            ctypes.byref(bytes_written),
            None,
        ):
            raise OSError(ctypes.get_last_error(), f"WriteFile failed for {endpoint}")

        response_bytes = bytearray()
        buffer_size = 4096
        while True:
            chunk = ctypes.create_string_buffer(buffer_size)
            bytes_read = wintypes.DWORD()
            success = kernel32.ReadFile(
                handle,
                chunk,
                buffer_size,
                ctypes.byref(bytes_read),
                None,
            )
            if not success:
                error = ctypes.get_last_error()
                if error == ERROR_BROKEN_PIPE and response_bytes:
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


def show_message(title: str, message: str, icon_flag: int = MB_ICONINFORMATION) -> None:
    if os.name != "nt":
        print(f"{title}\n{message}")
        return

    user32.MessageBoxW(
        None,
        message,
        title,
        MB_OK | icon_flag,
    )


def refresh_all_instances(registry_dir: Path | None = None) -> dict[str, Any]:
    registrations = enumerate_live_registrations(registry_dir=registry_dir)
    summary = {
        "total_instances": len(registrations),
        "success_instances": 0,
        "failed_instances": 0,
        "applied_threads": 0,
        "queued_threads": 0,
        "failed_threads": 0,
        "details": [],
    }

    for registration in registrations:
        instance_id = str(registration["instance_id"])
        endpoint = str(registration["control_endpoint"])
        try:
            response = send_control_request(endpoint, {"op": "refresh_all_loaded_threads"})
            failed_threads = response.get("failed_threads")
            applied_thread_ids = response.get("applied_thread_ids")
            queued_thread_ids = response.get("queued_thread_ids")
            instance_ok = bool(response.get("ok")) and isinstance(failed_threads, list) and not failed_threads
            if isinstance(applied_thread_ids, list):
                summary["applied_threads"] += len(applied_thread_ids)
            if isinstance(queued_thread_ids, list):
                summary["queued_threads"] += len(queued_thread_ids)
            if isinstance(failed_threads, list):
                summary["failed_threads"] += len(failed_threads)
            if instance_ok:
                summary["success_instances"] += 1
            else:
                summary["failed_instances"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": instance_ok,
                    "response": response,
                }
            )
        except Exception as exc:
            summary["failed_instances"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": False,
                    "error": str(exc),
                }
            )

    return summary


def short_error_text(message: str, limit: int = 96) -> str:
    collapsed = " ".join(message.split())
    if len(collapsed) <= limit:
        return collapsed
    return f"{collapsed[: limit - 3]}..."


def normalized_catalog_signature(response: dict[str, Any]) -> tuple[Any, ...]:
    current_model_provider_id = response.get("current_model_provider_id")
    if not isinstance(current_model_provider_id, str) or not current_model_provider_id:
        raise RuntimeError("provider catalog response is missing current_model_provider_id")

    providers = response.get("providers")
    if not isinstance(providers, list):
        raise RuntimeError("provider catalog response has invalid providers payload")

    normalized_providers: list[tuple[str, bool, bool]] = []
    for provider in providers:
        if not isinstance(provider, dict):
            raise RuntimeError("provider catalog entry is not an object")
        provider_id = provider.get("provider_id")
        if not isinstance(provider_id, str) or not provider_id:
            raise RuntimeError("provider catalog entry is missing provider_id")
        normalized_providers.append(
            (
                provider_id,
                bool(provider.get("has_base_url")),
                bool(provider.get("has_experimental_bearer_token")),
            )
        )
    normalized_providers.sort(key=lambda item: item[0])

    return (
        current_model_provider_id,
        bool(response.get("current_model_provider_writable")),
        tuple(normalized_providers),
    )


def fetch_provider_catalog(registry_dir: Path | None = None) -> dict[str, Any]:
    registrations = enumerate_live_registrations(registry_dir=registry_dir)
    if not registrations:
        raise RuntimeError("no live app-server instances were found")

    baseline_instance_id: str | None = None
    baseline_response: dict[str, Any] | None = None
    baseline_signature: tuple[Any, ...] | None = None
    failed_instances: list[str] = []
    mismatched_instances: list[str] = []

    for registration in registrations:
        instance_id = str(registration["instance_id"])
        endpoint = str(registration["control_endpoint"])
        try:
            response = send_control_request(endpoint, {"op": "list_effective_providers"})
            if response.get("ok") is not True:
                raise RuntimeError(
                    str(response.get("error") or "unknown provider catalog error")
                )
            signature = normalized_catalog_signature(response)
        except Exception as exc:
            failed_instances.append(f"{instance_id}: {exc}")
            continue
        if baseline_signature is None:
            baseline_instance_id = instance_id
            baseline_response = response
            baseline_signature = signature
            continue
        if signature != baseline_signature:
            mismatched_instances.append(instance_id)

    if baseline_response is None:
        raise RuntimeError(
            "failed to load provider catalog from live app-server instances: "
            + "; ".join(failed_instances)
        )
    if failed_instances:
        raise RuntimeError(
            "failed to load provider catalog from one or more live app-server instances: "
            + "; ".join(failed_instances)
        )
    if mismatched_instances:
        mismatched = ", ".join(mismatched_instances)
        raise RuntimeError(
            "provider catalog mismatch across live app-server instances; "
            f"baseline={baseline_instance_id}, mismatched={mismatched}"
        )
    return baseline_response


def apply_selected_provider_to_all_instances(
    source_provider_id: str,
    registry_dir: Path | None = None,
) -> dict[str, Any]:
    registrations = enumerate_live_registrations(registry_dir=registry_dir)
    summary = {
        "source_provider_id": source_provider_id,
        "total_instances": len(registrations),
        "success": 0,
        "partial_failure": 0,
        "config_write_failed": 0,
        "provider_parse_failed": 0,
        "provider_field_missing": 0,
        "request_failed": 0,
        "applied_threads": 0,
        "queued_threads": 0,
        "failed_threads": 0,
        "details": [],
    }

    for registration in registrations:
        instance_id = str(registration["instance_id"])
        endpoint = str(registration["control_endpoint"])
        try:
            response = send_control_request(
                endpoint,
                {
                    "op": "apply_provider_runtime_from_effective_provider",
                    "source_provider_id": source_provider_id,
                },
            )
            outcome = response.get("outcome")
            applied_thread_ids = response.get("applied_thread_ids")
            queued_thread_ids = response.get("queued_thread_ids")
            failed_threads = response.get("failed_threads")
            if isinstance(applied_thread_ids, list):
                summary["applied_threads"] += len(applied_thread_ids)
            if isinstance(queued_thread_ids, list):
                summary["queued_threads"] += len(queued_thread_ids)
            if isinstance(failed_threads, list):
                summary["failed_threads"] += len(failed_threads)
            if outcome == "success":
                summary["success"] += 1
            elif outcome == "partial_failure":
                summary["partial_failure"] += 1
            elif outcome == "config_write_failed":
                summary["config_write_failed"] += 1
            elif outcome == "provider_parse_failed":
                summary["provider_parse_failed"] += 1
            elif outcome == "provider_field_missing":
                summary["provider_field_missing"] += 1
            else:
                summary["request_failed"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "response": response,
                }
            )
        except Exception as exc:
            summary["request_failed"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "error": str(exc),
                }
            )

    return summary


def format_refresh_summary(summary: dict[str, Any]) -> tuple[str, str, int]:
    total_instances = int(summary.get("total_instances", 0))
    success_instances = int(summary.get("success_instances", 0))
    failed_instances = int(summary.get("failed_instances", 0))
    applied_threads = int(summary.get("applied_threads", 0))
    queued_threads = int(summary.get("queued_threads", 0))
    failed_threads = int(summary.get("failed_threads", 0))
    title = "Codex App Server Refresh"
    icon_flag = MB_ICONINFORMATION if failed_instances == 0 else MB_ICONWARNING
    message = (
        "刷新全部 app-server 完成\n\n"
        f"实例总数：{total_instances}\n"
        f"成功实例：{success_instances}\n"
        f"失败实例：{failed_instances}\n"
        f"Applied 线程：{applied_threads}\n"
        f"Queued 线程：{queued_threads}\n"
        f"Failed 线程：{failed_threads}"
    )
    return title, message, icon_flag


def format_apply_summary(summary: dict[str, Any], state_snapshot: dict[str, Any]) -> tuple[str, str, int]:
    total_instances = int(summary.get("total_instances", 0))
    success = int(summary.get("success", 0))
    partial_failure = int(summary.get("partial_failure", 0))
    config_write_failed = int(summary.get("config_write_failed", 0))
    provider_parse_failed = int(summary.get("provider_parse_failed", 0))
    provider_field_missing = int(summary.get("provider_field_missing", 0))
    request_failed = int(summary.get("request_failed", 0))
    applied_threads = int(summary.get("applied_threads", 0))
    queued_threads = int(summary.get("queued_threads", 0))
    failed_threads = int(summary.get("failed_threads", 0))
    source_provider_id = str(summary.get("source_provider_id") or "unknown")
    current_model_provider_id = state_snapshot.get("current_model_provider_id") or "unknown"
    catalog_error = state_snapshot.get("catalog_error")

    if config_write_failed or provider_parse_failed or provider_field_missing or request_failed:
        icon_flag = MB_ICONERROR
    elif partial_failure:
        icon_flag = MB_ICONWARNING
    else:
        icon_flag = MB_ICONINFORMATION

    lines = [
        "应用所选 provider 到当前 model_provider 并刷新",
        "",
        f"源 provider：{source_provider_id}",
        f"目标 model_provider：{current_model_provider_id}",
        f"实例总数：{total_instances}",
        f"成功：{success}",
        f"部分失败：{partial_failure}",
        f"配置写入失败：{config_write_failed}",
        f"provider 解析失败：{provider_parse_failed}",
        f"字段缺失失败：{provider_field_missing}",
        f"请求失败：{request_failed}",
        f"Applied 线程：{applied_threads}",
        f"Queued 线程：{queued_threads}",
        f"Failed 线程：{failed_threads}",
    ]
    if isinstance(catalog_error, str) and catalog_error:
        lines.extend(["", f"provider 列表状态：{catalog_error}"])
    return "Codex Provider Apply", "\n".join(lines), icon_flag


def create_tray_icon():
    import pystray
    from PIL import Image
    from PIL import ImageDraw

    state = TrayState()
    try:
        state.set_catalog(fetch_provider_catalog())
    except Exception as exc:
        state.clear_catalog(str(exc))

    def build_image() -> Image.Image:
        image = Image.new("RGBA", (64, 64), (245, 247, 250, 255))
        draw = ImageDraw.Draw(image)
        draw.rounded_rectangle((8, 8, 56, 56), radius=12, fill=(27, 38, 59, 255))
        draw.rectangle((18, 22, 46, 28), fill=(255, 196, 61, 255))
        draw.rectangle((18, 32, 46, 38), fill=(119, 141, 169, 255))
        draw.rectangle((18, 42, 38, 48), fill=(224, 225, 221, 255))
        return image

    def rebuild_menu(icon: pystray.Icon) -> None:
        icon.menu = build_menu(icon)
        icon.update_menu()

    def reload_catalog() -> None:
        try:
            state.set_catalog(fetch_provider_catalog())
        except Exception as exc:
            state.clear_catalog(str(exc))

    def handle_refresh(icon: pystray.Icon, _item: Any) -> None:
        def worker() -> None:
            summary = refresh_all_instances()
            reload_catalog()
            rebuild_menu(icon)
            title, message, icon_flag = format_refresh_summary(summary)
            show_message(title, message, icon_flag)

        threading.Thread(target=worker, daemon=True).start()

    def handle_apply(icon: pystray.Icon, _item: Any) -> None:
        snapshot = state.snapshot()
        catalog_error = snapshot.get("catalog_error")
        if isinstance(catalog_error, str) and catalog_error:
            show_message(
                "Codex Provider Apply",
                f"当前 provider 列表不可用：{catalog_error}",
                MB_ICONERROR,
            )
            return

        selected_provider_id = snapshot.get("selected_provider_id")
        if not isinstance(selected_provider_id, str) or not selected_provider_id:
            show_message(
                "Codex Provider Apply",
                "当前没有可用的 source provider 可供应用。",
                MB_ICONWARNING,
            )
            return
        selected_provider = next(
            (
                provider
                for provider in snapshot.get("providers", [])
                if provider.get("provider_id") == selected_provider_id
            ),
            None,
        )
        if not isinstance(selected_provider, dict):
            show_message(
                "Codex Provider Apply",
                "当前所选 provider 不存在于最新 provider 列表中，请先重新加载。",
                MB_ICONERROR,
            )
            return
        missing_fields: list[str] = []
        if not bool(selected_provider.get("has_base_url")):
            missing_fields.append("base_url")
        if not bool(selected_provider.get("has_experimental_bearer_token")):
            missing_fields.append("experimental_bearer_token")
        if missing_fields:
            show_message(
                "Codex Provider Apply",
                "当前所选 provider 缺少必需字段："
                + ", ".join(missing_fields),
                MB_ICONERROR,
            )
            return

        def worker() -> None:
            summary = apply_selected_provider_to_all_instances(selected_provider_id)
            reload_catalog()
            new_snapshot = state.snapshot()
            rebuild_menu(icon)
            title, message, icon_flag = format_apply_summary(summary, new_snapshot)
            show_message(title, message, icon_flag)

        threading.Thread(target=worker, daemon=True).start()

    def handle_select_provider(icon: pystray.Icon, provider_id: str) -> None:
        state.set_selected_provider(provider_id)
        rebuild_menu(icon)

    def noop(_icon: pystray.Icon, _item: Any) -> None:
        return

    def build_provider_menu(icon: pystray.Icon) -> pystray.Menu:
        snapshot = state.snapshot()
        catalog_error = snapshot.get("catalog_error")
        if isinstance(catalog_error, str) and catalog_error:
            return pystray.Menu(
                pystray.MenuItem(
                    f"provider 列表不可用：{short_error_text(catalog_error, 64)}",
                    noop,
                    enabled=False,
                )
            )
        providers = snapshot["providers"]
        if not providers:
            return pystray.Menu(
                pystray.MenuItem("无可用 provider", noop, enabled=False)
            )

        items = []
        for provider in providers:
            provider_id = provider["provider_id"]
            suffix_parts = []
            if provider["has_base_url"]:
                suffix_parts.append("base_url")
            if provider["has_experimental_bearer_token"]:
                suffix_parts.append("token")
            suffix = f" [{' + '.join(suffix_parts)}]" if suffix_parts else " [无两字段值]"
            label = f"{provider['display_name']} ({provider_id}){suffix}"
            items.append(
                pystray.MenuItem(
                    label,
                    lambda tray_icon, _item, pid=provider_id: handle_select_provider(tray_icon, pid),
                    checked=lambda item, pid=provider_id: state.snapshot()["selected_provider_id"] == pid,
                    radio=True,
                )
            )
        return pystray.Menu(*items)

    def build_menu(icon: pystray.Icon) -> pystray.Menu:
        snapshot = state.snapshot()
        current_model_provider_id = snapshot.get("current_model_provider_id") or "未知"
        writable_label = "可写" if snapshot.get("current_model_provider_writable") else "只读"
        catalog_error = snapshot.get("catalog_error")
        catalog_status_label = (
            f"provider 列表错误: {short_error_text(catalog_error, 56)}"
            if isinstance(catalog_error, str) and catalog_error
            else "provider 列表状态: 已同步"
        )
        can_apply = (
            not (isinstance(catalog_error, str) and catalog_error)
            and bool(snapshot.get("selected_provider_id"))
            and bool(snapshot.get("providers"))
        )
        return pystray.Menu(
            pystray.MenuItem("刷新全部 app-server", handle_refresh),
            pystray.MenuItem(
                f"当前 model_provider: {current_model_provider_id} ({writable_label})",
                noop,
                enabled=False,
            ),
            pystray.MenuItem(catalog_status_label, noop, enabled=False),
            pystray.MenuItem("选择 source provider", build_provider_menu(icon)),
            pystray.MenuItem(
                "应用所选 provider 到当前 model_provider 并刷新",
                handle_apply,
                enabled=can_apply,
            ),
            pystray.MenuItem(
                "退出",
                lambda tray_icon, _item: tray_icon.stop(),
            ),
        )

    icon = pystray.Icon(
        "codex-app-server-refresh",
        build_image(),
        "Codex App Server Refresh",
    )
    icon.menu = build_menu(icon)
    return icon


def main() -> None:
    if os.name != "nt":
        raise SystemExit("windows_app_server_refresh_tray.py only supports Windows")

    icon = create_tray_icon()
    icon.run()


if __name__ == "__main__":
    main()
