"""vwmcp pump — runs inside Vectorworks, in the Python menu-command context.

That context is the only one where document mutation is safe on VW2026 (web
palette callbacks, idle notifications and timers crash or hang VW on
mutation), so every job runs here and nowhere else.

The pump is deliberately generic: a job is a piece of Python plus params,
sent by the vwmcp server. Tools are defined server-side, so new or changed
tools never require redeploying anything into Vectorworks.

IPC layout under VWMCP_HOME (default %LOCALAPPDATA%\\vwmcp\\ipc):
  jobs/<seq>-<id>.json      written by the server (atomic tmp + rename)
  jobs/<seq>-<id>.working   claimed by the pump (atomic rename)
  results/<id>.json         written by the pump, consumed by the server
  pump.json                 heartbeat + environment, rewritten every drain
"""
import contextlib
import importlib
import io
import json
import math
import os
import sys
import time
import traceback

HOME = os.environ.get('VWMCP_HOME') or os.path.join(
    os.environ.get('LOCALAPPDATA', ''), 'vwmcp')
VW_DIR = os.path.join(HOME, 'vw')
IPC = os.path.join(HOME, 'ipc')
JOBS = os.path.join(IPC, 'jobs')
RESULTS = os.path.join(IPC, 'results')

# One drain never blocks the VW UI for longer than this; leftover jobs are
# picked up by the next trigger (the server retriggers while jobs wait).
DRAIN_BUDGET_S = 3.0
MAX_OUTPUT_CHARS = 200_000
RESULT_TTL_S = 3600.0

try:
    import vs  # only exists inside Vectorworks
except ImportError:  # pragma: no cover - test harness outside VW
    vs = None


def _atomic_write(path, data):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False)
    os.replace(tmp, path)


def _is_handle(v):
    return 'handle' in type(v).__name__.lower()


def jsonable(v, depth=0):
    """Best-effort conversion of script results to JSON-safe values.
    Vectorworks handles become their UUID string."""
    if depth > 12:
        return repr(v)
    if v is None or isinstance(v, (bool, int, str)):
        return v
    if isinstance(v, float):
        return v if math.isfinite(v) else None
    if isinstance(v, dict):
        return {str(k): jsonable(x, depth + 1) for k, x in v.items()}
    if isinstance(v, (list, tuple, set, frozenset)):
        return [jsonable(x, depth + 1) for x in v]
    if vs is not None and _is_handle(v):
        try:
            return {'handle': vs.GetObjectUuid(v)}
        except Exception:
            return repr(v)
    return repr(v)


def _helpers():
    """Import (or hot-reload) the `vw` helper module."""
    if VW_DIR not in sys.path:
        sys.path.insert(0, VW_DIR)
    import vw
    try:
        mt = os.path.getmtime(os.path.join(VW_DIR, 'vw.py'))
    except OSError:
        mt = 0.0
    if getattr(vw, '_vwmcp_mtime', None) != mt:
        vw = importlib.reload(vw)
        vw._vwmcp_mtime = mt
    return vw


def _trim_traceback(tb_text):
    # Drop the pump's own frames so the agent sees only its script's frames.
    lines = tb_text.splitlines()
    out, skip = [], False
    for line in lines:
        if line.startswith('  File ') and 'vwmcp_pump.py' in line:
            skip = True
            continue
        if skip and line.startswith('    '):
            continue
        skip = False
        out.append(line)
    return '\n'.join(out)


def run_job(job):
    started = time.time()
    buf = io.StringIO()
    ns = {
        '__name__': '__vwmcp__',
        'vs': vs,
        'params': job.get('params') or {},
        '__result__': None,
    }
    result = {'id': job.get('id'), 'ok': True}
    try:
        ns['vw'] = _helpers()
        code = compile(job.get('code', ''), '<%s>' % job.get('label', 'script'), 'exec')
        with contextlib.redirect_stdout(buf):
            exec(code, ns)
        result['result'] = jsonable(ns.get('__result__'))
    except BaseException as e:  # noqa: BLE001 - report everything, never kill VW's runner
        result['ok'] = False
        result['error'] = '%s: %s' % (type(e).__name__, e)
        result['traceback'] = _trim_traceback(traceback.format_exc())
    out = buf.getvalue()
    if len(out) > MAX_OUTPUT_CHARS:
        out = out[:MAX_OUTPUT_CHARS] + '\n… [output truncated]'
    result['stdout'] = out
    result['elapsed_ms'] = int((time.time() - started) * 1000)
    return result


def _heartbeat():
    info = {'last_drain': time.time(), 'pid': os.getpid(),
            'python': sys.version.split()[0]}
    if vs is not None:
        try:
            info['vectorworks'] = list(vs.GetVersion())
        except Exception:
            pass
    try:
        _atomic_write(os.path.join(IPC, 'pump.json'), info)
    except OSError:
        pass


def _sweep_results():
    now = time.time()
    try:
        for name in os.listdir(RESULTS):
            p = os.path.join(RESULTS, name)
            try:
                if now - os.path.getmtime(p) > RESULT_TTL_S:
                    os.remove(p)
            except OSError:
                pass
    except OSError:
        pass


def drain():
    for d in (JOBS, RESULTS):
        os.makedirs(d, exist_ok=True)
    _heartbeat()
    deadline = time.time() + DRAIN_BUDGET_S
    for name in sorted(os.listdir(JOBS)):
        if not name.endswith('.json'):
            continue
        if time.time() > deadline:
            break
        src = os.path.join(JOBS, name)
        work = src[:-5] + '.working'
        try:
            os.rename(src, work)  # atomic claim; loses cleanly to a server-side cancel
        except OSError:
            continue
        try:
            with open(work, encoding='utf-8') as f:
                job = json.load(f)
        except Exception as e:  # corrupt job file
            job = {'id': name.split('-', 1)[-1][:-5]}
            res = {'id': job['id'], 'ok': False, 'error': 'unreadable job: %r' % e}
        else:
            res = run_job(job)
        try:
            _atomic_write(os.path.join(RESULTS, '%s.json' % job.get('id')), res)
        finally:
            try:
                os.remove(work)
            except OSError:
                pass
    _sweep_results()
