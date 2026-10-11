# Changelog

All notable changes to vwmcp are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and vwmcp uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Until 1.0, minor versions may change tools, flags or file locations; such changes are listed under **Changed**.

## [Unreleased]

## [0.3.0] - 2026-10-11

First public release.

### Added

- MCP server for Vectorworks 2026 and later on Windows, as a single self-contained `vwmcp.exe`.
- Tools: `run_python`, `vs_lookup`, `vs_docs`, `document_overview`, `query_objects`, `screenshot`, `library_list`, `library_browse`, `library_install`, `library_import`, `status` and `job_result`.
- Runs while Vectorworks is in the background, and restores a minimised Vectorworks behind other windows for the duration of a job.
- Closes Vectorworks script-error dialogs automatically, so an error can't stall later jobs.
- Installer: double-click to install, register with Claude Desktop and Claude Code, add vwmcp to `PATH`, and prepare a one-click package for Vectorworks' **Install Partner Products**.
- `vwmcp setup`, `vwmcp doctor` and `vwmcp uninstall`.
- `vs.*` API reference fetched from Vectorworks' public developer documentation and cached.
- Resource library install, browse and import, with skip, replace or rename on name clashes.
- Documentation organised as tutorials, how-to guides, reference and explanation.

[Unreleased]: https://github.com/MattA-Official/vwmcp/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/MattA-Official/vwmcp/releases/tag/v0.3.0
