# Update vwmcp

1. Close Claude Desktop (and any other MCP client using vwmcp).
2. Download the new `vwmcp.exe` from the [latest release](https://github.com/MattA-Official/vwmcp/releases/latest) and run it.
3. Reopen Claude Desktop.

The new `vwmcp.exe` copies itself over the installed one in `%LOCALAPPDATA%\vwmcp\bin`, refreshes the scripts Vectorworks runs, and keeps your Claude registration. You don't need to delete or replace anything by hand, and the download can go afterwards.

Nothing needs doing in Vectorworks: its `vwmcp` command loads vwmcp's scripts from disk each time, so it always picks up the new version.

If Claude Desktop was still running, the update still installs: the old copy is moved aside and removed next time. Claude keeps using the old version until you restart it.

To check which version is running, ask Claude to check the Vectorworks connection (`status` reports `server_version`), or run `vwmcp --version`.
