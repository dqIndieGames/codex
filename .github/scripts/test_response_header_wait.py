"""Checklist #3: delayed headers preserve the original request past 60 seconds."""

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class DelayedResponses(BaseHTTPRequestHandler):
    requests = 0
    request_times = []

    def log_message(self, *_args):
        pass

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        type(self).requests += 1
        type(self).request_times.append(time.monotonic())
        # Product requirement: delayed headers must not trigger a client retry.
        time.sleep(90)
        message = {
            "id": "msg_header_wait",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "HEADER_WAIT_OK"}],
        }
        events = [
            {"type": "response.created", "response": {"id": "resp_header_wait"}},
            {"type": "response.output_item.done", "output_index": 0, "item": message},
            {
                "type": "response.completed",
                "response": {
                    "id": "resp_header_wait",
                    "status": "completed",
                    "output": [message],
                },
            },
        ]
        body = "".join(f"data: {json.dumps(event)}\n\n" for event in events).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    server = ThreadingHTTPServer(("127.0.0.1", 0), DelayedResponses)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    config = {
        "model_provider": '"header_fixture"',
        "model_providers.header_fixture.name": '"Header fixture"',
        "model_providers.header_fixture.base_url": f'"http://127.0.0.1:{server.server_port}/v1"',
        "model_providers.header_fixture.wire_api": '"responses"',
        "model_providers.header_fixture.request_max_retries": "1",
        "model_providers.header_fixture.stream_max_retries": "0",
        "model_providers.header_fixture.requires_openai_auth": "false",
        "model_providers.header_fixture.stream_idle_timeout_ms": "300000",
        "model_providers.header_fixture.supports_websockets": "false",
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
        args.extend(["-c", f"{key}={value}"])
    args.append("Reply with HEADER_WAIT_OK. Do not call tools.")
    try:
        with tempfile.TemporaryDirectory(prefix="codex-header-fixture-") as home:
            env = os.environ.copy()
            env["CODEX_HOME"] = home
            result = subprocess.run(args, cwd=home, env=env, capture_output=True,
                                    text=True, encoding="utf-8", errors="replace", timeout=130,
                                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        print(result.stdout)
        print(result.stderr, file=sys.stderr)
        if result.returncode != 0 or "HEADER_WAIT_OK" not in result.stdout:
            raise RuntimeError("CLI did not complete after delayed response headers")
        if DelayedResponses.requests != 1:
            raise RuntimeError("Delayed headers must complete the original request without retry")
        print("Delayed headers: PASS (90 seconds, one request)")
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)


if __name__ == "__main__":
    main()
