# Configuration and files

## Environment variables

| Variable | Default | Meaning |
|---|---|---|
| `VWMCP_HOTKEY` | `ctrl+shift+b` | Shortcut assigned to the vwmcp command. Needs `ctrl` or `alt`, plus a letter, a digit or `f1`–`f24` (function keys only with manual set-up). Modifier names: `ctrl`/`control`/`strg`, `shift`/`umschalt`, `alt`. |
| `VWMCP_HOME` | `%LOCALAPPDATA%\vwmcp` | Where everything below lives. If you change it, set it for Vectorworks as well (system-wide user variable), since the menu command reads it. |
| `VWMCP_LOG` | `info` | Log filter (`error`, `warn`, `info`, `debug`, `trace`), written to stderr. |
| `VWMCP_DOCS_BASE` | Vectorworks' GitHub raw URL | Where `vs_lookup`/`vs_docs` fetch the API reference from. |

## Files

Under `VWMCP_HOME`:

| Path | Written by | Contents |
|---|---|---|
| `bin\vwmcp.exe` | `install` | The installed program. `vwmcp.exe.old-*` are copies set aside during an update. |
| `vw\entry.py`, `vw\vwmcp_pump.py`, `vw\vw.py` | the server, on every start | Code run inside Vectorworks. Rewritten only when changed. |
| `ipc\jobs\*.json` | the server | Queued jobs. `*.working` while Vectorworks runs them. |
| `ipc\results\*.json` | Vectorworks | Results, removed when collected (or after an hour). |
| `ipc\pump.json` | Vectorworks | Heartbeat: last run time, process id, Python and Vectorworks versions. |
| `cache\vs.py`, `cache\pages\*.md` | `vs_lookup`, `vs_docs` | API reference, refreshed after 30 days. |

Under `%APPDATA%\Nemetschek\Vectorworks\<year>\`:

| Path | Written by | |
|---|---|---|
| `Plug-ins\Data\PartnerInstallerDev\vwmcp.xml`, `vwmcp.zip` | `install`, `setup` | The one-click package shown in **Install Partner Products**. |
| `Plug-ins\vwmcp\vwmcp.vsm` | Vectorworks, when you install the package | The vwmcp menu command. |

Elsewhere:

| Path | |
|---|---|
| `%APPDATA%\Claude\claude_desktop_config.json` | Gets a `vectorworks` entry; the previous version is saved as `.json.vwmcp-backup`. |
| User `Path` environment variable | Gets `%LOCALAPPDATA%\vwmcp\bin`. |

## Limits

| | |
|---|---|
| Wait for Vectorworks to pick up a job | 15 s, then the job is withdrawn unrun |
| Wait for a running script | `timeout_s`, default 120 s |
| New jobs started per Vectorworks drain | until 3 s have passed; the rest run on the next trigger |
| Script output returned | 200,000 characters |
| Re-minimise Vectorworks after the last job | 2.5 s |
