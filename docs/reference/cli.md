# Command line

`vwmcp [COMMAND]`

Run with no command from a terminal (or by double-clicking), it does `install`. When an MCP client starts it with no terminal attached, it does `serve`.

| Command | Does |
|---|---|
| `serve` | Runs the MCP server on stdin/stdout. MCP clients run this; you don't need to. |
| `install [--name NAME] [--no-claude-desktop] [--no-claude-code]` | Copies itself to `%LOCALAPPDATA%\vwmcp\bin`, adds that folder to your user PATH, writes the Vectorworks-side scripts, registers with Claude Desktop and Claude Code (as `vectorworks` unless `--name` says otherwise), and prepares Vectorworks (see `setup`). Also how you update. |
| `setup [--manual]` | Prepares every Vectorworks 2026+ that you've run: reports versions already set up, and writes the one-click package for the rest. `--manual` prints the Plug-in Manager steps and copies the script line to the clipboard. |
| `doctor` | Checks Vectorworks is running and unblocked, sends a test job, and reports the round-trip time, Python and Vectorworks versions, or what's wrong. |
| `uninstall [--name NAME]` | Unregisters from Claude Desktop and Claude Code, removes vwmcp from PATH, deletes its scripts, queue, cache and the Vectorworks package. Leaves `bin\`. |
| `--version`, `--help` | |

Exit status is non-zero on error. Double-clicked runs wait for Enter before closing.
