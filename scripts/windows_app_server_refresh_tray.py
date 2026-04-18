from __future__ import annotations

import ctypes
import datetime as dt
import json
import os
import re
import tempfile
import threading
import tomllib
from ctypes import wintypes
from pathlib import Path
from typing import Any
from typing import Callable

APP_SERVERS_DIR_NAME = "app_servers"
CONFIG_TOML_FILE_NAME = "config.toml"
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
STANDARD_PROVIDER_ID_RE = re.compile(r"^[A-Za-z0-9_-]+$")
TABLE_HEADER_RE = re.compile(r"^\s*\[(?P<name>[^\[\]]+)\]\s*(?:#.*)?$")
ARRAY_TABLE_HEADER_RE = re.compile(r"^\s*\[\[(?P<name>[^\[\]]+)\]\]\s*(?:#.*)?$")
ROOT_PROVIDER_KEY_RE = re.compile(
    r"^(?P<indent>\s*)(?P<key>base_url|experimental_bearer_token)\s*="
)


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
        self.config_path: str | None = None
        self.catalog_error: str | None = None

    def snapshot(self) -> dict[str, Any]:
        with self._lock:
            return {
                "providers": [provider.copy() for provider in self.providers],
                "selected_provider_id": self.selected_provider_id,
                "current_model_provider_id": self.current_model_provider_id,
                "config_path": self.config_path,
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
        if not isinstance(current_model_provider_id, str) or not current_model_provider_id:
            current_model_provider_id = None

        config_path = response.get("config_path")
        if not isinstance(config_path, str) or not config_path:
            config_path = str(config_toml_path())

        error_message = response.get("catalog_error")
        if not isinstance(error_message, str) or not error_message:
            error_message = None

        with self._lock:
            self.providers = normalized
            self.current_model_provider_id = current_model_provider_id
            self.config_path = config_path
            self.catalog_error = error_message
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
            self.config_path = str(config_toml_path())
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


def config_toml_path(codex_home: Path | None = None) -> Path:
    root = codex_home or default_codex_home()
    return root / CONFIG_TOML_FILE_NAME


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


def normalize_string(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    return stripped or None


def load_user_provider_catalog(codex_home: Path | None = None) -> dict[str, Any]:
    config_path = config_toml_path(codex_home)
    response = {
        "config_path": str(config_path),
        "providers": [],
        "current_model_provider_id": None,
        "catalog_error": None,
    }

    try:
        raw_text = config_path.read_text(encoding="utf-8-sig")
    except FileNotFoundError:
        response["catalog_error"] = "config.toml 不存在"
        return response
    except UnicodeDecodeError:
        response["catalog_error"] = "config.toml 不是有效的 UTF-8"
        return response
    except OSError as exc:
        response["catalog_error"] = f"读取 config.toml 失败: {exc}"
        return response

    try:
        payload = tomllib.loads(raw_text)
    except tomllib.TOMLDecodeError:
        response["catalog_error"] = "config.toml 语法无效"
        return response

    model_providers = payload.get("model_providers")
    if model_providers is None:
        model_providers = {}
    if not isinstance(model_providers, dict):
        response["catalog_error"] = "config.toml 结构不支持自动写回"
        return response

    providers = []
    for provider_id, provider in sorted(model_providers.items()):
        if not isinstance(provider_id, str) or not provider_id:
            continue
        if not isinstance(provider, dict):
            continue
        display_name = provider.get("name")
        providers.append(
            {
                "provider_id": provider_id,
                "display_name": display_name
                if isinstance(display_name, str) and display_name
                else provider_id,
                "has_base_url": normalize_string(provider.get("base_url")) is not None,
                "has_experimental_bearer_token": normalize_string(
                    provider.get("experimental_bearer_token")
                )
                is not None,
            }
        )
    response["providers"] = providers

    errors: list[str] = []
    current_model_provider_id = normalize_string(payload.get("model_provider"))
    if current_model_provider_id is None:
        errors.append("顶层 model_provider 未配置")
    else:
        response["current_model_provider_id"] = current_model_provider_id
        if current_model_provider_id not in model_providers:
            errors.append("target provider 条目不存在")

    if errors:
        response["catalog_error"] = "；".join(errors)
    return response


def parse_config_for_apply(
    codex_home: Path | None = None,
) -> tuple[Path, str, dict[str, Any]]:
    config_path = config_toml_path(codex_home)
    try:
        raw_text = config_path.read_text(encoding="utf-8-sig")
    except FileNotFoundError as exc:
        raise RuntimeError("config.toml 不存在") from exc
    except UnicodeDecodeError as exc:
        raise RuntimeError("config.toml 不是有效的 UTF-8") from exc
    except OSError as exc:
        raise RuntimeError(f"读取 config.toml 失败: {exc}") from exc

    try:
        payload = tomllib.loads(raw_text)
    except tomllib.TOMLDecodeError as exc:
        raise RuntimeError("config.toml 语法无效") from exc

    if not isinstance(payload, dict):
        raise RuntimeError("config.toml 结构不支持自动写回")

    return config_path, raw_text, payload


def toml_quote(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def detect_newline(text: str) -> str:
    return "\r\n" if "\r\n" in text else "\n"


def table_header_name(line: str) -> str | None:
    match = TABLE_HEADER_RE.match(line)
    if match is None:
        return None
    return match.group("name").strip()


def array_table_header_name(line: str) -> str | None:
    match = ARRAY_TABLE_HEADER_RE.match(line)
    if match is None:
        return None
    return match.group("name").strip()


def find_provider_table_bounds(lines: list[str], provider_id: str) -> tuple[int, int]:
    if STANDARD_PROVIDER_ID_RE.fullmatch(provider_id) is None:
        raise RuntimeError("config.toml 结构不支持自动写回")

    root_table_name = f"model_providers.{provider_id}"
    root_indices: list[int] = []

    for index, line in enumerate(lines):
        array_name = array_table_header_name(line)
        if array_name == root_table_name or (
            isinstance(array_name, str) and array_name.startswith(f"{root_table_name}.")
        ):
            raise RuntimeError("config.toml 结构不支持自动写回")

        header_name = table_header_name(line)
        if isinstance(header_name, str) and header_name.startswith(f"{root_table_name}."):
            raise RuntimeError("config.toml 结构不支持自动写回")
        if header_name == root_table_name:
            root_indices.append(index)

    if len(root_indices) != 1:
        raise RuntimeError("config.toml 结构不支持自动写回")

    start_index = root_indices[0]
    end_index = len(lines)
    for index in range(start_index + 1, len(lines)):
        if table_header_name(lines[index]) is not None or array_table_header_name(lines[index]) is not None:
            end_index = index
            break
    return start_index, end_index


def rewrite_provider_runtime_section(
    raw_text: str,
    provider_id: str,
    base_url: str,
    bearer_token: str,
) -> str:
    lines = raw_text.splitlines(keepends=True)
    newline = detect_newline(raw_text)
    start_index, end_index = find_provider_table_bounds(lines, provider_id)

    prefix_lines = lines[: start_index + 1]
    if prefix_lines and not prefix_lines[-1].endswith(("\n", "\r")):
        prefix_lines[-1] = prefix_lines[-1] + newline

    body_lines = lines[start_index + 1 : end_index]
    indent = ""
    seen_keys: set[str] = set()
    updated_body_lines: list[str] = []

    for line in body_lines:
        match = ROOT_PROVIDER_KEY_RE.match(line)
        if match is None:
            updated_body_lines.append(line)
            if not indent and line.strip() and not line.lstrip().startswith("#"):
                indent_match = re.match(r"^\s*", line)
                indent = indent_match.group(0) if indent_match is not None else ""
            continue

        current_indent = match.group("indent")
        key = match.group("key")
        seen_keys.add(key)
        if not indent:
            indent = current_indent
        if key == "base_url":
            updated_body_lines.append(
                f"{current_indent}base_url = {toml_quote(base_url)}{newline}"
            )
        else:
            updated_body_lines.append(
                f"{current_indent}experimental_bearer_token = {toml_quote(bearer_token)}{newline}"
            )

    if "base_url" not in seen_keys:
        updated_body_lines.append(f"{indent}base_url = {toml_quote(base_url)}{newline}")
    if "experimental_bearer_token" not in seen_keys:
        updated_body_lines.append(
            f"{indent}experimental_bearer_token = {toml_quote(bearer_token)}{newline}"
        )

    updated_text = "".join(prefix_lines + updated_body_lines + lines[end_index:])
    try:
        tomllib.loads(updated_text)
    except tomllib.TOMLDecodeError as exc:
        raise RuntimeError("config.toml 结构不支持自动写回") from exc
    return updated_text


def atomic_write_utf8(path: Path, content: str) -> None:
    temp_fd, temp_name = tempfile.mkstemp(
        prefix=f"{path.name}.",
        suffix=".tmp",
        dir=str(path.parent),
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(temp_fd, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
        os.replace(temp_path, path)
    except Exception:
        remove_file_if_exists(temp_path)
        raise


def apply_selected_provider_to_config(
    source_provider_id: str,
    codex_home: Path | None = None,
    registry_dir: Path | None = None,
) -> dict[str, Any]:
    source_provider_id = source_provider_id.strip()
    if not source_provider_id:
        return {
            "ok": False,
            "message": "source provider 不存在",
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "refresh_summary": None,
        }

    try:
        config_path, raw_text, payload = parse_config_for_apply(codex_home)
    except RuntimeError as exc:
        return {
            "ok": False,
            "message": str(exc),
            "source_provider_id": source_provider_id,
            "current_model_provider_id": None,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "refresh_summary": None,
        }

    model_providers = payload.get("model_providers")
    if not isinstance(model_providers, dict):
        return {
            "ok": False,
            "message": "config.toml 结构不支持自动写回",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": None,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    current_model_provider_id = normalize_string(payload.get("model_provider"))
    if current_model_provider_id is None:
        return {
            "ok": False,
            "message": "顶层 model_provider 未配置",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": None,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    source_provider = model_providers.get(source_provider_id)
    if not isinstance(source_provider, dict):
        return {
            "ok": False,
            "message": "source provider 不存在",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    source_base_url = normalize_string(source_provider.get("base_url"))
    if source_base_url is None:
        return {
            "ok": False,
            "message": "source provider 缺少 base_url",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    source_bearer_token = normalize_string(
        source_provider.get("experimental_bearer_token")
    )
    if source_bearer_token is None:
        return {
            "ok": False,
            "message": "source provider 缺少 experimental_bearer_token",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    target_provider = model_providers.get(current_model_provider_id)
    if target_provider is None:
        return {
            "ok": False,
            "message": "target provider 条目不存在",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }
    if not isinstance(target_provider, dict):
        return {
            "ok": False,
            "message": "config.toml 结构不支持自动写回",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "refresh_summary": None,
        }

    target_base_url = normalize_string(target_provider.get("base_url"))
    target_bearer_token = normalize_string(target_provider.get("experimental_bearer_token"))
    config_changed = not (
        target_base_url == source_base_url and target_bearer_token == source_bearer_token
    )

    if config_changed:
        try:
            updated_text = rewrite_provider_runtime_section(
                raw_text,
                current_model_provider_id,
                source_base_url,
                source_bearer_token,
            )
            atomic_write_utf8(config_path, updated_text)
        except RuntimeError as exc:
            return {
                "ok": False,
                "message": str(exc),
                "source_provider_id": source_provider_id,
                "current_model_provider_id": current_model_provider_id,
                "config_path": str(config_path),
                "config_changed": False,
                "refresh_summary": None,
            }
        except OSError as exc:
            return {
                "ok": False,
                "message": f"写入 config.toml 失败: {exc}",
                "source_provider_id": source_provider_id,
                "current_model_provider_id": current_model_provider_id,
                "config_path": str(config_path),
                "config_changed": False,
                "refresh_summary": None,
            }

    refresh_summary = refresh_all_instances(registry_dir=registry_dir)
    return {
        "ok": True,
        "message": None,
        "source_provider_id": source_provider_id,
        "current_model_provider_id": current_model_provider_id,
        "config_path": str(config_path),
        "config_changed": config_changed,
        "refresh_summary": refresh_summary,
    }


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




def format_refresh_summary(summary: dict[str, Any]) -> tuple[str, str, int]:
    total_instances = int(summary.get("total_instances", 0))
    success_instances = int(summary.get("success_instances", 0))
    failed_instances = int(summary.get("failed_instances", 0))
    applied_threads = int(summary.get("applied_threads", 0))
    queued_threads = int(summary.get("queued_threads", 0))
    failed_threads = int(summary.get("failed_threads", 0))
    title = "Codex App Server Refresh"
    icon_flag = MB_ICONINFORMATION if failed_instances == 0 else MB_ICONWARNING
    if total_instances == 0:
        message = "未发现 live app-server 实例。\n\n实例总数: 0"
    else:
        message = (
            "刷新全部 app-server 完成\n\n"
            f"实例总数: {total_instances}\n"
            f"成功实例: {success_instances}\n"
            f"失败实例: {failed_instances}\n"
            f"Applied 线程: {applied_threads}\n"
            f"Queued 线程: {queued_threads}\n"
            f"Failed 线程: {failed_threads}"
        )
    return title, message, icon_flag


def format_apply_summary(summary: dict[str, Any]) -> tuple[str, str, int]:
    title = "Codex Provider Apply"
    if not bool(summary.get("ok")):
        message = str(summary.get("message") or "应用 provider 失败")
        return title, message, MB_ICONERROR

    refresh_summary = summary.get("refresh_summary") or {}
    total_instances = int(refresh_summary.get("total_instances", 0))
    success_instances = int(refresh_summary.get("success_instances", 0))
    failed_instances = int(refresh_summary.get("failed_instances", 0))
    applied_threads = int(refresh_summary.get("applied_threads", 0))
    queued_threads = int(refresh_summary.get("queued_threads", 0))
    failed_threads = int(refresh_summary.get("failed_threads", 0))
    config_changed = bool(summary.get("config_changed"))
    source_provider_id = str(summary.get("source_provider_id") or "unknown")
    current_model_provider_id = str(
        summary.get("current_model_provider_id") or "unknown"
    )
    config_path = str(summary.get("config_path") or config_toml_path())

    if total_instances == 0:
        headline = (
            "已写入 target provider，两字段已更新，未刷新任何实例"
            if config_changed
            else "target provider 已是所选值，未修改配置，未刷新任何实例"
        )
        icon_flag = MB_ICONINFORMATION
    elif failed_instances == 0:
        headline = (
            "已写入 target provider，并已刷新实例"
            if config_changed
            else "target provider 已是所选值，未修改配置，已刷新实例"
        )
        icon_flag = MB_ICONINFORMATION
    else:
        headline = (
            "已写入 target provider，但实例刷新部分失败"
            if config_changed
            else "target provider 已是所选值，未修改配置，但实例刷新部分失败"
        )
        icon_flag = MB_ICONWARNING

    lines = [
        headline,
        "",
        f"source provider: {source_provider_id}",
        f"target model_provider: {current_model_provider_id}",
        f"配置文件: {config_path}",
    ]
    if total_instances > 0:
        lines.extend(
            [
                "",
                f"实例总数: {total_instances}",
                f"成功实例: {success_instances}",
                f"失败实例: {failed_instances}",
                f"Applied 线程: {applied_threads}",
                f"Queued 线程: {queued_threads}",
                f"Failed 线程: {failed_threads}",
            ]
        )
    if failed_instances > 0:
        failed_detail_lines: list[str] = []
        details = refresh_summary.get("details")
        if isinstance(details, list):
            for detail in details:
                if not isinstance(detail, dict) or bool(detail.get("ok")):
                    continue
                instance_id = str(detail.get("instance_id") or "unknown")
                error_text = detail.get("error")
                if isinstance(error_text, str) and error_text:
                    failed_detail_lines.append(
                        f"- {instance_id}: {short_error_text(error_text, 120)}"
                    )
                    continue

                response = detail.get("response")
                if isinstance(response, dict):
                    failed_threads_value = response.get("failed_threads")
                    if isinstance(failed_threads_value, list) and failed_threads_value:
                        failed_detail_lines.append(
                            f"- {instance_id}: failed_threads={len(failed_threads_value)}"
                        )
                        continue
                failed_detail_lines.append(f"- {instance_id}: refresh 返回失败")

        if failed_detail_lines:
            lines.extend(["", "刷新失败明细:"])
            lines.extend(failed_detail_lines[:5])
            remaining = len(failed_detail_lines) - 5
            if remaining > 0:
                lines.append(f"... 另有 {remaining} 个失败实例")
    return title, "\n".join(lines), icon_flag


def create_tray_icon():
    import pystray
    from PIL import Image
    from PIL import ImageDraw

    state = TrayState()
    state.set_catalog(load_user_provider_catalog())

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
        state.set_catalog(load_user_provider_catalog())

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
                "当前所选 source provider 不存在，请先重新加载配置。",
                MB_ICONERROR,
            )
            return

        def worker() -> None:
            summary = apply_selected_provider_to_config(selected_provider_id)
            reload_catalog()
            rebuild_menu(icon)
            title, message, icon_flag = format_apply_summary(summary)
            show_message(title, message, icon_flag)

        threading.Thread(target=worker, daemon=True).start()

    def handle_select_provider(icon: pystray.Icon, provider_id: str) -> None:
        state.set_selected_provider(provider_id)
        rebuild_menu(icon)

    def select_provider_action(provider_id: str) -> Callable[[pystray.Icon, Any], None]:
        def action(tray_icon: pystray.Icon, _item: Any) -> None:
            handle_select_provider(tray_icon, provider_id)

        return action

    def provider_checked(provider_id: str) -> Callable[[Any], bool]:
        def checked(_item: Any) -> bool:
            return state.snapshot()["selected_provider_id"] == provider_id

        return checked

    def noop(_icon: pystray.Icon, _item: Any) -> None:
        return

    def build_provider_menu(icon: pystray.Icon) -> pystray.Menu:
        snapshot = state.snapshot()
        providers = snapshot["providers"]
        if not providers:
            catalog_error = snapshot.get("catalog_error")
            label = (
                f"config 错误: {short_error_text(catalog_error, 64)}"
                if isinstance(catalog_error, str) and catalog_error
                else "无可用 source provider"
            )
            return pystray.Menu(
                pystray.MenuItem(label, noop, enabled=False)
            )

        items = []
        for provider in providers:
            provider_id = provider["provider_id"]
            suffix_parts = []
            if provider["has_base_url"]:
                suffix_parts.append("base_url")
            if provider["has_experimental_bearer_token"]:
                suffix_parts.append("token")
            suffix = (
                f" [{' + '.join(suffix_parts)}]"
                if suffix_parts
                else " [缺少两字段]"
            )
            label = f"{provider['display_name']} ({provider_id}){suffix}"
            items.append(
                pystray.MenuItem(
                    label,
                    select_provider_action(provider_id),
                    checked=provider_checked(provider_id),
                    radio=True,
                )
            )
        return pystray.Menu(*items)

    def build_menu(icon: pystray.Icon) -> pystray.Menu:
        snapshot = state.snapshot()
        current_model_provider_id = snapshot.get("current_model_provider_id") or "未配置"
        config_path = snapshot.get("config_path") or str(config_toml_path())
        catalog_error = snapshot.get("catalog_error")
        catalog_status_label = (
            f"config 错误: {short_error_text(catalog_error, 56)}"
            if isinstance(catalog_error, str) and catalog_error
            else "source provider 列表: 已加载"
        )
        can_apply = bool(snapshot.get("selected_provider_id")) and bool(
            snapshot.get("providers")
        )
        return pystray.Menu(
            pystray.MenuItem("刷新全部 app-server", handle_refresh),
            pystray.MenuItem(
                f"当前 target model_provider: {current_model_provider_id}",
                noop,
                enabled=False,
            ),
            pystray.MenuItem(
                f"配置文件: {config_path}",
                noop,
                enabled=False,
            ),
            pystray.MenuItem(catalog_status_label, noop, enabled=False),
            pystray.MenuItem("选择 source provider", build_provider_menu(icon)),
            pystray.MenuItem(
                "应用所选 provider 到当前 target 并刷新",
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
        "Codex Provider Refresh",
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
