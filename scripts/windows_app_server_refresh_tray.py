from __future__ import annotations

import ctypes
import datetime as dt
import json
import os
import queue
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
SMART_APPLY_PROVIDER_OP = "apply_provider_runtime_from_effective_provider"
REFRESH_ALL_LOADED_THREADS_OP = "refresh_all_loaded_threads"
REFRESH_CONSOLE_LOADED_THREADS_OP = "refresh_console_loaded_threads"
REFRESH_APP_SERVER_LOADED_THREADS_OP = "refresh_app_server_loaded_threads"
REFRESH_SCOPE_ALL = "all"
REFRESH_SCOPE_CONSOLE = "console"
REFRESH_SCOPE_APP_SERVER = "appServer"
REFRESH_SCOPE_LABELS = {
    REFRESH_SCOPE_ALL: "刷新全部 app-server",
    REFRESH_SCOPE_CONSOLE: "只刷新 Codex 控制台",
    REFRESH_SCOPE_APP_SERVER: "只刷新 Windows App app-server",
}
STANDARD_PROVIDER_ID_RE = re.compile(r"^[A-Za-z0-9_-]+$")
TABLE_HEADER_RE = re.compile(r"^\s*\[(?P<name>[^\[\]]+)\]\s*(?:#.*)?$")
ARRAY_TABLE_HEADER_RE = re.compile(r"^\s*\[\[(?P<name>[^\[\]]+)\]\]\s*(?:#.*)?$")
ROOT_PROVIDER_KEY_RE = re.compile(
    r"^(?P<indent>\s*)(?P<key>base_url|experimental_bearer_token)\s*="
)
ROOT_SCALAR_KEY_RE = re.compile(
    r"^(?P<indent>\s*)(?P<key>force_service_tier_priority|service_tier)\s*="
)
FEATURES_KEY_RE = re.compile(r"^(?P<indent>\s*)(?P<key>fast_mode)\s*=")


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
                    "base_url": normalize_string(provider.get("base_url")),
                    "experimental_bearer_token": normalize_string(
                        provider.get("experimental_bearer_token")
                    ),
                    "has_base_url": bool(provider.get("has_base_url")),
                    "has_experimental_bearer_token": bool(
                        provider.get("has_experimental_bearer_token")
                    ),
                    "requires_openai_auth": bool(provider.get("requires_openai_auth")),
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
        base_url = normalize_string(provider.get("base_url"))
        bearer_token = normalize_string(provider.get("experimental_bearer_token"))
        providers.append(
            {
                "provider_id": provider_id,
                "display_name": display_name
                if isinstance(display_name, str) and display_name
                else provider_id,
                "base_url": base_url,
                "experimental_bearer_token": bearer_token,
                "has_base_url": base_url is not None,
                "has_experimental_bearer_token": bearer_token is not None,
                "requires_openai_auth": bool(provider.get("requires_openai_auth")),
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


def toml_quote_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return toml_quote(value)
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


def find_table_bounds(lines: list[str], table_name: str) -> tuple[int, int]:
    table_indices = [
        index
        for index, line in enumerate(lines)
        if table_header_name(line) == table_name
    ]
    if len(table_indices) != 1:
        raise RuntimeError("config.toml 结构不支持自动写回")

    start_index = table_indices[0]
    end_index = len(lines)
    for index in range(start_index + 1, len(lines)):
        if table_header_name(lines[index]) is not None or array_table_header_name(lines[index]) is not None:
            end_index = index
            break
    return start_index, end_index


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


def rewrite_root_scalar_fields(
    raw_text: str,
    fields: dict[str, Any],
) -> str:
    if not fields:
        return raw_text
    lines = raw_text.splitlines(keepends=True)
    newline = detect_newline(raw_text)
    seen_keys: set[str] = set()
    output: list[str] = []
    insert_index = 0

    for line in lines:
        if table_header_name(line) is not None or array_table_header_name(line) is not None:
            break
        insert_index += 1

    for index, line in enumerate(lines):
        match = ROOT_SCALAR_KEY_RE.match(line)
        if match is not None and match.group("key") in fields and index < insert_index:
            key = match.group("key")
            seen_keys.add(key)
            output.append(f"{match.group('indent')}{key} = {toml_quote_value(fields[key])}{newline}")
        else:
            output.append(line)

    additions = [
        f"{key} = {toml_quote_value(value)}{newline}"
        for key, value in fields.items()
        if key not in seen_keys
    ]
    if additions:
        output[insert_index:insert_index] = additions
    updated_text = "".join(output)
    try:
        tomllib.loads(updated_text)
    except tomllib.TOMLDecodeError as exc:
        raise RuntimeError("config.toml 结构不支持自动写回") from exc
    return updated_text


def rewrite_features_fields(raw_text: str, fields: dict[str, Any]) -> str:
    if not fields:
        return raw_text
    lines = raw_text.splitlines(keepends=True)
    newline = detect_newline(raw_text)
    try:
        start_index, end_index = find_table_bounds(lines, "features")
    except RuntimeError:
        insertion = ["[features]" + newline] + [
            f"{key} = {toml_quote_value(value)}{newline}" for key, value in fields.items()
        ]
        if lines and lines[-1].strip():
            insertion.insert(0, newline)
        updated_text = "".join(lines + insertion)
        try:
            tomllib.loads(updated_text)
        except tomllib.TOMLDecodeError as exc:
            raise RuntimeError("config.toml 结构不支持自动写回") from exc
        return updated_text

    prefix_lines = lines[: start_index + 1]
    if prefix_lines and not prefix_lines[-1].endswith(("\n", "\r")):
        prefix_lines[-1] = prefix_lines[-1] + newline
    body_lines = lines[start_index + 1 : end_index]
    seen_keys: set[str] = set()
    updated_body_lines: list[str] = []

    for line in body_lines:
        match = FEATURES_KEY_RE.match(line)
        if match is not None and match.group("key") in fields:
            key = match.group("key")
            seen_keys.add(key)
            updated_body_lines.append(
                f"{match.group('indent')}{key} = {toml_quote_value(fields[key])}{newline}"
            )
        else:
            updated_body_lines.append(line)

    for key, value in fields.items():
        if key not in seen_keys:
            updated_body_lines.append(f"{key} = {toml_quote_value(value)}{newline}")
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
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
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
    source_force_service_tier_priority = bool(
        payload.get("force_service_tier_priority", True)
    )
    source_service_tier = normalize_string(payload.get("service_tier"))
    features = payload.get("features")
    source_fast_mode = bool(features.get("fast_mode")) if isinstance(features, dict) else False
    root_scalar_fields = {
        "force_service_tier_priority": source_force_service_tier_priority,
        **({"service_tier": source_service_tier} if source_service_tier is not None else {}),
    }
    provider_runtime_changed = not (
        target_base_url == source_base_url and target_bearer_token == source_bearer_token
    )

    try:
        updated_text = raw_text
        if provider_runtime_changed:
            updated_text = rewrite_provider_runtime_section(
                updated_text,
                current_model_provider_id,
                source_base_url,
                source_bearer_token,
            )
        updated_text = rewrite_root_scalar_fields(updated_text, root_scalar_fields)
        updated_text = rewrite_features_fields(updated_text, {"fast_mode": source_fast_mode})
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

    config_changed = updated_text != raw_text
    if config_changed:
        try:
            atomic_write_utf8(config_path, updated_text)
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

    refresh_summary = refresh_all_instances(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
        send_request=send_request,
    )
    return {
        "ok": True,
        "message": None,
        "source_provider_id": source_provider_id,
        "current_model_provider_id": current_model_provider_id,
        "config_path": str(config_path),
        "config_changed": config_changed,
        "refresh_summary": refresh_summary,
    }


def apply_runtime_values_to_current_provider(
    base_url: str,
    bearer_token: str,
    codex_home: Path | None = None,
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
) -> dict[str, Any]:
    next_base_url = normalize_string(base_url)
    if next_base_url is None:
        return {
            "ok": False,
            "message": "新的 Base URL 不能为空",
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    next_bearer_token = normalize_string(bearer_token)
    if next_bearer_token is None:
        return {
            "ok": False,
            "message": "新的 Token 不能为空",
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    try:
        config_path, raw_text, payload = parse_config_for_apply(codex_home)
    except RuntimeError as exc:
        return {
            "ok": False,
            "message": str(exc),
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    model_providers = payload.get("model_providers")
    if not isinstance(model_providers, dict):
        return {
            "ok": False,
            "message": "config.toml 结构不支持自动写回",
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_path),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    current_model_provider_id = normalize_string(payload.get("model_provider"))
    if current_model_provider_id is None:
        return {
            "ok": False,
            "message": "顶层 model_provider 未配置",
            "source_provider_id": None,
            "current_model_provider_id": None,
            "config_path": str(config_path),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    target_provider = model_providers.get(current_model_provider_id)
    if target_provider is None:
        return {
            "ok": False,
            "message": f"当前 provider「{current_model_provider_id}」不存在",
            "source_provider_id": None,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }
    if not isinstance(target_provider, dict):
        return {
            "ok": False,
            "message": "config.toml 结构不支持自动写回",
            "source_provider_id": None,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    current_base_url = normalize_string(target_provider.get("base_url"))
    current_bearer_token = normalize_string(
        target_provider.get("experimental_bearer_token")
    )
    runtime_changed = not (
        current_base_url == next_base_url and current_bearer_token == next_bearer_token
    )

    try:
        updated_text = (
            rewrite_provider_runtime_section(
                raw_text,
                current_model_provider_id,
                next_base_url,
                next_bearer_token,
            )
            if runtime_changed
            else raw_text
        )
    except RuntimeError as exc:
        return {
            "ok": False,
            "message": str(exc),
            "source_provider_id": None,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_path),
            "config_changed": False,
            "apply_strategy": "manual_runtime_values",
            "refresh_summary": None,
        }

    config_changed = updated_text != raw_text
    if config_changed:
        try:
            atomic_write_utf8(config_path, updated_text)
        except OSError as exc:
            return {
                "ok": False,
                "message": f"写入 config.toml 失败: {exc}",
                "source_provider_id": None,
                "current_model_provider_id": current_model_provider_id,
                "config_path": str(config_path),
                "config_changed": False,
                "apply_strategy": "manual_runtime_values",
                "refresh_summary": None,
            }

    refresh_summary = refresh_all_instances(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
        send_request=send_request,
    )
    return {
        "ok": True,
        "message": None,
        "source_provider_id": None,
        "current_model_provider_id": current_model_provider_id,
        "config_path": str(config_path),
        "config_changed": config_changed,
        "apply_strategy": "manual_runtime_values",
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


def unsupported_control_operation(response: dict[str, Any], op: str) -> bool:
    if bool(response.get("ok")):
        return False
    for key in ("error", "message"):
        value = response.get(key)
        if isinstance(value, str) and "unsupported control operation" in value and op in value:
            return True
    return False


def smart_apply_wrote_config(response: dict[str, Any]) -> bool:
    return response.get("outcome") in {"success", "partial_failure"}


def empty_refresh_summary(total_instances: int = 0) -> dict[str, Any]:
    return {
        "total_instances": total_instances,
        "success_instances": 0,
        "failed_instances": 0,
        "applied_threads": 0,
        "queued_threads": 0,
        "failed_threads": 0,
        "details": [],
        "smart_apply_instances": 0,
        "fallback_instances": 0,
    }


def add_refresh_response_counts(
    summary: dict[str, Any],
    response: dict[str, Any],
) -> bool:
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
    return instance_ok


def registrations_for_refresh(
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
) -> list[dict[str, Any]]:
    return enumerate_live_registrations(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
    )


def refresh_registrations(
    registrations: list[dict[str, Any]],
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
    *,
    op: str = REFRESH_ALL_LOADED_THREADS_OP,
    scope: str = REFRESH_SCOPE_ALL,
    method: str | None = None,
) -> dict[str, Any]:
    summary = empty_refresh_summary(len(registrations))
    summary["scope"] = scope
    method_name = method or op

    for registration in registrations:
        instance_id = str(registration["instance_id"])
        endpoint = str(registration["control_endpoint"])
        try:
            response = send_request(endpoint, {"op": op, "scope": scope})
            instance_ok = add_refresh_response_counts(summary, response)
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": instance_ok,
                    "method": method_name,
                    "scope": scope,
                    "response": response,
                }
            )
        except Exception as exc:
            summary["failed_instances"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": False,
                    "method": method_name,
                    "scope": scope,
                    "error": str(exc),
                }
            )

    return summary


def merge_refresh_summaries(
    left: dict[str, Any],
    right: dict[str, Any],
) -> dict[str, Any]:
    merged = left.copy()
    merged["total_instances"] = int(left.get("total_instances", 0))
    for key in (
        "success_instances",
        "failed_instances",
        "applied_threads",
        "queued_threads",
        "failed_threads",
        "smart_apply_instances",
        "fallback_instances",
    ):
        merged[key] = int(left.get(key, 0)) + int(right.get(key, 0))
    merged["details"] = list(left.get("details") or []) + list(right.get("details") or [])
    return merged


def refresh_all_instances(
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
    *,
    scope: str = REFRESH_SCOPE_ALL,
    op: str = REFRESH_ALL_LOADED_THREADS_OP,
) -> dict[str, Any]:
    registrations = registrations_for_refresh(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
    )
    return refresh_registrations(registrations, send_request, op=op, scope=scope)


def refresh_console_instances(
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
) -> dict[str, Any]:
    return refresh_all_instances(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
        send_request=send_request,
        scope=REFRESH_SCOPE_CONSOLE,
        op=REFRESH_CONSOLE_LOADED_THREADS_OP,
    )


def refresh_app_server_instances(
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
) -> dict[str, Any]:
    return refresh_all_instances(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
        send_request=send_request,
        scope=REFRESH_SCOPE_APP_SERVER,
        op=REFRESH_APP_SERVER_LOADED_THREADS_OP,
    )


def apply_provider_runtime_smart_first(
    source_provider_id: str,
    codex_home: Path | None = None,
    registry_dir: Path | None = None,
    pid_checker: Callable[[int], bool] = is_pid_alive,
    ping_checker: Callable[[str], bool] = ping_instance,
    send_request: Callable[[str, dict[str, Any]], dict[str, Any]] = send_control_request,
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
            "apply_strategy": "invalid_source_provider",
            "refresh_summary": None,
        }

    registrations = registrations_for_refresh(
        registry_dir=registry_dir,
        pid_checker=pid_checker,
        ping_checker=ping_checker,
    )
    if not registrations:
        legacy = apply_selected_provider_to_config(
            source_provider_id,
            codex_home=codex_home,
            registry_dir=registry_dir,
            pid_checker=pid_checker,
            ping_checker=ping_checker,
            send_request=send_request,
        )
        legacy["apply_strategy"] = "legacy_config_write_no_live_instances"
        return legacy

    summary = {
        **empty_refresh_summary(len(registrations)),
        "smart_apply_instances": 0,
        "fallback_instances": 0,
    }
    unsupported_registrations: list[dict[str, Any]] = []
    current_model_provider_id: str | None = None
    supported_attempts = 0
    unsupported_attempts = 0
    smart_apply_config_writes = 0

    for registration in registrations:
        instance_id = str(registration["instance_id"])
        endpoint = str(registration["control_endpoint"])
        try:
            response = send_request(
                endpoint,
                {
                    "op": SMART_APPLY_PROVIDER_OP,
                    "source_provider_id": source_provider_id,
                },
            )
        except Exception as exc:
            summary["failed_instances"] += 1
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": False,
                    "method": "smart_apply",
                    "error": str(exc),
                }
            )
            continue

        if unsupported_control_operation(response, SMART_APPLY_PROVIDER_OP):
            unsupported_attempts += 1
            unsupported_registrations.append(registration)
            summary["details"].append(
                {
                    "instance_id": instance_id,
                    "ok": False,
                    "method": "smart_apply_unsupported",
                    "response": response,
                }
            )
            continue

        supported_attempts += 1
        summary["smart_apply_instances"] += 1
        if isinstance(response.get("current_model_provider_id"), str):
            current_model_provider_id = str(response["current_model_provider_id"])
        instance_ok = add_refresh_response_counts(summary, response)
        summary["details"].append(
            {
                "instance_id": instance_id,
                "ok": instance_ok,
                "method": "smart_apply",
                "response": response,
            }
        )
        if smart_apply_wrote_config(response):
            smart_apply_config_writes += 1

    if supported_attempts == 0 and unsupported_attempts == len(registrations):
        legacy = apply_selected_provider_to_config(
            source_provider_id,
            codex_home=codex_home,
            registry_dir=registry_dir,
            pid_checker=pid_checker,
            ping_checker=ping_checker,
            send_request=send_request,
        )
        legacy["apply_strategy"] = "legacy_config_write"
        refresh_summary = legacy.get("refresh_summary")
        if isinstance(refresh_summary, dict):
            refresh_summary["fallback_instances"] = int(
                refresh_summary.get("total_instances", 0)
            )
            legacy["fallback_instances"] = refresh_summary["fallback_instances"]
            legacy["success_instances"] = int(refresh_summary.get("success_instances", 0))
            legacy["failed_instances"] = int(refresh_summary.get("failed_instances", 0))
            legacy["applied_threads"] = int(refresh_summary.get("applied_threads", 0))
            legacy["queued_threads"] = int(refresh_summary.get("queued_threads", 0))
            legacy["failed_threads"] = int(refresh_summary.get("failed_threads", 0))
        return legacy

    if unsupported_registrations and smart_apply_config_writes > 0:
        fallback_summary = refresh_registrations(
            unsupported_registrations,
            send_request,
            method="legacy_refresh_after_smart_apply",
        )
        fallback_summary["fallback_instances"] = len(unsupported_registrations)
        summary = merge_refresh_summaries(summary, fallback_summary)

    ok = smart_apply_config_writes > 0
    if not ok:
        return {
            "ok": False,
            "message": "app-server 智能 provider apply 未成功刷新任何实例",
            "source_provider_id": source_provider_id,
            "current_model_provider_id": current_model_provider_id,
            "config_path": str(config_toml_path(codex_home)),
            "config_changed": False,
            "apply_strategy": "app_server_smart_apply",
            "refresh_summary": summary,
            "success_instances": summary["success_instances"],
            "failed_instances": summary["failed_instances"],
            "smart_apply_instances": summary["smart_apply_instances"],
            "fallback_instances": summary["fallback_instances"],
            "applied_threads": summary["applied_threads"],
            "queued_threads": summary["queued_threads"],
            "failed_threads": summary["failed_threads"],
        }

    return {
        "ok": True,
        "message": None,
        "source_provider_id": source_provider_id,
        "current_model_provider_id": current_model_provider_id,
        "config_path": str(config_toml_path(codex_home)),
        "config_changed": True,
        "apply_strategy": "app_server_smart_apply",
        "refresh_summary": summary,
        "success_instances": summary["success_instances"],
        "failed_instances": summary["failed_instances"],
        "smart_apply_instances": summary["smart_apply_instances"],
        "fallback_instances": summary["fallback_instances"],
        "applied_threads": summary["applied_threads"],
        "queued_threads": summary["queued_threads"],
        "failed_threads": summary["failed_threads"],
    }


def format_result_message(success_instances: int, failed_instances: int) -> str:
    return (
        "刷新全部 app-server 完成\n\n"
        f"成功实例：{success_instances}\n"
        f"失败实例：{failed_instances}"
    )


def refresh_scope_label(scope: str | None) -> str:
    return REFRESH_SCOPE_LABELS.get(scope or REFRESH_SCOPE_ALL, REFRESH_SCOPE_LABELS[REFRESH_SCOPE_ALL])


def short_error_text(message: str, limit: int = 96) -> str:
    collapsed = " ".join(message.split())
    if len(collapsed) <= limit:
        return collapsed
    return f"{collapsed[: limit - 3]}..."


def failed_detail_lines_from_refresh_summary(summary: dict[str, Any]) -> list[str]:
    failed_detail_lines: list[str] = []
    details = summary.get("details")
    if not isinstance(details, list):
        return failed_detail_lines

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
            response_message = response.get("message")
            if isinstance(response_message, str) and response_message:
                failed_detail_lines.append(
                    f"- {instance_id}: {short_error_text(response_message, 120)}"
                )
                continue

            response_error = response.get("error")
            if isinstance(response_error, str) and response_error:
                failed_detail_lines.append(
                    f"- {instance_id}: {short_error_text(response_error, 120)}"
                )
                continue

            failed_threads_value = response.get("failed_threads")
            if isinstance(failed_threads_value, list) and failed_threads_value:
                failed_detail_lines.append(
                    f"- {instance_id}: failed_threads={len(failed_threads_value)}"
                )
                continue
        failed_detail_lines.append(f"- {instance_id}: refresh 返回失败")

    return failed_detail_lines




def format_refresh_summary(summary: dict[str, Any]) -> tuple[str, str, int]:
    total_instances = int(summary.get("total_instances", 0))
    success_instances = int(summary.get("success_instances", 0))
    failed_instances = int(summary.get("failed_instances", 0))
    applied_threads = int(summary.get("applied_threads", 0))
    queued_threads = int(summary.get("queued_threads", 0))
    failed_threads = int(summary.get("failed_threads", 0))
    scope_label = refresh_scope_label(summary.get("scope") if isinstance(summary.get("scope"), str) else None)
    title = "Codex App Server Refresh"
    icon_flag = MB_ICONINFORMATION if failed_instances == 0 else MB_ICONWARNING
    if total_instances == 0:
        message = f"{scope_label}\n\n未发现 live app-server 实例。\n\n实例总数: 0"
    else:
        message = (
            f"{scope_label} 完成\n\n"
            f"实例总数: {total_instances}\n"
            f"成功实例: {success_instances}\n"
            f"失败实例: {failed_instances}\n"
            f"Applied 线程: {applied_threads}\n"
            f"Queued 线程: {queued_threads}\n"
            f"Failed 线程: {failed_threads}"
        )
    return title, message, icon_flag


def format_apply_summary(summary: dict[str, Any]) -> tuple[str, str, int]:
    title = "Codex URL/token Apply"
    if not bool(summary.get("ok")):
        lines = [
            "无法应用新的 URL/token",
            "",
            f"原因: {summary.get('message') or '未知错误'}",
        ]
        refresh_summary = summary.get("refresh_summary")
        if isinstance(refresh_summary, dict):
            detail_lines = failed_detail_lines_from_refresh_summary(refresh_summary)
            if detail_lines:
                lines.extend(["", "失败明细:"])
                lines.extend(detail_lines[:5])
                remaining = len(detail_lines) - 5
                if remaining > 0:
                    lines.append(f"... 另有 {remaining} 个失败实例")
        message = "\n".join(lines)
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
            "已应用新的 URL/token，当前没有已打开的 Codex 实例"
            if config_changed
            else "当前 provider 已经使用这组 URL/token，未刷新任何 Codex 实例"
        )
        icon_flag = MB_ICONINFORMATION
    elif failed_instances == 0:
        headline = (
            "已应用新的 URL/token，并已刷新 Codex"
            if config_changed
            else "当前 provider 已经使用这组 URL/token，并已刷新 Codex"
        )
        icon_flag = MB_ICONINFORMATION
    else:
        headline = (
            "已应用新的 URL/token，但部分 Codex 实例刷新失败"
            if config_changed
            else "当前 provider 已经使用这组 URL/token，但部分 Codex 实例刷新失败"
        )
        icon_flag = MB_ICONWARNING

    lines = [
        headline,
        "",
        f"当前 provider: {current_model_provider_id}",
        f"配置文件: {config_path}",
    ]
    if source_provider_id != "unknown":
        lines.insert(2, f"来源配置: {source_provider_id}")
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
        failed_detail_lines = failed_detail_lines_from_refresh_summary(refresh_summary)
        if failed_detail_lines:
            lines.extend(["", "刷新失败明细:"])
            lines.extend(failed_detail_lines[:5])
            remaining = len(failed_detail_lines) - 5
            if remaining > 0:
                lines.append(f"... 另有 {remaining} 个失败实例")
    return title, "\n".join(lines), icon_flag


def mask_secret(value: str | None) -> str:
    normalized = normalize_string(value)
    if normalized is None:
        return "未配置"
    if len(normalized) <= 4:
        return "*" * len(normalized)
    return f"{'*' * 8}{normalized[-4:]}"


def provider_status_text(provider: dict[str, Any]) -> tuple[str, bool]:
    if bool(provider.get("requires_openai_auth")):
        return "不支持", False
    has_base_url = bool(provider.get("has_base_url"))
    has_token = bool(provider.get("has_experimental_bearer_token"))
    if has_base_url and has_token:
        return "可用于填入", True
    if not has_base_url and not has_token:
        return "缺少 base_url 和 token", False
    if not has_base_url:
        return "缺少 base_url", False
    return "缺少 token", False


def provider_display_rows(
    providers: list[dict[str, Any]],
    *,
    selected_provider_id: str | None,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for provider in providers:
        provider_id = str(provider.get("provider_id") or "")
        if not provider_id:
            continue
        display_name = str(provider.get("display_name") or provider_id)
        status, can_apply = provider_status_text(provider)
        rows.append(
            {
                "provider_id": provider_id,
                "display_name": display_name,
                "status": status,
                "selected": provider_id == selected_provider_id,
                "can_apply": can_apply,
            }
        )
    return rows


def dashboard_model_from_snapshot(
    snapshot: dict[str, Any],
    *,
    live_instance_count: int,
    last_result: str | None,
) -> dict[str, Any]:
    providers = snapshot.get("providers")
    if not isinstance(providers, list):
        providers = []
    selected_provider_id = snapshot.get("selected_provider_id")
    if not isinstance(selected_provider_id, str) or not selected_provider_id:
        selected_provider_id = None

    rows = provider_display_rows(
        providers,
        selected_provider_id=selected_provider_id,
    )
    selected_row = next((row for row in rows if row["selected"]), None)
    catalog_error = snapshot.get("catalog_error")
    if isinstance(catalog_error, str) and catalog_error:
        config_status = f"配置错误：{catalog_error}"
    else:
        config_status = "配置已加载"

    current_provider_id = snapshot.get("current_model_provider_id")
    if not isinstance(current_provider_id, str) or not current_provider_id:
        current_provider_id = "未配置"
    current_provider = next(
        (provider for provider in providers if provider.get("provider_id") == current_provider_id),
        None,
    )
    current_base_url = (
        normalize_string(current_provider.get("base_url"))
        if isinstance(current_provider, dict)
        else None
    )
    current_token = (
        normalize_string(current_provider.get("experimental_bearer_token"))
        if isinstance(current_provider, dict)
        else None
    )

    can_apply = current_provider_id != "未配置" and not bool(catalog_error)
    apply_disabled_reason = None
    if not can_apply:
        if isinstance(catalog_error, str) and catalog_error:
            apply_disabled_reason = catalog_error
        else:
            apply_disabled_reason = "当前 provider 未配置"

    config_path = snapshot.get("config_path")
    if not isinstance(config_path, str) or not config_path:
        config_path = str(config_toml_path())

    return {
        "target_provider_label": current_provider_id,
        "current_provider_label": current_provider_id,
        "current_base_url_label": current_base_url or "未配置",
        "current_token_label": mask_secret(current_token),
        "new_base_url_value": current_base_url or "",
        "new_token_value": current_token or "",
        "config_path_label": config_path,
        "config_status_label": config_status,
        "live_instances_label": str(live_instance_count),
        "provider_rows": rows,
        "selected_provider_id": selected_provider_id,
        "can_apply": can_apply,
        "apply_disabled_reason": apply_disabled_reason,
        "last_result": last_result or "尚未执行",
    }


class DashboardController:
    def __init__(
        self,
        state: TrayState,
        *,
        on_state_changed: Callable[[], None],
    ) -> None:
        self.state = state
        self.on_state_changed = on_state_changed
        self.root: Any | None = None
        self.provider_tree: Any | None = None
        self.runtime_inputs_initialized = False
        self.target_var: Any | None = None
        self.current_base_url_var: Any | None = None
        self.current_token_var: Any | None = None
        self.new_base_url_var: Any | None = None
        self.new_token_var: Any | None = None
        self.config_path_var: Any | None = None
        self.config_status_var: Any | None = None
        self.live_instances_var: Any | None = None
        self.apply_button: Any | None = None
        self.apply_hint_var: Any | None = None
        self.result_text: Any | None = None
        self.last_result = "尚未执行"
        self._ui_queue: queue.Queue[Callable[[], None]] = queue.Queue()
        self._thread: threading.Thread | None = None

    def show(self) -> None:
        self._ensure_thread()
        self._post(self._show_on_ui_thread)

    def close_to_tray(self) -> None:
        if self.root is not None:
            self.root.withdraw()

    def destroy(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            return
        self._post(self._destroy_on_ui_thread)

    def reload_catalog_from_tray(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            return
        self._post(self.reload_catalog)

    def _ensure_thread(self) -> None:
        if self._thread is not None and self._thread.is_alive():
            return
        self._thread = threading.Thread(target=self._run_ui, daemon=True)
        self._thread.start()

    def _run_ui(self) -> None:
        self._create_window()
        self._show_on_ui_thread()
        self.root.mainloop()

    def _post(self, callback: Callable[[], None]) -> None:
        self._ui_queue.put(callback)

    def _show_on_ui_thread(self) -> None:
        if self.root is None:
            return
        self.root.deiconify()
        self.refresh_view()
        self.root.lift()
        self.root.focus_force()

    def _destroy_on_ui_thread(self) -> None:
        if self.root is not None:
            self.root.destroy()
            self.root = None

    def _create_window(self) -> None:
        import tkinter as tk
        from tkinter import ttk

        root = tk.Tk()
        root.title("Codex Provider Refresh")
        root.geometry("760x520")
        root.minsize(680, 440)
        root.protocol("WM_DELETE_WINDOW", self.close_to_tray)
        self.root = root

        self.target_var = tk.StringVar()
        self.current_base_url_var = tk.StringVar()
        self.current_token_var = tk.StringVar()
        self.new_base_url_var = tk.StringVar()
        self.new_token_var = tk.StringVar()
        self.config_path_var = tk.StringVar()
        self.config_status_var = tk.StringVar()
        self.live_instances_var = tk.StringVar()
        self.apply_hint_var = tk.StringVar()

        frame = ttk.Frame(root, padding=12)
        frame.grid(row=0, column=0, sticky="nsew")
        root.columnconfigure(0, weight=1)
        root.rowconfigure(0, weight=1)
        frame.columnconfigure(0, weight=2)
        frame.columnconfigure(1, weight=1)
        frame.rowconfigure(1, weight=1)
        frame.rowconfigure(4, weight=1)

        header = ttk.LabelFrame(frame, text="当前状态")
        header.grid(row=0, column=0, columnspan=2, sticky="ew", pady=(0, 10))
        header.columnconfigure(1, weight=1)
        ttk.Label(header, text="当前 Provider").grid(row=0, column=0, sticky="w", padx=8, pady=(6, 2))
        ttk.Label(header, textvariable=self.target_var).grid(row=0, column=1, sticky="w", padx=8, pady=(6, 2))
        ttk.Label(header, text="已打开的 Codex 实例").grid(row=0, column=2, sticky="w", padx=8, pady=(6, 2))
        ttk.Label(header, textvariable=self.live_instances_var).grid(row=0, column=3, sticky="w", padx=8, pady=(6, 2))
        ttk.Label(header, text="当前 Base URL").grid(row=1, column=0, sticky="w", padx=8, pady=2)
        ttk.Label(header, textvariable=self.current_base_url_var).grid(row=1, column=1, columnspan=3, sticky="ew", padx=8, pady=2)
        ttk.Label(header, text="当前 Token").grid(row=2, column=0, sticky="w", padx=8, pady=2)
        ttk.Label(header, textvariable=self.current_token_var).grid(row=2, column=1, sticky="w", padx=8, pady=2)
        ttk.Label(header, text="配置文件").grid(row=3, column=0, sticky="w", padx=8, pady=(2, 6))
        ttk.Label(header, textvariable=self.config_path_var).grid(row=3, column=1, columnspan=3, sticky="ew", padx=8, pady=(2, 6))

        providers = ttk.LabelFrame(frame, text="从已有 provider 填入")
        providers.grid(row=1, column=0, sticky="nsew", padx=(0, 8))
        providers.columnconfigure(0, weight=1)
        providers.rowconfigure(0, weight=1)
        self.provider_tree = ttk.Treeview(
            providers,
            columns=("name", "status"),
            show="headings",
            selectmode="browse",
            height=9,
        )
        self.provider_tree.heading("name", text="Provider")
        self.provider_tree.heading("status", text="状态")
        self.provider_tree.column("name", width=260, anchor="w")
        self.provider_tree.column("status", width=160, anchor="w")
        self.provider_tree.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)
        self.provider_tree.bind("<<TreeviewSelect>>", self._on_provider_selected)
        ttk.Button(
            providers,
            text="填入 URL/token",
            command=self.fill_from_selected_provider,
        ).grid(row=1, column=0, sticky="ew", padx=8, pady=(0, 8))

        target = ttk.LabelFrame(frame, text="准备应用的新 URL/token")
        target.grid(row=1, column=1, sticky="nsew")
        target.columnconfigure(0, weight=1)
        ttk.Label(target, text="新的 Base URL").grid(row=0, column=0, sticky="w", padx=8, pady=(8, 2))
        ttk.Entry(target, textvariable=self.new_base_url_var).grid(row=1, column=0, sticky="ew", padx=8, pady=2)
        ttk.Label(target, text="新的 Token").grid(row=2, column=0, sticky="w", padx=8, pady=(8, 2))
        ttk.Entry(target, textvariable=self.new_token_var, show="*").grid(row=3, column=0, sticky="ew", padx=8, pady=2)
        ttk.Label(
            target,
            text="点击应用后只覆盖当前 provider 的 base_url 和 token，不切换 model_provider。",
            wraplength=240,
        ).grid(row=4, column=0, sticky="w", padx=8, pady=(10, 2))
        ttk.Label(target, textvariable=self.config_status_var).grid(row=5, column=0, sticky="w", padx=8, pady=(12, 2))
        ttk.Button(target, text="打开配置文件", command=self.open_config_file).grid(row=6, column=0, sticky="ew", padx=8, pady=(12, 2))
        ttk.Button(target, text="打开 .codex 文件夹", command=self.open_codex_home).grid(row=7, column=0, sticky="ew", padx=8, pady=2)
        ttk.Button(target, text="重新加载配置", command=self.reload_catalog).grid(row=8, column=0, sticky="ew", padx=8, pady=2)

        actions = ttk.LabelFrame(frame, text="操作")
        actions.grid(row=2, column=0, columnspan=2, sticky="ew", pady=(10, 10))
        for index in range(4):
            actions.columnconfigure(index, weight=1)
        self.apply_button = ttk.Button(
            actions,
            text="应用到当前 provider 并刷新 Codex",
            command=self.apply_entered_runtime_values,
        )
        self.apply_button.grid(row=0, column=0, columnspan=4, sticky="ew", padx=8, pady=(8, 4))
        ttk.Label(actions, textvariable=self.apply_hint_var).grid(row=1, column=0, columnspan=4, sticky="w", padx=8, pady=(0, 6))
        ttk.Button(actions, text="仅刷新全部", command=self.refresh_all).grid(row=2, column=0, sticky="ew", padx=8, pady=(0, 8))
        ttk.Button(actions, text="仅刷新命令行 Codex", command=self.refresh_console).grid(row=2, column=1, sticky="ew", padx=8, pady=(0, 8))
        ttk.Button(actions, text="仅刷新 Windows App", command=self.refresh_app_server).grid(row=2, column=2, sticky="ew", padx=8, pady=(0, 8))
        ttk.Button(actions, text="隐藏到托盘", command=self.close_to_tray).grid(row=2, column=3, sticky="ew", padx=8, pady=(0, 8))

        result = ttk.LabelFrame(frame, text="最近结果")
        result.grid(row=4, column=0, columnspan=2, sticky="nsew")
        result.columnconfigure(0, weight=1)
        result.rowconfigure(0, weight=1)
        self.result_text = tk.Text(result, height=6, wrap="word", state="disabled")
        self.result_text.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)

        self._drain_queue()

    def refresh_view(self) -> None:
        if self.root is None:
            return
        model = dashboard_model_from_snapshot(
            self.state.snapshot(),
            live_instance_count=len(registrations_for_refresh()),
            last_result=self.last_result,
        )
        self.target_var.set(model["target_provider_label"])
        self.current_base_url_var.set(model["current_base_url_label"])
        self.current_token_var.set(model["current_token_label"])
        if not self.runtime_inputs_initialized:
            self.new_base_url_var.set(model["new_base_url_value"])
            self.new_token_var.set(model["new_token_value"])
            self.runtime_inputs_initialized = True
        self.config_path_var.set(model["config_path_label"])
        self.config_status_var.set(model["config_status_label"])
        self.live_instances_var.set(model["live_instances_label"])
        self._render_provider_rows(model)
        if model["can_apply"]:
            self.apply_button.state(["!disabled"])
            self.apply_hint_var.set(
                f"将覆盖当前 provider「{model['current_provider_label']}」的 base_url 和 token，并刷新已打开的 Codex。"
            )
        else:
            self.apply_button.state(["disabled"])
            reason = model.get("apply_disabled_reason") or "当前不可应用"
            self.apply_hint_var.set(str(reason))
        self._set_result_text(model["last_result"])

    def _render_provider_rows(self, model: dict[str, Any]) -> None:
        self.provider_tree.delete(*self.provider_tree.get_children())
        selected_id = model.get("selected_provider_id")
        for row in model["provider_rows"]:
            item_id = row["provider_id"]
            self.provider_tree.insert(
                "",
                "end",
                iid=item_id,
                values=(row["display_name"], row["status"]),
            )
            if item_id == selected_id:
                self.provider_tree.selection_set(item_id)

    def _set_result_text(self, message: str) -> None:
        self.result_text.configure(state="normal")
        self.result_text.delete("1.0", "end")
        self.result_text.insert("1.0", message)
        self.result_text.configure(state="disabled")

    def _on_provider_selected(self, _event: Any) -> None:
        selection = self.provider_tree.selection()
        if not selection:
            return
        provider_id = str(selection[0])
        if self.state.snapshot().get("selected_provider_id") == provider_id:
            return
        self.state.set_selected_provider(provider_id)
        self.on_state_changed()
        self.refresh_view()

    def fill_from_selected_provider(self) -> None:
        selected_provider_id = self.state.snapshot().get("selected_provider_id")
        if not isinstance(selected_provider_id, str) or not selected_provider_id:
            self.last_result = "请选择一个 provider 作为 URL/token 来源。"
            self.refresh_view()
            return
        provider = next(
            (
                row
                for row in self.state.snapshot().get("providers", [])
                if row.get("provider_id") == selected_provider_id
            ),
            None,
        )
        if not isinstance(provider, dict):
            self.last_result = "所选 provider 不存在。"
            self.refresh_view()
            return
        base_url = normalize_string(provider.get("base_url"))
        token = normalize_string(provider.get("experimental_bearer_token"))
        if base_url is None or token is None:
            self.last_result = f"所选 provider {provider_status_text(provider)[0]}，不能填入。"
            self.refresh_view()
            return
        self.new_base_url_var.set(base_url)
        self.new_token_var.set(token)
        self.last_result = f"已从 {provider.get('display_name') or selected_provider_id} 填入 URL/token，尚未应用。"
        self.refresh_view()

    def _run_worker(self, work: Callable[[], tuple[str, str, int]]) -> None:
        def worker() -> None:
            try:
                title, message, icon_flag = work()
            except Exception as exc:
                title = "Codex Provider Refresh"
                message = f"操作失败：{exc}"
                icon_flag = MB_ICONERROR

            def update() -> None:
                self.last_result = f"{title}\n\n{message}"
                self.refresh_view()

            self._ui_queue.put(update)

        threading.Thread(target=worker, daemon=True).start()

    def _drain_queue(self) -> None:
        while True:
            try:
                callback = self._ui_queue.get_nowait()
            except queue.Empty:
                break
            callback()
        if self.root is not None:
            self.root.after(100, self._drain_queue)

    def reload_catalog(self) -> None:
        self.state.set_catalog(load_user_provider_catalog())
        self.on_state_changed()
        self.runtime_inputs_initialized = False
        self.refresh_view()

    def refresh_all(self) -> None:
        self._run_worker(lambda: self._refresh_summary(refresh_all_instances))

    def refresh_console(self) -> None:
        self._run_worker(lambda: self._refresh_summary(refresh_console_instances))

    def refresh_app_server(self) -> None:
        self._run_worker(lambda: self._refresh_summary(refresh_app_server_instances))

    def _refresh_summary(
        self,
        refresh_fn: Callable[[], dict[str, Any]],
    ) -> tuple[str, str, int]:
        summary = refresh_fn()
        self.state.set_catalog(load_user_provider_catalog())
        self.on_state_changed()
        return format_refresh_summary(summary)

    def apply_entered_runtime_values(self) -> None:
        base_url = self.new_base_url_var.get()
        token = self.new_token_var.get()
        def work() -> tuple[str, str, int]:
            summary = apply_runtime_values_to_current_provider(base_url, token)
            self.state.set_catalog(load_user_provider_catalog())
            self.on_state_changed()
            return format_apply_summary(summary)

        self._run_worker(work)

    def open_config_file(self) -> None:
        open_path(config_toml_path())

    def open_codex_home(self) -> None:
        open_path(default_codex_home())


def open_path(path: Path) -> None:
    if os.name == "nt":
        try:
            os.startfile(path)  # type: ignore[attr-defined]
        except OSError as exc:
            show_message(
                "Codex Provider Refresh",
                f"无法打开路径：\n{path}\n\n{exc}",
                MB_ICONERROR,
            )


def create_tray_icon():
    import pystray
    from PIL import Image
    from PIL import ImageDraw

    state = TrayState()
    state.set_catalog(load_user_provider_catalog())
    dashboard: DashboardController

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

    def handle_open_dashboard(_icon: pystray.Icon, _item: Any) -> None:
        dashboard.show()

    def handle_refresh(icon: pystray.Icon, _item: Any) -> None:
        def worker() -> None:
            summary = refresh_all_instances()
            reload_catalog()
            rebuild_menu(icon)
            title, message, icon_flag = format_refresh_summary(summary)
            show_message(title, message, icon_flag)

        threading.Thread(target=worker, daemon=True).start()

    def noop(_icon: pystray.Icon, _item: Any) -> None:
        return

    def build_menu(icon: pystray.Icon) -> pystray.Menu:
        snapshot = state.snapshot()
        current_model_provider_id = snapshot.get("current_model_provider_id") or "未配置"
        catalog_error = snapshot.get("catalog_error")
        catalog_status_label = (
            f"config 错误: {short_error_text(catalog_error, 56)}"
            if isinstance(catalog_error, str) and catalog_error
            else f"当前 provider: {current_model_provider_id}"
        )
        return pystray.Menu(
            pystray.MenuItem(
                "打开主窗口",
                handle_open_dashboard,
                default=True,
            ),
            pystray.MenuItem(catalog_status_label, noop, enabled=False),
            pystray.MenuItem("快速刷新全部", handle_refresh),
            pystray.MenuItem(
                "重新加载配置",
                lambda tray_icon, _item: (
                    reload_catalog(),
                    dashboard.reload_catalog_from_tray(),
                    rebuild_menu(tray_icon),
                ),
            ),
            pystray.MenuItem(
                "退出",
                lambda tray_icon, _item: (
                    dashboard.destroy(),
                    tray_icon.stop(),
                ),
            ),
        )

    dashboard = DashboardController(
        state,
        on_state_changed=lambda: None,
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
