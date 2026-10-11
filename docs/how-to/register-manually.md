# Set up Vectorworks by hand

Use this if **Help › Install Partner Products** doesn't list vwmcp, for example because your organisation manages Vectorworks centrally. Otherwise use the one-click route in the [tutorial](../tutorials/first-session.md#2-connect-vectorworks).

1. Run `vwmcp setup --manual` in a terminal. It prints the one line of Python and copies it to your clipboard.
2. In Vectorworks: **Tools › Plug-ins › Plug-in Manager… › Custom Plug-ins › New…**
   - Type: **Command**. Name: **vwmcp**.
   - **Edit Script…**, set the language to **Python**, paste the line, click **OK**.
3. **Tools › Workspaces › Edit Current Workspace… › Menus**: drag **vwmcp** into any menu (Tools › Utilities is a good home). Select it there and set the shortcut to **Ctrl+Shift+B**. Click **OK**.
4. Run `vwmcp doctor` to confirm.

Menu names vary a little between Vectorworks versions and languages.

The line never changes between vwmcp versions, so you only do this once per Vectorworks version.
