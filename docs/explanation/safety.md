# Safety and trust

## `run_python` is real code

`run_python` runs any Python inside Vectorworks, with the same rights as Vectorworks: it can change or delete anything in your drawings, read and write files your Windows account can reach, and use the network. That's what makes vwmcp capable, and it's why vwmcp doesn't try to restrict it.

What limits the damage:

- **Your MCP client asks first.** Claude Desktop asks before running a tool unless you've allowed it. `run_python`, `library_import` and `library_install` are marked as making changes; the others as read-only.
- **Each script is one undo step**, so a bad edit to the drawing is one Ctrl+Z away.
- **Save often**, and keep important work under version control or backed up.

Be careful with text Claude reads from elsewhere (a web page, a file, a PDF). It could contain instructions aimed at Claude. Read scripts before approving them when you're working with material you don't trust.

## Library files

`.vwx` files can contain scripts (plug-in objects, worksheet scripts). `library_install` only fetches over `https`, refuses web pages served instead of files, and never overwrites without being asked. It can't judge whether a library is trustworthy, so only install from sources you'd trust if you were downloading the file yourself.

## The keyboard trigger

vwmcp posts its shortcut into Vectorworks' message queue and touches no other application. Its error-dialog handling only acts on Vectorworks windows whose title or text marks them as script errors, and only clicks OK or closes them.

## What stays on your machine

vwmcp has no telemetry and runs no server. The only network requests it makes:

- downloading Vectorworks' API reference from GitHub (`vs_lookup`, `vs_docs`), and
- downloading a library when you ask (`library_install`).

Jobs and results are files under `%LOCALAPPDATA%\vwmcp\ipc`, inside your Windows user profile. Results are deleted when collected.
