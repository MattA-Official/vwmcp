# Uninstall

1. In Vectorworks: **Help › Install Partner Products…**, find **vwmcp**, click **Uninstall**. (If you set it up by hand, delete the **vwmcp** command in **Tools › Plug-ins › Plug-in Manager** instead.)
2. Close Claude Desktop.
3. Run `vwmcp uninstall` from a terminal. It removes vwmcp from Claude Desktop and Claude Code, deletes its scripts, queue and cache, and removes the install package from Vectorworks' installer folder.
4. Delete `%LOCALAPPDATA%\vwmcp`. (The program itself is left for you to delete, because it may still be running.)
