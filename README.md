# vwmcp

[![CI](https://github.com/MattA-Official/vwmcp/actions/workflows/ci.yml/badge.svg)](https://github.com/MattA-Official/vwmcp/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MattA-Official/vwmcp)](https://github.com/MattA-Official/vwmcp/releases/latest)
[![Licence: MIT](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)

An MCP server for Vectorworks 2026 and later, on Windows. One self-contained `.exe`: no Python to install, no plug-in to compile, nothing to keep running.

Claude (or any MCP client) can run scripts in your open Vectorworks drawing, look up any `vs.*` function, inspect the document, take screenshots, and install and import resource libraries, all while Vectorworks sits in the background.

## Install

1. Download `vwmcp.exe` from the [latest release](https://github.com/MattA-Official/vwmcp/releases/latest), close Claude Desktop, and double-click it.
   The `.exe` isn't code-signed yet, so Windows may warn: choose **More info › Run anyway** ([tracked](ROADMAP.md#05-install-and-update)).
2. In Vectorworks: **Help › Install Partner Products… › vwmcp › Install**. Vectorworks restarts.
3. Open Claude Desktop and ask it to check the Vectorworks connection.

> **Status (0.3.0):** confirmed on Vectorworks 2027, including the one-click setup, background running and the tools. Vectorworks 2026 is supported but less tested; if vwmcp isn't listed in Install Partner Products, [set it up by hand](docs/how-to/register-manually.md).

That's all. Claude Desktop starts vwmcp itself, and Vectorworks loads its side automatically. To update, run a newer `vwmcp.exe` the same way.

New to it? Follow [Your first session](docs/tutorials/first-session.md).

## Tools

`run_python` · `vs_lookup` · `vs_docs` · `document_overview` · `query_objects` · `screenshot` · `library_list` · `library_browse` · `library_install` · `library_import` · `status` · `job_result`

Details: [MCP tools reference](docs/reference/tools.md).

## Documentation

All in [docs/](docs/README.md), organised with [Diátaxis](https://diataxis.fr/):

- **Tutorial:** [Your first session](docs/tutorials/first-session.md)
- **How-to:** [update](docs/how-to/update.md) · [set up by hand](docs/how-to/register-manually.md) · [change the shortcut](docs/how-to/change-shortcut.md) · [move from vwx-mcp](docs/how-to/migrate-from-vwx-mcp.md) · [resource libraries](docs/how-to/resource-libraries.md) · [other MCP clients](docs/how-to/other-clients.md) · [troubleshoot](docs/how-to/troubleshoot.md) · [uninstall](docs/how-to/uninstall.md)
- **Reference:** [tools](docs/reference/tools.md) · [command line](docs/reference/cli.md) · [configuration and files](docs/reference/configuration.md) · [scripting](docs/reference/scripting.md) · [API pitfalls](docs/reference/api-pitfalls.md)
- **Explanation:** [how it works](docs/explanation/architecture.md) · [design decisions](docs/explanation/design.md) · [safety and trust](docs/explanation/safety.md)

## Contributing

Bug reports, Vectorworks pitfalls, docs and code are all welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup (most work needs neither Windows nor Vectorworks: an end-to-end test runs against a simulated Vectorworks), and the [roadmap](ROADMAP.md) for what's planned on the way to 1.0.

## Licence

MIT. The Windows trigger and the Vectorworks API pitfalls are adapted from [vwx-mcp](https://github.com/vicquick/vwx-mcp) (MIT, © vicquick).
