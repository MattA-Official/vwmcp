# Use a different keyboard shortcut

vwmcp uses Ctrl+Shift+B by default. If that clashes with something you use, pick another shortcut. It must include Ctrl or Alt, plus a letter, a digit, or (for manual set-up only) F1–F24.

1. Set the `VWMCP_HOTKEY` environment variable for your Windows user, e.g. `ctrl+alt+m`:
   **Settings › System › About › Advanced system settings › Environment Variables… › New** (user variable).
2. Assign the same shortcut to the vwmcp command in Vectorworks:
   **Tools › Workspaces › Edit Current Workspace… › Menus**, select **vwmcp**, set the shortcut, click **OK**.
3. Restart Claude Desktop so vwmcp sees the new variable.
4. Run `vwmcp doctor`.

If you haven't connected Vectorworks yet, set the variable first and then run `vwmcp setup`. The one-click installer will then assign your shortcut instead of Ctrl+Shift+B.
