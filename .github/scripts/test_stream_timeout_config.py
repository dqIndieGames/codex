"""Black-box configuration contracts from checklist #3; never builds the CLI."""

from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time


class Fixture(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, scenario):
        super().__init__(("127.0.0.1", 0), Handler)
        self.scenario = scenario
        self.requests = []
        self.stopped = threading.Event()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def event(self, data):
        self.wfile.write(f"data: {json.dumps(data)}\n\n".encode())
        self.wfile.flush()

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.server.requests.append(time.monotonic())
        attempt = len(self.server.requests)
        scenario = self.server.scenario
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        try:
            self.event(
                {"type": "response.created", "response": {"id": "timeout_fixture"}}
            )
            if scenario in ("first_idle", "post_idle") and attempt == 1:
                if scenario == "post_idle":
                    self.event(
                        {"type": "response.output_text.delta", "delta": "waiting"}
                    )
                self.server.stopped.wait(330)
                return
            if scenario == "above_old_cap":
                if self.server.stopped.wait(400):
                    return
            if scenario == "long_output":
                for _ in range(4):
                    if self.server.stopped.wait(90):
                        return
                    self.event(
                        {"type": "response.output_text.delta", "delta": "progress "}
                    )
            message = {
                "id": "msg_timeout",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "STREAM_CONFIG_OK"}],
            }
            self.event(
                {
                    "type": "response.output_item.done",
                    "output_index": 0,
                    "item": message,
                }
            )
            self.event(
                {
                    "type": "response.completed",
                    "response": {
                        "id": "timeout_fixture",
                        "status": "completed",
                        "output": [message],
                    },
                }
            )
        except (BrokenPipeError, ConnectionResetError):
            return


def verify(binary, scenario, output=None):
    server = Fixture(scenario)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    requested_ms = 600000 if scenario == "above_old_cap" else 300000
    config = {
        "model_provider": "fixture",
        "model_providers.fixture.name": "Stream configuration fixture",
        "model_providers.fixture.base_url": f"http://127.0.0.1:{server.server_port}/v1",
        "model_providers.fixture.wire_api": "responses",
        "model_providers.fixture.supports_websockets": False,
        "model_providers.fixture.requires_openai_auth": False,
        "model_providers.fixture.request_max_retries": 1,
        "model_providers.fixture.stream_max_retries": 1,
        "model_providers.fixture.stream_idle_timeout_ms": requested_ms,
        "features.remote_models": False,
        "features.enable_request_compression": False,
    }
    args = [
        binary,
        "exec",
        "--ignore-user-config",
        "--ephemeral",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--json",
        "-m",
        "gpt-5.4",
    ]
    for key, value in config.items():
        args.extend(["-c", f"{key}={json.dumps(value)}"])
    args.append("Reply with STREAM_CONFIG_OK. Do not call tools.")
    started = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix="local3-stream-config-") as home:
            env = os.environ.copy()
            env["CODEX_HOME"] = home
            env["CODEX_INTERNAL_RETRY_MODE"] = "bounded"
            for key in ("OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL"):
                env.pop(key, None)
            result = subprocess.run(
                args,
                cwd=home,
                env=env,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=440,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        assert result.returncode == 0, result.stderr[-3000:]
        assert "STREAM_CONFIG_OK" in result.stdout, result.stdout[-3000:]
        if scenario in ("first_idle", "post_idle"):
            assert len(server.requests) == 2, (
                "one timed-out stream must recover with one retry"
            )
            interval = server.requests[1] - server.requests[0]
            expected = requested_ms / 1000 + 5
            assert expected - 2 <= interval <= expected + 15, (
                scenario,
                interval,
                expected,
            )
            retry_events = [
                json.loads(line) for line in result.stdout.splitlines() if line.strip()
            ]
            retry_messages = [
                event.get("message", "")
                for event in retry_events
                if event.get("type") == "error"
            ]
            assert any(
                str(requested_ms // 1000) in message for message in retry_messages
            ), retry_messages
        else:
            assert len(server.requests) == 1, (
                "configured quiet/progress periods must not be cut short"
            )
        report = {
            "scenario": scenario,
            "passed": True,
            "configured_ms": requested_ms,
            "requests": len(server.requests),
            "elapsed_s": round(time.monotonic() - started, 2),
        }
        if output:
            output.mkdir(parents=True, exist_ok=True)
            (output / f"{scenario}.stdout.txt").write_text(
                result.stdout, encoding="utf-8"
            )
            (output / f"{scenario}.stderr.txt").write_text(
                result.stderr, encoding="utf-8"
            )
        print(json.dumps(report), flush=True)
        return report
    finally:
        server.stopped.set()
        server.shutdown()
        server.server_close()
        worker.join(5)


if __name__ == "__main__":
    exe = str(Path(sys.argv[1]).resolve(strict=True))
    output = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(
            pool.map(
                lambda scenario: verify(exe, scenario, output),
                ("first_idle", "post_idle", "above_old_cap", "long_output"),
            )
        )
    if output:
        (output / "stream-results.json").write_text(
            json.dumps(results, indent=2), encoding="utf-8"
        )
