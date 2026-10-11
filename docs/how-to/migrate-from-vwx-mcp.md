# Move over from vwx-mcp

vwmcp replaces vwx-mcp's server, Python bridge files and native palette.

1. Install vwmcp and connect Vectorworks as in the [tutorial](../tutorials/first-session.md). Installing through **Install Partner Products** moves Ctrl+Shift+B off vwx-mcp's **VWX Bridge Start** command (or an earlier **VW MCP** command), so the two don't share it.

   If you'd already pasted vwmcp's line into **VWX Bridge Start**, that works too; vwmcp's installer sees it and reports Vectorworks as set up.
2. Run `vwmcp doctor`.
3. Remove vwx-mcp from Claude Desktop's configuration, so Claude doesn't see two sets of Vectorworks tools. Edit `%APPDATA%\Claude\claude_desktop_config.json` and delete the `vwx-mcp` entry under `mcpServers`. (vwmcp's installer warns you if one is still there.)
4. Optional clean-up, with Vectorworks closed:
   - Delete `%APPDATA%\Nemetschek\Vectorworks\<year>\Plug-ins\VWX-MCP` (or `VW-MCP`).
   - Delete `VwxBridge.vlb` and `VwxBridge.vwr` from `C:\Program Files\Vectorworks <year>\Plug-ins` (needs admin).
   - Delete vwx-mcp's **VWX Bridge Start** command in the Plug-in Manager.
   - Delete `%USERPROFILE%\.local\share\vwx-mcp` and `%USERPROFILE%\bridge\vwx-mcp.bat`.

## What changes for Claude

vwx-mcp offered 248 tools; vwmcp offers [12](../reference/tools.md). Anything a vwx-mcp tool did, Claude now does with `run_python`, after checking the function with `vs_lookup`. The same Vectorworks pitfalls are documented in the `run_python` tool description, so Claude sees them every time.
