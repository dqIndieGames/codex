"""清单 2.1、5.4：真实 exe 的注册版本和逐次 401 文件重读；仅本机假凭据。"""
import base64
from datetime import datetime, timezone
import json
from pathlib import Path
import sys
import threading
from test_local3_lifecycle import Rpc
from test_websocket_recovery import Fixture, Handler
from _groundtruth.local3_retry_contract import AUTH_FILE_RELOAD_FAILURES


class AuthHandler(Handler):
    def do_POST(self):
        if self.path.endswith('/server/enroll'):
            payload = json.loads(self.read_exact(int(self.headers['Content-Length'])))
            self.server.enrollment = {'body': payload, 'ua': self.headers.get('User-Agent')}
            body = b'{}'
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()
            self.server.enrolled.set()
            return
        self.server.authorizations.append(self.headers.get('Authorization'))
        super().do_POST()


class AuthFixture(Fixture):
    def __init__(self, home):
        super().__init__('auth-version')
        self.RequestHandlerClass = AuthHandler
        self.home, self.authorizations = home, []
        self.enrolled = threading.Event()
        self.enrollment = None
        self.reload_count = 0
    def http_error(self, payload, compact):
        if self.reload_count and len(self.authorizations) <= self.reload_count:
            index = len(self.authorizations)
            (self.home/'auth.json').write_text(json.dumps({'OPENAI_API_KEY': f'sk-fixture-{index}'}), encoding='utf-8')
            return 401, {'error': {'message': 'fixture rejected old credentials'}}
        return None


def verify(binary, root, version):
    root.mkdir(parents=True, exist_ok=True)
    home = root/'home'
    home.mkdir(exist_ok=True)
    fixture = AuthFixture(home)
    worker = threading.Thread(target=fixture.serve_forever, daemon=True)
    worker.start()
    rpc = None
    try:
        (home/'auth.json').write_text(json.dumps({'OPENAI_API_KEY': 'sk-fixture-0'}), encoding='utf-8')
        fixture.reload_count = AUTH_FILE_RELOAD_FAILURES
        extra = ['-c', 'cli_auth_credentials_store="file"',
                 '-c', 'model_providers.fixture.requires_openai_auth=true',
                 '-c', 'model_providers.fixture.supports_websockets=false',
                 '-c', 'model_providers.fixture.request_max_retries=5']
        rpc = Rpc(binary, fixture, home, root/'reload', extra)
        tid = rpc.start(home)
        rpc.turn(tid, 'Reply briefly.')
        assert fixture.authorizations == [f'Bearer sk-fixture-{i}' for i in range(fixture.reload_count + 1)]
        rpc.stop()
        rpc = None
        fixture.reload_count = 0
        def part(value):
            return base64.urlsafe_b64encode(json.dumps(value).encode()).decode().rstrip('=')
        # 与官方 app-server/tests/common/auth_fixtures.rs 的本机假 JWT 格式一致。
        token = part({'alg': 'none', 'typ': 'JWT'}) + '.' + part({
            'email': 'fixture@example.invalid', 'https://api.openai.com/auth': {
                'chatgpt_account_id': 'fixture-account', 'chatgpt_user_id': 'fixture-user',
                'chatgpt_plan_type': 'plus'}}) + '.c2lnbmF0dXJl'
        (home/'auth.json').write_text(json.dumps({'auth_mode': 'chatgpt', 'OPENAI_API_KEY': None,
            'tokens': {'id_token': token, 'access_token': 'fixture-access', 'refresh_token': 'fixture-refresh',
                       'account_id': 'fixture-account'},
            'last_refresh': datetime.now(timezone.utc).isoformat()}), encoding='utf-8')
        rpc = Rpc(binary, fixture, home, root/'registration', [
            '-c', 'cli_auth_credentials_store="file"', '-c', f'chatgpt_base_url="http://127.0.0.1:{fixture.server_port}/backend-api/"',
            '--remote-control'])
        assert fixture.enrolled.wait(8), 'real remote-control enrollment request missing'
        assert fixture.enrollment['body']['app_server_version'] == version
        assert '-local3' not in fixture.enrollment['ua']
        report = {'passed': True, 'file_reload_failures': AUTH_FILE_RELOAD_FAILURES,
                  'enrollment': fixture.enrollment}
        (root/'result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report), flush=True)
    finally:
        if rpc:
            rpc.stop()
        fixture.shutdown()
        fixture.server_close()
        worker.join(2)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve(), sys.argv[3])
