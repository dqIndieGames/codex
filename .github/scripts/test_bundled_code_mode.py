"""Exercise official code mode through a downloaded single-file local3 CLI."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading

from test_websocket_recovery import Fixture, command


class CodeModeFixture(Fixture):
    def __init__(self):
        super().__init__("bundled-code-mode")
        self.tool_output = None

    def events(self, payload, compact=False):
        outputs = [item for item in payload.get("input", [])
                   if item.get("type") == "custom_tool_call_output"
                   and item.get("call_id") == "call_bundled_host"]
        if outputs:
            self.tool_output = json.dumps(outputs[-1], ensure_ascii=False)
            return super().events(payload, compact)
        item = {"type": "custom_tool_call", "call_id": "call_bundled_host",
                "name": "exec",
                "input": 'text(6 * 7); text(await tools.exec_command({cmd:"echo LOCAL3_NESTED_TOOL_OK",max_output_tokens:100}));'}
        return [
            {"type": "response.created", "response": {"id": "resp_code_mode"}},
            {"type": "response.output_item.done", "output_index": 0, "item": item},
            {"type": "response.completed", "response": {"id": "resp_code_mode",
                "status": "completed", "output": [item]}}]


def verify(binary, output=None):
    server = CodeModeFixture()
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    try:
        with tempfile.TemporaryDirectory(prefix="local3-code-mode-") as temp:
            env = os.environ.copy()
            env["CODEX_HOME"] = temp
            for key in ("OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL"):
                env.pop(key, None)
            args = command(str(binary), server)
            args[args.index("-m") + 1] = "gpt-6.1-sol"
            args[-1:-1] = ["-c", "model_providers.fixture.supports_websockets=false"]
            result = subprocess.run(args, cwd=temp, env=env, capture_output=True,
                text=True, encoding="utf-8", errors="replace", timeout=120,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        if output:
            output.mkdir(parents=True, exist_ok=True)
            (output / "stdout.txt").write_text(result.stdout, encoding="utf-8")
            (output / "stderr.txt").write_text(result.stderr, encoding="utf-8")
            (output / "tool-output.json").write_text(server.tool_output or "null", encoding="utf-8")
        assert result.returncode == 0, result.stderr[-3000:]
        assert server.tool_output, "No real code-mode output returned to the model"
        assert "42" in server.tool_output, server.tool_output
        assert "LOCAL3_NESTED_TOOL_OK" in server.tool_output, server.tool_output
        assert "Code Mode is unavailable" not in result.stdout + result.stderr
        assert "WS_RECOVERY_OK" in result.stdout
        report = {"scenario": "single_exe_code_mode_and_nested_command", "passed": True}
        if output:
            (output / "result.json").write_text(json.dumps(report), encoding="utf-8")
        print(json.dumps(report), flush=True)
    finally:
        server.shutdown()
        server.server_close()
        worker.join(5)


if __name__ == "__main__":
    verify(Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]) if len(sys.argv) > 2 else None)
