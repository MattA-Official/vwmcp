# Connect an MCP client other than Claude

vwmcp is a standard MCP server over stdio.

- **Command:** `%LOCALAPPDATA%\vwmcp\bin\vwmcp.exe`
- **Arguments:** `serve`

For example, in a client that uses the common `mcpServers` JSON format:

```json
{
  "mcpServers": {
    "vectorworks": {
      "command": "C:\\Users\\you\\AppData\\Local\\vwmcp\\bin\\vwmcp.exe",
      "args": ["serve"]
    }
  }
}
```

The client starts and stops vwmcp itself; you don't run anything separately.

To register with Claude Code by hand: `claude mcp add --scope user vectorworks -- "%LOCALAPPDATA%\vwmcp\bin\vwmcp.exe" serve`. The installer does this for you when Claude Code is on your PATH.
