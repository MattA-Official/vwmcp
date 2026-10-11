# vwmcp entry point — executed by the Vectorworks menu command on every trigger.
#
# The menu command itself contains only a one-liner that execs this file, so
# upgrading vwmcp never requires touching Vectorworks again: `vwmcp` rewrites
# this folder and the pump hot-reloads when its file changes.
import os
import sys
import importlib

_home = os.environ.get('VWMCP_HOME') or os.path.join(
    os.environ.get('LOCALAPPDATA', ''), 'vwmcp')
_dir = os.path.join(_home, 'vw')
if _dir not in sys.path:
    sys.path.insert(0, _dir)

import vwmcp_pump  # noqa: E402

try:
    _mt = os.path.getmtime(os.path.join(_dir, 'vwmcp_pump.py'))
except OSError:
    _mt = 0.0
if getattr(vwmcp_pump, '_vwmcp_mtime', None) != _mt:
    vwmcp_pump = importlib.reload(vwmcp_pump)
    vwmcp_pump._vwmcp_mtime = _mt

vwmcp_pump.drain()
