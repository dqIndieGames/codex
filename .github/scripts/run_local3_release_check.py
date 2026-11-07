"""发布测试的累计预算入口；所有子进程归入本轮 Job，绝不编译目标程序。"""
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from _groundtruth.local3_retry_contract import (
    RELEASE_TEST_SECONDS, RELEASE_CHECK_RESERVE_SECONDS, CLEANUP_RESERVE_SECONDS,
)


def main():
    ledger_path = Path(sys.argv[1]).resolve()
    mode = sys.argv[2]
    ledger = json.loads(ledger_path.read_text(encoding='utf-8')) if ledger_path.exists() else {'used_seconds': 0, 'checks': []}
    if ledger.get('active'):
        if mode != 'visual-end':
            raise SystemExit('存在未结算验证段，须先结算；不能重置额度')
        elapsed = time.time() - ledger.pop('active')['started_at']
        ledger['used_seconds'] += elapsed
        ledger['checks'].append({'name': '实际打开图片验收', 'seconds': elapsed})
        ledger_path.write_text(json.dumps(ledger, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps(ledger, ensure_ascii=False))
        if ledger['used_seconds'] > RELEASE_TEST_SECONDS - RELEASE_CHECK_RESERVE_SECONDS - CLEANUP_RESERVE_SECONDS:
            raise SystemExit('视觉验收超出剩余额度，不得发布')
        return
    limit = RELEASE_TEST_SECONDS - CLEANUP_RESERVE_SECONDS
    if mode != 'release':
        limit -= RELEASE_CHECK_RESERVE_SECONDS
    remaining = limit - ledger['used_seconds']
    if remaining <= 0:
        raise SystemExit('累计验证额度已用尽，不得继续测试或发布')
    ledger_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    ledger['active'] = {'started_at': time.time(), 'mode': mode}
    ledger_path.write_text(json.dumps(ledger, ensure_ascii=False, indent=2), encoding='utf-8')
    if mode == 'visual-start':
        print(json.dumps({'visual_seconds_available': remaining}))
        return
    command = sys.argv[3:]
    if not command:
        raise SystemExit('缺少测试命令')
    kernel = C.WinDLL('kernel32', use_last_error=True)
    nt = C.WinDLL('ntdll')
    kernel.CreateJobObjectW.argtypes = [C.c_void_p, W.LPCWSTR]
    kernel.CreateJobObjectW.restype = W.HANDLE
    kernel.AssignProcessToJobObject.argtypes = [W.HANDLE, W.HANDLE]
    kernel.TerminateJobObject.argtypes = [W.HANDLE, W.UINT]
    kernel.QueryInformationJobObject.argtypes = [W.HANDLE, C.c_int, C.c_void_p, W.DWORD, C.c_void_p]
    kernel.CloseHandle.argtypes = [W.HANDLE]
    nt.NtResumeProcess.argtypes = [W.HANDLE]
    class Accounting(C.Structure):
        _fields_ = [('user', C.c_longlong), ('kernel', C.c_longlong), ('period_user', C.c_longlong),
                    ('period_kernel', C.c_longlong), ('faults', W.DWORD), ('total', W.DWORD),
                    ('active', W.DWORD), ('terminated', W.DWORD)]
    job = kernel.CreateJobObjectW(None, None)
    if not job:
        raise C.WinError(C.get_last_error())
    process = None
    code, cleanup_ok = 1, False
    try:
        env = os.environ.copy()
        env['LOCAL3_CHECK_REMAINING_SECONDS'] = str(remaining)
        # 启动后先分配 Job，再恢复线程，防止子进程逃出本轮清理范围。
        process = subprocess.Popen(command, env=env, creationflags=0x4 | subprocess.CREATE_NO_WINDOW)
        if not kernel.AssignProcessToJobObject(job, int(process._handle)):
            process.kill()
            raise C.WinError(C.get_last_error())
        if nt.NtResumeProcess(int(process._handle)) != 0:
            raise RuntimeError('无法恢复本轮测试进程')
        try:
            code = process.wait(timeout=max(.01, remaining - (time.monotonic() - started)))
        except subprocess.TimeoutExpired:
            code = 124
    finally:
        kernel.TerminateJobObject(job, 1)
        deadline = time.monotonic() + min(CLEANUP_RESERVE_SECONDS, RELEASE_TEST_SECONDS - ledger['used_seconds'] - (time.monotonic() - started))
        while time.monotonic() < deadline:
            info = Accounting()
            if not kernel.QueryInformationJobObject(job, 1, C.byref(info), C.sizeof(info), None):
                break
            if info.active == 0:
                cleanup_ok = True
                break
            time.sleep(.05)
        if process is not None and cleanup_ok:
            process.wait(timeout=.1)
        kernel.CloseHandle(job)
        elapsed = time.monotonic() - started
        ledger.pop('active', None)
        ledger['used_seconds'] += elapsed
        ledger['checks'].append({'command': command, 'seconds': elapsed, 'exit_code': code, 'process_tree_clean': cleanup_ok})
        ledger_path.write_text(json.dumps(ledger, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'used_seconds': ledger['used_seconds'], 'exit_code': code, 'process_tree_clean': cleanup_ok}), flush=True)
    if not cleanup_ok or ledger['used_seconds'] > RELEASE_TEST_SECONDS:
        raise SystemExit('验证清理或累计时限失败，不得发布')
    raise SystemExit(code)


if __name__ == '__main__':
    main()
