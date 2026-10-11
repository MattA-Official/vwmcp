# vwmcp documentation

Organised by what you need right now, following [Diátaxis](https://diataxis.fr/).

## Tutorial: learning by doing

- [Your first session](tutorials/first-session.md): install vwmcp, connect Vectorworks, and have Claude draw, inspect and check its own work.

## How-to guides: getting a specific job done

- [Update vwmcp](how-to/update.md)
- [Set up Vectorworks by hand](how-to/register-manually.md), if one-click setup isn't available
- [Use a different keyboard shortcut](how-to/change-shortcut.md)
- [Move over from vwx-mcp](how-to/migrate-from-vwx-mcp.md)
- [Install and use a resource library](how-to/resource-libraries.md)
- [Connect an MCP client other than Claude](how-to/other-clients.md)
- [Fix common problems](how-to/troubleshoot.md)
- [Uninstall](how-to/uninstall.md)

## Reference: facts to look up

- [MCP tools](reference/tools.md)
- [Command line](reference/cli.md)
- [Configuration and files](reference/configuration.md)
- [Scripting inside Vectorworks: `params`, `__result__` and the `vw` helpers](reference/scripting.md)
- [Vectorworks 2026+ API pitfalls](reference/api-pitfalls.md)

## Explanation: how and why it works

- [How vwmcp talks to Vectorworks](explanation/architecture.md): the menu command, the shortcut, the job queue
- [Design decisions](explanation/design.md): a small code-first toolset, docs fetched rather than bundled, one binary
- [Safety and trust](explanation/safety.md): what vwmcp can do on your machine and how it limits damage
