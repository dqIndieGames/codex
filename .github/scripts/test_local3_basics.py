"""SOP local downloaded-exe identity and fake-key account isolation checks."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def verify(binary, output, version):
    output.mkdir(parents=True, exist_ok=True)
    home = output / 'home'
    home.mkdir(exist_ok=True)
    env = os.environ.copy()
    env['CODEX_HOME'] = str(home)
    for key in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'CODEX_TUI_COMPLETION_DIAGNOSTICS_DIR'):
        env.pop(key, None)
    def run(args, input=None):
        result = subprocess.run([binary, *args], input=input, cwd=home, env=env,
            capture_output=True, text=True, encoding='utf-8', timeout=30,
            creationflags=subprocess.CREATE_NO_WINDOW)
        assert result.returncode == 0, (args, result.stderr)
        return result.stdout
    actual = run(['--version']).strip()
    assert version + '-local3' in actual.split(), actual
    assert '--account <NAME>' in run(['--help'])
    for sub in ('exec', 'app-server'):
        run([sub, '--help'])
    options = ['-c', 'forced_login_method="api"', '-c', 'cli_auth_credentials_store="file"']
    run([*options, 'login', '--with-api-key'], 'sk-default-fixture\n')
    default = home / 'auth.json'
    hash_before = hashlib.sha256(default.read_bytes()).hexdigest()
    run([*options, '--account', 'alpha', 'login', '--with-api-key'], 'sk-alpha-fixture\n')
    assert (home / 'accounts/alpha/auth.json').is_file()
    assert hashlib.sha256(default.read_bytes()).hexdigest() == hash_before
    run([*options, '--account', 'alpha', 'logout'])
    assert not (home / 'accounts/alpha/auth.json').exists()
    assert hashlib.sha256(default.read_bytes()).hexdigest() == hash_before
    report = {'passed': True, 'version': actual, 'help': True, 'account_isolation': True,
              'exe_sha256': hashlib.sha256(Path(binary).read_bytes()).hexdigest()}
    (output / 'result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report), flush=True)


if __name__ == '__main__':
    verify(str(Path(sys.argv[1]).resolve(strict=True)), Path(sys.argv[2]).resolve(), sys.argv[3])
