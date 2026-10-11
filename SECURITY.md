# Security policy

vwmcp runs code inside Vectorworks on behalf of an AI agent, so security reports matter.

## Supported versions

Only the latest release receives fixes until 1.0.

## Reporting a vulnerability

Please report privately through **[GitHub's private vulnerability reporting](https://github.com/MattA-Official/vwmcp/security/advisories/new)**, not in a public issue.

Include what an attacker could do, the steps to reproduce, and the affected version. You'll get an acknowledgement within a week.

## Scope

In scope, for example:

- Ways for content Claude reads (a web page, a document, a library file) to make vwmcp run code or write files without the MCP client asking the user.
- Path traversal or unsafe writes in `library_install` or the job queue.
- vwmcp affecting applications other than Vectorworks (keystrokes, dialog handling).
- Downloads that bypass the `https` and file-type checks.

Out of scope: `run_python` running arbitrary code is the purpose of the tool. It's controlled by your MCP client's tool approval; see [Safety and trust](docs/explanation/safety.md).
