"""End-to-end test: the real server and the real Vectorworks-side pump, with a
fake `vs` module standing in for Vectorworks.

    python3 tests/harness/e2e.py target/debug/vwmcp

Runs on any OS (the Windows trigger is the only part not exercised: the fake
Vectorworks polls the job queue instead of waiting for the shortcut).
Exits non-zero on the first failed check.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def menu_command_script():
    src = open(os.path.join(ROOT, 'src', 'paths.rs'), encoding='utf-8').read()
    return re.search(r'MENU_COMMAND_SCRIPT: &str = "(.*)";', src).group(1)


class Client:
    def __init__(self, exe, env):
        self.proc = subprocess.Popen([exe, 'serve'], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, text=True, env=env)
        self.id = 0

    def rpc(self, method, params=None, notify=False):
        msg = {'jsonrpc': '2.0', 'method': method}
        if params is not None:
            msg['params'] = params
        if not notify:
            self.id += 1
            msg['id'] = self.id
        self.proc.stdin.write(json.dumps(msg) + '\n')
        self.proc.stdin.flush()
        if notify:
            return None
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise SystemExit('server closed stdout')
            r = json.loads(line)
            if r.get('id') == self.id:
                return r

    def call(self, name, args=None):
        res = self.rpc('tools/call', {'name': name, 'arguments': args or {}})['result']
        text = '\n'.join(c.get('text', '') for c in res.get('content', []))
        return bool(res.get('isError')), text

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=10)


failures = 0


def check(label, cond, detail=''):
    global failures
    print(('ok    ' if cond else 'FAIL  ') + label)
    if not cond:
        failures += 1
        if detail:
            print('      ' + detail.replace('\n', '\n      ')[:2000])


def main(exe):
    tmp = tempfile.mkdtemp(prefix='vwmcp-e2e-')
    user = os.path.join(tmp, 'user')
    os.makedirs(os.path.join(user, 'Libraries'))
    lib_src = os.path.join(tmp, 'Stage.vwx')
    with open(lib_src, 'w') as f:
        f.write('Truss 3m\nTruss 2m\nPar 64\n')
    env = dict(os.environ, VWMCP_HOME=os.path.join(tmp, 'home'), FAKE_VW_USER=user, VWMCP_LOG='warn')

    vw = subprocess.Popen([sys.executable, os.path.join(HERE, 'fake_vw.py'), menu_command_script(), '120'],
                          env=env, stderr=subprocess.DEVNULL)
    c = Client(exe, env)
    try:
        init = c.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
                                    'clientInfo': {'name': 'e2e', 'version': '0'}})
        check('initialize', init['result']['serverInfo']['name'] == 'vwmcp')
        c.rpc('notifications/initialized', notify=True)
        tools = {t['name'] for t in c.rpc('tools/list', {})['result']['tools']}
        check('12 tools listed', len(tools) == 12, str(sorted(tools)))

        err, text = c.call('status')
        check('status round trip', not err and '"ok": true' in text, text)

        err, text = c.call('run_python', {'code': "h = vw.circle(0, 0, 2)\nprint('drew')\n__result__ = vw.info(h)"})
        check('run_python draws and returns data', not err and '"type": "oval"' in text and 'drew' in text, text)

        err, text = c.call('run_python', {'code': "__result__ = params['n'] * 2", 'params': {'n': 21}})
        check('run_python params', not err and '"result": 42' in text, text)

        err, text = c.call('run_python', {'code': 'def f():\n    return 1/0\nf()'})
        check('run_python reports exceptions', err and 'ZeroDivisionError' in text and 'vwmcp_pump' not in text, text)

        err, text = c.call('run_python', {'code': "__result__ = {'s': {1}, 'n': float('nan')}"})
        check('run_python converts odd values', not err and '"n": null' in text, text)

        err, text = c.call('run_python', {'code': 'import time\ntime.sleep(2)\n__result__ = 1', 'timeout_s': 1})
        jid = re.search(r"id '([0-9a-f]+)'", text)
        check('long job reports a job id', err and jid is not None, text)
        if jid:
            time.sleep(2.5)
            err, text = c.call('job_result', {'id': jid.group(1)})
            check('job_result collects it', not err and '"result": 1' in text, text)
        err, text = c.call('job_result', {'id': '../../etc'})
        check('job_result rejects bad ids', err, text)

        err, text = c.call('document_overview')
        check('document_overview', not err and 'Design Layer-1' in text, text)

        err, text = c.call('query_objects', {'criteria': '(T=RECT)'})
        check('query_objects', not err and '"total": 1' in text and 'Stage' in text, text)

        err, text = c.call('library_install', {'source': lib_src, 'subfolder': 'Stage'})
        check('library_install', not err and 'Stage.vwx' in text, text)
        err, text = c.call('library_install', {'source': lib_src, 'subfolder': 'Stage'})
        check('library_install refuses to overwrite', err and 'already exists' in text, text)
        err, text = c.call('library_install', {'source': lib_src, 'subfolder': '../escape'})
        check('library_install rejects unsafe subfolders', err, text)

        installed = os.path.join(user, 'Libraries', 'Stage', 'Stage.vwx')
        err, text = c.call('library_browse', {'file': installed, 'filter': 'truss'})
        check('library_browse', not err and '"total": 2' in text, text)

        err, text = c.call('library_import', {'file': installed, 'names': ['Truss 3m', 'Nope'], 'on_conflict': 'rename'})
        check('library_import (rename on conflict)', not err and 'Truss 3m-2' in text and 'not_found' in text, text)
        err, text = c.call('library_import', {'file': installed, 'names': ['Truss 2m'], 'on_conflict': 'replace'})
        check('library_import (replace)', not err and '"imported"' in text, text)
    finally:
        c.close()
        vw.kill()
    print('\n%d check(s) failed' % failures if failures else '\nall checks passed')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'target', 'debug', 'vwmcp')))
