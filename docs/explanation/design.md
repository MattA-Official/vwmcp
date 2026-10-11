# Design decisions

## A dozen tools, not hundreds

vwx-mcp wrapped Vectorworks in 248 tools. Every tool's name, description and parameters are loaded into the model's context before it does anything. At that size, a large share of the context goes on tools it won't use, and it has to choose among near-duplicates.

vwmcp gives Claude one powerful tool, `run_python`, and makes it reliable:

- **`vs_lookup` and `vs_docs`** give the exact signature and remarks of any of Vectorworks' ~3,000 functions, so scripts are right first time. That matters: a wrong argument count opens a modal dialog rather than raising an exception.
- **The `run_python` description lists the known pitfalls** (functions that silently do nothing, calls that hang), so Claude sees them every time.
- **The `vw` helpers** wrap the patterns that are easy to get wrong.

The other tools cover what scripts can't do from inside Vectorworks (`screenshot`, `library_install`) or what Claude needs constantly (`document_overview`, `query_objects`).

The cost: Claude writes small scripts where vwx-mcp had a ready-made button, so a call takes a few hundred tokens of code instead of a few parameters. In practice the agent was writing scripts for anything non-trivial anyway.

## The API reference is fetched, not bundled

Vectorworks publishes its scripting reference on GitHub (`Vectorworks/developer-scripting`), including the `vs.py` stub with every function's signature. vwmcp downloads it on first use, caches it, and refreshes it monthly.

- It stays current: a new Vectorworks version updates the stub, not vwmcp.
- That material is Vectorworks', marked "All rights reserved", so vwmcp doesn't redistribute it.

## One binary

`vwmcp.exe` contains the server, the installer, the trigger and the Python that runs inside Vectorworks. There's no Python environment, package manager or service to keep working: the binary is the install, and running a newer one is the upgrade.

## Setup through Vectorworks' own installer

A Vectorworks command can only be created from inside Vectorworks, and vwmcp can't run anything inside Vectorworks until the command exists. Vectorworks' **Install Partner Products** feature breaks that loop: it reads packages from a local folder (documented for developers testing packages), installs the plug-in, runs a script that adds the menu item and shortcut through the workspace API, and restarts.

The plug-in it installs is an unlocked script, so Vectorworks needs no developer credentials for it and doesn't show its "unknown developer" warning.

The fallback, creating the command by hand in the Plug-in Manager, still works if the installer is unavailable.

## Background operation over focus

vwmcp never brings Vectorworks to the front. The background shortcut, restoring a minimised window behind others, and window capture for screenshots all exist so that you can keep working while Claude works.
