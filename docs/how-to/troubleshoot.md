# Fix common problems

Start with `vwmcp doctor` in a terminal, or ask Claude to check the Vectorworks connection. Then find the message below.

## "Vectorworks is not running"

Start Vectorworks. vwmcp looks for a running `Vectorworks*.exe` belonging to your user.

## "A Vectorworks dialog is open"

A modal dialog (a settings window, a save prompt) is blocking Vectorworks. Close it and try again. vwmcp closes Vectorworks' *script error* dialogs itself, but never other dialogs, in case you're working in them.

## "…received the Ctrl+Shift+B shortcut but the vwmcp menu command did not run"

The keystroke arrived but nothing ran it.

- If you've only just installed through **Install Partner Products**, make sure Vectorworks restarted afterwards.
- Check **Tools › Utilities** (or wherever you put it) for **vwmcp**, with the shortcut shown next to it. If it's missing, run `vwmcp setup` again, or [set it up by hand](register-manually.md).
- Check no other command uses the same shortcut (**Tools › Workspaces › Edit Current Workspace › Menus**).

## Vectorworks is minimised

vwmcp restores it behind your other windows to run the job, then minimises it again a few seconds after the last job.

## A "Script Errors" dialog appears

A script hit a Vectorworks engine error. vwmcp closes these dialogs itself and reports the error to Claude, which usually corrects the script and retries. If one stays open, click **OK**. If the same error keeps appearing, tell Claude what the dialog says.

## "The script is still running…"

Long scripts outlive the tool call's wait (2 minutes by default). Claude can collect the result later with `job_result`, or pass a longer `timeout_s`. Vectorworks stays busy until the script finishes.

## `vs_lookup` / `vs_docs` fail on first use

They download Vectorworks' public API reference from GitHub once and cache it. Check your internet connection or proxy. After the first download they work offline.

## Logs

vwmcp logs to stderr; set `VWMCP_LOG=debug` for more. Claude Desktop shows MCP server logs under **Settings › Developer**.
