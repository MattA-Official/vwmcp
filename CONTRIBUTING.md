# Contributing to vwmcp

Thanks for helping. Bug reports, Vectorworks pitfalls you've hit, docs fixes and code are all welcome.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **Report a bug.** Use the bug report form, and include the output of `vwmcp doctor` and your Vectorworks version and language.
- **Report a Vectorworks pitfall.** If a `vs.*` call silently does nothing, hangs or has changed, open an issue with a minimal script. These go into the [pitfalls reference](docs/reference/api-pitfalls.md) and the `run_python` tool description, so every agent benefits.
- **Test on your setup.** Reports from Vectorworks 2026, non-English builds and other MCP clients are especially useful (see the [roadmap](ROADMAP.md)).
- **Improve the docs.** See [Documentation](#documentation) below.
- **Write code.** Look for issues labelled `good first issue` or `help wanted`, or comment on an issue before starting something large.

## Development setup

You need [Rust](https://rustup.rs/) (stable, 1.88 or later) and Python 3.10+ for the test harness.

```sh
git clone https://github.com/MattA-Official/vwmcp
cd vwmcp
cargo test
cargo build
python3 tests/harness/e2e.py target/debug/vwmcp
```

The end-to-end test runs the real server and the real Vectorworks-side code against a fake `vs` module, so you can work on almost everything without Windows or Vectorworks.

To build the Windows binary from Linux or macOS:

```sh
rustup target add x86_64-pc-windows-gnu
# Debian/Ubuntu: sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

### Trying a build in Vectorworks

1. Close Claude Desktop.
2. Run your build's `vwmcp.exe install`. It replaces the installed copy and refreshes the Vectorworks-side scripts.
3. Run `vwmcp doctor`, then reopen Claude Desktop.

Use a scratch document. If you change the Python under `python/`, the server redeploys it on start and Vectorworks reloads it on the next job, so you don't need to restart Vectorworks.

## Project layout

| Path | |
|---|---|
| `src/main.rs` | CLI entry point |
| `src/server.rs` | MCP tools |
| `src/bridge.rs` | File-queue bridge to Vectorworks |
| `src/trigger.rs`, `src/win.rs` | Keyboard trigger, dialog handling, screenshots (Windows) |
| `src/docs.rs` | `vs.*` API reference lookup |
| `src/library.rs` | Resource library install |
| `src/install.rs`, `src/vwreg.rs` | Installer and Vectorworks registration |
| `src/paths.rs` | File layout; embeds the Python files |
| `python/` | Code that runs inside Vectorworks: entry point, job pump, `vw` helpers |
| `assets/` | The menu command plug-in, installer scripts and icon |
| `tests/harness/` | Fake Vectorworks and end-to-end test |
| `docs/` | Documentation |

### The menu command plug-in

`assets/vwmcp.vsm` is the Vectorworks menu command plug-in, written by Vectorworks 2027's Plug-in Manager (category Tools, tooltip help, created for version 31 = 2026). Its only script is the line in `src/paths.rs` (`MENU_COMMAND_SCRIPT`); a test checks they match. To change its details, edit it in **Tools › Plug-ins › Plug-in Manager** and copy the file back. Don't edit the bytes: the format is undocumented. `assets/icons/` holds the icon: `vwmcp-installer.svg` is the source of the listing image in Install Partner Products.

[How vwmcp talks to Vectorworks](docs/explanation/architecture.md) explains how the parts fit together. Read it before changing the bridge or the trigger.

## Guidelines

### Code

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass; CI checks both on Linux and Windows.
- Add or update tests. Rust unit tests sit next to the code; anything that crosses into Vectorworks belongs in `tests/harness/e2e.py` (extend `fake_vs/vs.py` as needed, matching real Vectorworks behaviour as closely as you can).
- Keep the Vectorworks side generic. A new tool is usually a few lines of Python in `src/server.rs`, plus a helper in `python/vw.py` if it's reusable. Avoid adding verbs to the pump.
- Think twice before adding a tool. The toolset is deliberately small ([why](docs/explanation/design.md#a-dozen-tools-not-hundreds)). If `run_python` plus a `vw` helper can do it, prefer that.
- Never run code in Vectorworks outside the menu command context; see the architecture doc for why.

### Documentation

Docs follow [Diátaxis](https://diataxis.fr/). Put new material where it belongs:

- **Tutorial** (`docs/tutorials/`): a lesson for newcomers.
- **How-to guide** (`docs/how-to/`): steps for a specific task.
- **Reference** (`docs/reference/`): facts, structured like the thing described.
- **Explanation** (`docs/explanation/`): background and reasons.

Write in British English, in plain language. Diagrams use Mermaid.

### Commits and pull requests

- Keep commits small and focused, one logical change each, with messages in the [Conventional Commits](https://www.conventionalcommits.org/) style: `feat(server): add …`, `fix(win): …`, `docs: …`, `ci: …`.
- Open a pull request against `main` and fill in the template. CI must pass.
- Add a line under **Unreleased** in [CHANGELOG.md](CHANGELOG.md) for anything a user would notice.

## Releasing (maintainers)

1. Move the **Unreleased** entries in `CHANGELOG.md` under a new version heading and update the links at the bottom.
2. Set the same version in `Cargo.toml`, and run `cargo build` so `Cargo.lock` updates.
3. Commit as `chore(release): vX.Y.Z`, tag `vX.Y.Z`, and push the tag.

The release workflow checks the tag matches `Cargo.toml`, runs the tests, builds `vwmcp.exe` on Windows, and publishes a GitHub release with the changelog entry and a SHA-256 checksum.
