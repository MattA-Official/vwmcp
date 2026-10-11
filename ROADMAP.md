# Roadmap to 1.0

vwmcp works today on Vectorworks 2027 for Windows. 1.0 means it works for people who aren't its author: on every supported Vectorworks version and language, installed and updated without warnings or manual steps, with a tool interface that won't change under them.

Each item below has a tracking issue in the matching [milestone](https://github.com/MattA-Official/vwmcp/milestones). Order within a milestone is rough priority. Comment on an issue before starting work on it.

## What 1.0 means

- **Works on every supported version.** Vectorworks 2026 and later, English and German at least, verified by someone other than the author.
- **Installs cleanly.** No SmartScreen warning, and one click in Vectorworks at most.
- **Updates itself.** No re-downloading by hand.
- **Never stalls silently.** Any job either finishes, fails with a reason, or can be cancelled.
- **Stable interface.** Tool names, parameters and result shapes, the CLI and the `vw` helpers follow semantic versioning from 1.0 on.

## 0.4: Reliability

Make the bridge trustworthy on setups other than the author's.

- **Verify Vectorworks 2026.** One-click install, plug-in loading (the bundled plug-in was saved by 2027), the trigger and dialog handling.
- **Non-English Vectorworks.** Check error-dialog detection and Install Partner Products on German builds; collect dialog titles for other languages.
- **Cancel jobs.** Support MCP cancellation: withdraw queued jobs, and report clearly when a running script can't be stopped.
- **Progress for long scripts.** Send MCP progress notifications while a job runs, so clients don't look frozen.
- **Recover after a Vectorworks crash.** Detect `*.working` jobs orphaned by a crash or restart and report them as failed, instead of waiting for the timeout.
- **Confirm undo behaviour.** Verify that each `run_python` call is exactly one undo step in all cases (including errors part-way through), and document any exceptions.
- **Windows CI for the trigger.** Test the keystroke delivery and dialog detection against a stand-in window on the Windows runner.

## 0.5: Install and update

- **Code signing.** Sign `vwmcp.exe` (for example with SignPath's free open-source programme or Azure Trusted Signing) so Windows doesn't warn on download.
- **`vwmcp update`.** Self-update from GitHub releases, verifying the published SHA-256, and offer it from `vwmcp doctor` when a newer release exists.
- **Package managers.** Publish to winget and Scoop.
- **Official partner listing.** Ask Vectorworks to list vwmcp in Install Partner Products, so setup no longer relies on the local developer folder.
- **Pre-fetch the API reference** during install, so `vs_lookup` works offline from the start.

## 0.6: Tools

Polish the agent's experience. The toolset stays small; see [design decisions](docs/explanation/design.md).

- **Tool allowlist / read-only mode.** A setting to expose only read-only tools, for cautious use or shared machines.
- **Better screenshots.** Capture just the drawing area, or a given region, at full resolution.
- **MCP resources and prompts.** Expose the document overview as a resource, and ship prompts for common workflows (for example "draft a lighting plot from this list").
- **Streamable HTTP transport.** Optional, for clients that can't launch a local process.
- **More `vw` helpers.** Driven by pitfall reports: whatever agents keep getting wrong.

## 1.0: Stability

- **Freeze the interface.** Review every tool name, parameter, result field, CLI flag and `vw` helper; remove or rename anything awkward before the freeze.
- **Documentation site.** Publish the docs (for example mdBook on GitHub Pages).
- **Support policy.** Which Vectorworks versions are supported, and for how long after a new one ships.

## Later, or out of scope for 1.0

- **macOS.** The file queue and Vectorworks side would carry over; the trigger and dialog handling need a macOS equivalent (for example AppleScript). Worth investigating after 1.0, or before if someone wants to own it.
- **Vectorworks Cloud libraries** in the library tools.

## Not planned

- **One tool per Vectorworks feature.** That's what vwx-mcp does; vwmcp deliberately doesn't ([why](docs/explanation/design.md#a-dozen-tools-not-hundreds)).
- **Bundling Vectorworks' API reference.** It's Vectorworks' copyrighted material; vwmcp fetches it instead.
