"""Black-box local3 transport contract tests; never builds the supplied CLI.

Truth source: local3 checklist sections 3.1 and 3.4: three failed WS
attempts, fixed ten-second waits, then a successful WS retry; tools survive.
The server implements RFC 6455 framing and the Responses streaming lifecycle.
"""

import base64
import hashlib
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class Fixture(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, scenario):
        super().__init__(("127.0.0.1", 0), Handler)
        self.scenario = scenario
        self.requests = []
        self.failures = []
        self.tool_sent = False
        self.handshake_ready = False
        self.lock = threading.Lock()

    def record(self, transport, payload):
        compact = "COMPACT_FIXTURE" in json.dumps(payload)
        with self.lock:
            entry = {"transport": transport, "compact": compact,
                     "payload": payload, "at": time.monotonic()}
            self.requests.append(entry)
        return compact

    def events(self, payload, compact=False):
        output = []
        tokens = 10
        if compact:
            text = "The marker command already ran once. Finish without calling tools."
        elif self.scenario == "compact" and not self.tool_sent:
            self.tool_sent = True
            output = [{"type": "function_call", "name": "exec_command",
                       "call_id": "call_marker_once",
                       "arguments": json.dumps({"cmd": "echo WS_TOOL_ONCE",
                                                "max_output_tokens": 30})}]
            tokens = 21000
            text = None
        else:
            text = "WS_RECOVERY_OK"
        if text is not None:
            output = [{"id": "msg_fixture", "type": "message",
                       "role": "assistant",
                       "content": [{"type": "output_text", "text": text}]}]
        rid = f"resp_fixture_{len(self.requests)}"
        events = [{"type": "response.created", "response": {"id": rid}}]
        events.extend({"type": "response.output_item.done", "output_index": i, "item": item}
                      for i, item in enumerate(output))
        events.append({"type": "response.completed", "response": {
            "id": rid, "status": "completed", "output": output,
            "usage": {"input_tokens": tokens, "output_tokens": 10,
                      "total_tokens": tokens + 10}}})
        return events


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_args):
        pass

    def frame(self, opcode, data):
        n = len(data)
        header = bytes([0x80 | opcode])
        if n < 126:
            header += bytes([n])
        elif n < 65536:
            header += bytes([126]) + struct.pack("!H", n)
        else:
            header += bytes([127]) + struct.pack("!Q", n)
        self.wfile.write(header + data)
        self.wfile.flush()

    def read_exact(self, n):
        result = self.rfile.read(n)
        if len(result) != n:
            raise EOFError
        return result

    def read_message(self):
        parts = []
        while True:
            a, b = self.read_exact(2)
            opcode, n = a & 15, b & 127
            if n == 126:
                n = struct.unpack("!H", self.read_exact(2))[0]
            elif n == 127:
                n = struct.unpack("!Q", self.read_exact(8))[0]
            if n > 32 * 1024 * 1024:
                raise ValueError("fixture frame too large")
            mask = self.read_exact(4) if b & 128 else None
            data = self.read_exact(n)
            if mask:
                data = bytes(value ^ mask[i % 4] for i, value in enumerate(data))
            if opcode == 8:
                raise EOFError
            if opcode == 9:
                self.frame(10, data)
                continue
            if opcode == 10:
                continue
            parts.append(data)
            if a & 128:
                return json.loads(b"".join(parts))

    def do_GET(self):
        if self.headers.get("Upgrade", "").lower() != "websocket":
            self.send_error(404)
            return
        if (self.server.scenario == "handshake" and self.server.handshake_ready
                and len(self.server.failures) < 3):
            self.server.record("handshake_eof", {})
            self.server.failures.append(time.monotonic())
            self.connection.shutdown(socket.SHUT_RDWR)
            self.close_connection = True
            return
        accept = base64.b64encode(hashlib.sha1(
            (self.headers["Sec-WebSocket-Key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()
        ).digest()).decode()
        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        self.wfile.flush()
        self.close_connection = True
        self.connection.settimeout(90)
        try:
            while True:
                payload = self.read_message()
                if payload.get("generate") is False:
                    self.frame(1, json.dumps({"type": "response.completed",
                                             "response": {"id": "warmup", "output": []}}).encode())
                    if self.server.scenario == "handshake":
                        self.server.handshake_ready = True
                        self.frame(8, struct.pack("!H", 1000))
                        return
                    continue
                compact = self.server.record("ws", payload)
                fail = self.server.scenario == "close" or (self.server.scenario == "compact" and compact)
                if fail and len(self.server.failures) < 3:
                    self.server.failures.append(time.monotonic())
                    self.frame(8, struct.pack("!H", 1011) + b"fixture-private-reason")
                    return
                for event in self.server.events(payload, compact):
                    self.frame(1, json.dumps(event).encode())
        except (EOFError, ConnectionError, TimeoutError, OSError):
            return

    def do_POST(self):
        payload = json.loads(self.read_exact(int(self.headers["Content-Length"])))
        compact = self.server.record("http", payload)
        events = self.server.events(payload, compact)
        body = "".join(f"data: {json.dumps(event)}\n\n" for event in events).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        self.wfile.flush()


def command(binary, fixture):
    config = {
        "model_provider": "fixture",
        "model_providers.fixture.name": "WebSocket recovery fixture",
        "model_providers.fixture.base_url": f"http://127.0.0.1:{fixture.server_port}/v1",
        "model_providers.fixture.wire_api": "responses",
        "model_providers.fixture.supports_websockets": True,
        "model_providers.fixture.requires_openai_auth": False,
        "model_providers.fixture.request_max_retries": 0,
        "model_providers.fixture.stream_max_retries": 100000,
        "features.enable_request_compression": False,
        "features.remote_models": False,
        "compact_prompt": "COMPACT_FIXTURE",
        "model_auto_compact_token_limit": 20000 if fixture.scenario == "compact" else 200000,
    }
    # This isolated process talks only to our loopback fixture, whose sole tool
    # call prints a literal marker. Do not depend on machine sandbox setup or
    # permit product-policy rejection to masquerade as a completed tool.
    args = [binary, "exec", "--ignore-user-config", "--ephemeral",
            "--skip-git-repo-check", "--sandbox", "danger-full-access", "--json", "-m", "gpt-5.4"]
    for key, value in config.items():
        args.extend(["-c", f"{key}={json.dumps(value)}"])
    args.append("Run the read-only marker command once if requested, then reply with WS_RECOVERY_OK.")
    return args


def verify(binary, scenario, output_dir=None):
    server = Fixture(scenario)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    try:
        with tempfile.TemporaryDirectory(prefix="codex-ws-fixture-") as temp:
            env = os.environ.copy()
            env["CODEX_HOME"] = temp
            env["CODEX_INTERNAL_RETRY_MODE"] = "unbounded"
            try:
                result = subprocess.run(command(binary, server), cwd=temp, env=env,
                                        capture_output=True, text=True, encoding="utf-8",
                                        errors="replace", timeout=120,
                                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            except subprocess.TimeoutExpired as exc:
                print(json.dumps({"scenario": scenario, "requests": [
                    {k: v for k, v in item.items() if k != "payload"} for item in server.requests]}))
                raise RuntimeError(f"{scenario}: CLI failed to recover within 120 seconds") from exc
        if output_dir:
            output_dir.mkdir(parents=True, exist_ok=True)
            (output_dir / f"{scenario}.stdout.txt").write_text(result.stdout, encoding="utf-8")
            (output_dir / f"{scenario}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        if result.returncode or "WS_RECOVERY_OK" not in result.stdout:
            print(result.stdout[-5000:]); print(result.stderr[-3000:])
            raise AssertionError(f"{scenario}: expected completed CLI response")
        http = [r for r in server.requests if r["transport"] == "http"]
        assert not http, "WS recovery must never switch to HTTP"
        waits = []
        if scenario != "healthy":
            assert len(server.failures) == 3, "three failed attempts must recover through WS"
            for failed_at in server.failures:
                resumed_at = min(r["at"] for r in server.requests if r["at"] > failed_at)
                wait = resumed_at - failed_at
                waits.append(round(wait, 3))
                assert 9.5 <= wait <= 20, ("expected ten-second failure-to-retry wait", wait)
            assert "fixture-private-reason" not in result.stdout + result.stderr
        if scenario == "handshake":
            attempts = [r for r in server.requests if r["transport"] == "handshake_eof"]
            assert len(attempts) == 3, "exactly three inference handshakes must fail"
        if scenario in ("close", "compact"):
            # Checklist 3.5: relay recovery rotates the cache key only after the
            # failed sequence and replays the complete input without a stale ID.
            attempts = [r["payload"] for r in server.requests
                        if scenario == "close" or r["compact"]]
            failed, recovered = attempts[:-1], attempts[-1]
            assert len(failed) == len(server.failures)
            original_key = failed[0]["prompt_cache_key"]
            assert all(r["prompt_cache_key"] == original_key for r in failed)
            assert recovered["prompt_cache_key"] != original_key
            assert not recovered.get("previous_response_id")
        if scenario == "compact":
            assert any(r["compact"] for r in server.requests), "auto compact must complete over WS"
            assert any(not r["compact"] for r in server.requests), "sampling must resume after compaction"
            outputs = [item for r in server.requests if r["compact"]
                       for item in r["payload"].get("input", [])
                       if item.get("type") == "function_call_output"]
            assert outputs, "completed tool result must reach compaction"
            assert all(item.get("call_id") == "call_marker_once" for item in outputs)
            events = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
            commands = [e for e in events if e.get("type") == "item.completed"
                        and e.get("item", {}).get("type") == "command_execution"]
            assert len(commands) == 1, "completed tool must not execute again during recovery"
            assert commands[0]["item"]["exit_code"] == 0, "marker command must actually succeed"
            assert "WS_TOOL_ONCE" in commands[0]["item"]["aggregated_output"]
        report = {"scenario": scenario, "passed": True,
                  "ws_failures": len(server.failures), "http_requests": len(http),
                  "failure_to_retry_seconds": waits}
        if output_dir:
            (output_dir / f"{scenario}.requests.json").write_text(
                json.dumps(server.requests, indent=2), encoding="utf-8")
        print(json.dumps(report), flush=True)
        return report
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)


if __name__ == "__main__":
    exe = str(Path(sys.argv[1]).resolve(strict=True))
    output = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    results = [verify(exe, scenario, output) for scenario in ("healthy", "close", "handshake", "compact")]
    if output:
        (output / "protocol-results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
