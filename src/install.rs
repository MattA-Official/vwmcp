//! `vwmcp install`, `setup`, `doctor`, `uninstall`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::bridge::{Bridge, BridgeConfig};
use crate::paths::{MENU_COMMAND_SCRIPT, Paths, sync_python};
use crate::trigger::{Hotkey, platform_trigger};

pub struct InstallOptions {
    pub name: String,
    pub claude_desktop: bool,
    pub claude_code: bool,
}

const EXE: &str = if cfg!(windows) { "vwmcp.exe" } else { "vwmcp" };

/// Copy this binary to a stable location so client configs survive the
/// download being moved or deleted. A running copy (an MCP client has it
/// open) cannot be overwritten on Windows but can be renamed aside.
fn install_binary(paths: &Paths) -> Result<PathBuf> {
    let current = std::env::current_exe()?;
    let target = paths.bin.join(EXE);
    std::fs::create_dir_all(&paths.bin)?;
    if same_file(&current, &target) {
        return Ok(target);
    }
    // Clear copies set aside by earlier upgrades (any still running stay).
    if let Ok(rd) = std::fs::read_dir(&paths.bin) {
        for e in rd.filter_map(|e| e.ok()) {
            if e.file_name()
                .to_string_lossy()
                .starts_with(&format!("{EXE}.old"))
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    if target.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let old = paths.bin.join(format!("{EXE}.old-{stamp}"));
        std::fs::rename(&target, &old).context("moving the previous vwmcp aside")?;
    }
    std::fs::copy(&current, &target).with_context(|| format!("copying to {}", target.display()))?;
    Ok(target)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn claude_desktop_config() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("Claude");
    dir.is_dir().then(|| dir.join("claude_desktop_config.json"))
}

/// Add (or update) our entry in Claude Desktop's config, keeping a backup.
fn register_claude_desktop(name: &str, exe: &Path) -> Result<String> {
    let Some(cfg) = claude_desktop_config() else {
        return Ok("Claude Desktop not found — skipped.".into());
    };
    let mut doc: Value = match std::fs::read_to_string(&cfg) {
        Ok(s) if !s.trim().is_empty() => serde_json::from_str(&s).with_context(|| {
            format!(
                "{} is not valid JSON; fix or remove it first",
                cfg.display()
            )
        })?,
        _ => json!({}),
    };
    if cfg.exists() {
        std::fs::copy(&cfg, cfg.with_extension("json.vwmcp-backup"))?;
    }
    let servers = doc
        .as_object_mut()
        .context("Claude Desktop config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    let others: Vec<String> = servers
        .as_object()
        .map(|m| {
            m.keys()
                .filter(|k| k.contains("vwx") && *k != name)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    servers[name] = json!({ "command": exe.to_string_lossy(), "args": ["serve"] });
    std::fs::write(&cfg, serde_json::to_string_pretty(&doc)?)?;
    let mut msg =
        format!("Claude Desktop: registered \"{name}\" (restart Claude Desktop to load it).");
    if !others.is_empty() {
        msg.push_str(&format!(
            "\n  Note: you also have {} configured. Remove it once vwmcp works, so Claude doesn't see two \
             Vectorworks servers.",
            others.join(", ")
        ));
    }
    Ok(msg)
}

fn unregister_claude_desktop(name: &str) -> Result<String> {
    let Some(cfg) = claude_desktop_config().filter(|c| c.exists()) else {
        return Ok("Claude Desktop config not found — nothing to remove.".into());
    };
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&cfg)?)?;
    let removed = doc
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.remove(name))
        .is_some();
    if removed {
        std::fs::write(&cfg, serde_json::to_string_pretty(&doc)?)?;
        Ok(format!("Claude Desktop: removed \"{name}\"."))
    } else {
        Ok(format!("Claude Desktop: no \"{name}\" entry."))
    }
}

fn claude_cli() -> Option<&'static str> {
    let candidates: &[&str] = if cfg!(windows) {
        &["claude.cmd", "claude.exe", "claude"]
    } else {
        &["claude"]
    };
    candidates.iter().copied().find(|c| {
        Command::new(c)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

fn register_claude_code(name: &str, exe: &Path) -> String {
    let Some(cli) = claude_cli() else {
        return "Claude Code CLI not found — skipped.".into();
    };
    let _ = Command::new(cli)
        .args(["mcp", "remove", "--scope", "user", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let exe = exe.to_string_lossy();
    match Command::new(cli)
        .args(["mcp", "add", "--scope", "user", name, "--", &exe, "serve"])
        .output()
    {
        Ok(o) if o.status.success() => {
            format!("Claude Code: registered \"{name}\" for all projects.")
        }
        Ok(o) => format!(
            "Claude Code: registration failed ({}). Run: claude mcp add --scope user {name} -- \"{exe}\" serve",
            String::from_utf8_lossy(&o.stderr).trim()
        ),
        Err(e) => format!("Claude Code: could not run the CLI ({e})."),
    }
}

fn unregister_claude_code(name: &str) -> String {
    match claude_cli() {
        None => "Claude Code CLI not found — nothing to remove.".into(),
        Some(cli) => match Command::new(cli)
            .args(["mcp", "remove", "--scope", "user", name])
            .output()
        {
            Ok(o) if o.status.success() => format!("Claude Code: removed \"{name}\"."),
            _ => format!("Claude Code: no \"{name}\" entry."),
        },
    }
}

fn copy_to_clipboard(text: &str) -> bool {
    if !cfg!(windows) {
        return false;
    }
    // clip.exe reads the console code page; the script is pure ASCII.
    let Ok(mut child) = Command::new("clip").stdin(Stdio::piped()).spawn() else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }
    child.wait().is_ok_and(|s| s.success())
}

pub fn manual_steps(hotkey: &Hotkey) -> String {
    let copied = copy_to_clipboard(MENU_COMMAND_SCRIPT);
    let clip_note = if copied {
        " (it's on your clipboard now)"
    } else {
        ""
    };
    format!(
        "Manual setup in Vectorworks (if Install Partner Products isn't available):

  1. Tools > Plug-ins > Plug-in Manager… > Custom Plug-ins > New…
     Type: Command, name: vwmcp. Then Edit Script…, set the language to Python,
     and paste this single line{clip_note}:

     {MENU_COMMAND_SCRIPT}

  2. Tools > Workspaces > Edit Current Workspace… > Menus.
     Drag the vwmcp command into any menu, select it there and assign the
     shortcut {hotkey}. Click OK.

  3. Check it with:  vwmcp doctor

That line never changes, so you won't need to do this again when vwmcp updates.
Menu names may differ slightly between Vectorworks versions and languages.

Already using vwx-mcp? You can instead paste the line into your existing
\"VWX Bridge Start\" command, which already has {hotkey}. vwx-mcp stops working
after that, and its palette is no longer needed.
Different shortcut? Set the VWMCP_HOTKEY environment variable (e.g. ctrl+alt+f9)."
    )
}

/// Prepare Vectorworks and say what, if anything, the user still has to do.
pub fn vectorworks_steps(hotkey: &Hotkey) -> String {
    let reports = match crate::vwreg::prepare(hotkey) {
        Ok(r) => r,
        Err(e) => {
            return format!(
                "Automatic Vectorworks setup unavailable: {e:#}\n\n{}",
                manual_steps(hotkey)
            );
        }
    };
    if reports.is_empty() {
        return format!(
            "No Vectorworks 2026 or later found for this user. Start Vectorworks once, then run \
             `vwmcp setup`.\n\n{}",
            manual_steps(hotkey)
        );
    }
    let mut out = String::from("Vectorworks:\n");
    let mut pending = Vec::new();
    for r in &reports {
        match &r.state {
            crate::vwreg::VersionState::Registered { plugin } => out.push_str(&format!(
                "  {}: already set up ({})\n",
                r.year,
                plugin.display()
            )),
            crate::vwreg::VersionState::PackageReady => {
                out.push_str(&format!(
                    "  {}: ready to install (one click, below)\n",
                    r.year
                ));
                pending.push(r.year.clone());
            }
        }
    }
    if !pending.is_empty() {
        out.push_str(&format!(
            "\nOne click in Vectorworks {}:\n  \
             Help > Install Partner Products…  →  vwmcp  →  Install\n  \
             Vectorworks restarts, and Tools > Utilities > vwmcp has {hotkey}.\n  \
             Then check with:  vwmcp doctor\n\n\
             You only do this once; updating vwmcp never needs it again.\n\
             If vwmcp isn't listed there, run `vwmcp setup --manual`.",
            pending.join(" / ")
        ));
    } else {
        out.push_str("\nNothing to do in Vectorworks. Check with:  vwmcp doctor");
    }
    out
}

pub fn install(opts: &InstallOptions) -> Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure()?;
    let exe = install_binary(&paths)?;
    let written = sync_python(&paths)?;
    println!(
        "vwmcp {} installed to {}",
        env!("CARGO_PKG_VERSION"),
        paths.home.display()
    );
    if !written.is_empty() {
        println!("  Vectorworks scripts: {}", written.join(", "));
    }
    #[cfg(windows)]
    match crate::win::set_user_path_entry(&paths.bin, true) {
        Ok(true) => {
            println!("  Added to your PATH: open a new terminal to run `vwmcp doctor` and friends.")
        }
        Ok(false) => {}
        Err(e) => println!(
            "  Could not add vwmcp to your PATH ({e:#}); run it as {}",
            exe.display()
        ),
    }
    if opts.claude_desktop {
        match register_claude_desktop(&opts.name, &exe) {
            Ok(m) => println!("  {m}"),
            Err(e) => println!("  Claude Desktop: not registered ({e:#})"),
        }
    }
    if opts.claude_code {
        println!("  {}", register_claude_code(&opts.name, &exe));
    }
    println!(
        "\nOther MCP clients: command \"{}\" with argument \"serve\".\n",
        exe.display()
    );
    let hotkey = Hotkey::from_env().map_err(anyhow::Error::msg)?;
    println!("{}", vectorworks_steps(&hotkey));
    Ok(())
}

pub fn uninstall(name: &str) -> Result<()> {
    println!("{}", unregister_claude_desktop(name)?);
    println!("{}", unregister_claude_code(name));
    crate::vwreg::remove_packages();
    let paths = Paths::resolve()?;
    #[cfg(windows)]
    let _ = crate::win::set_user_path_entry(&paths.bin, false);
    for d in [&paths.vw, &paths.cache, &paths.home.join("ipc")] {
        let _ = std::fs::remove_dir_all(d);
    }
    println!(
        "Removed vwmcp's files from {}.\nThe binary in {} is left in place (it may be running); delete that folder \
         after closing your MCP clients.\nIn Vectorworks, uninstall vwmcp in Help > Install Partner Products \
         (or delete the vwmcp command in the Plug-in Manager if you set it up by hand).",
        paths.home.display(),
        paths.bin.display()
    );
    Ok(())
}

pub async fn doctor() -> Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure()?;
    sync_python(&paths)?;
    let hotkey = Hotkey::from_env().map_err(anyhow::Error::msg)?;
    let bridge = Bridge::new(
        paths.clone(),
        Arc::from(platform_trigger(hotkey)),
        BridgeConfig::default(),
        hotkey.to_string(),
    );

    let vw = bridge.vw_status().await;
    let mut ok = true;
    if cfg!(windows) {
        if vw.running {
            println!(
                "✓ Vectorworks is running ({})",
                vw.window_title.as_deref().unwrap_or("?")
            );
        } else {
            println!("✗ Vectorworks is not running — start it and run this again.");
            return Ok(());
        }
        if !vw.open_dialogs.is_empty() {
            println!(
                "✗ A Vectorworks dialog is open: {} — close it and run this again.",
                vw.open_dialogs.join(", ")
            );
            return Ok(());
        }
    }
    match bridge.pump_last_seen() {
        Some(s) => println!("✓ The vwmcp menu command last ran {s:.0}s ago"),
        None => println!("· The vwmcp menu command has not run yet on this machine"),
    }
    print!("… Sending a test job with {hotkey} … ");
    let started = std::time::Instant::now();
    let r = bridge
        .run_with_pickup(
            "doctor",
            "import sys\n__result__ = {'python': sys.version.split()[0], 'vectorworks': vw._safe(lambda: list(vs.GetVersion())), 'document': vw._safe(vs.GetFName)}",
            json!({}),
            Duration::from_secs(10),
            Duration::from_secs(10),
        )
        .await;
    match r {
        Ok(r) if r.ok => println!(
            "✓ round trip in {} ms: {}",
            started.elapsed().as_millis(),
            r.result
        ),
        Ok(r) => {
            ok = false;
            println!("✗ the job ran but failed: {}", r.error.unwrap_or_default());
            if let Some(tb) = r.traceback {
                println!("{tb}");
            }
        }
        Err(e) => {
            ok = false;
            println!("✗ {e}");
        }
    }
    if !ok {
        println!("\n{}", vectorworks_steps(&hotkey));
        println!(
            "\nIf Vectorworks says it's set up but this still fails: Vectorworks must be restarted once after \
             installing, and the shortcut must not be assigned to another command."
        );
    }
    Ok(())
}
