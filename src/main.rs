//! vwmcp — an MCP server for Vectorworks.
//!
//! Run with no arguments from a terminal (or double-click) to install; MCP
//! clients launch it as `vwmcp serve` over stdio.

mod bridge;
mod docs;
mod install;
mod library;
mod paths;
mod server;
mod trigger;
mod vwreg;
#[cfg(windows)]
mod win;

use std::io::IsTerminal;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rmcp::ServiceExt;

#[derive(Parser)]
#[command(version, about = "MCP server for Vectorworks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the MCP server over stdio (what MCP clients launch).
    Serve,
    /// Install for the current user and register with Claude Desktop / Claude Code.
    Install {
        /// Name to register the server under.
        #[arg(long, default_value = "vectorworks")]
        name: String,
        #[arg(long)]
        no_claude_desktop: bool,
        #[arg(long)]
        no_claude_code: bool,
    },
    /// Prepare (or re-prepare) the Vectorworks side and show what's left to do.
    Setup {
        /// Show the manual Plug-in Manager steps instead.
        #[arg(long)]
        manual: bool,
    },
    /// Check that Vectorworks and the bridge are working.
    Doctor,
    /// Remove registrations and vwmcp's files.
    Uninstall {
        #[arg(long, default_value = "vectorworks")]
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("VWMCP_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr) // stdout carries the MCP protocol
        .with_ansi(false)
        .init();

    let interactive = std::io::stdin().is_terminal();
    let command = cli.command.unwrap_or(if interactive {
        Cmd::Install {
            name: "vectorworks".into(),
            no_claude_desktop: false,
            no_claude_code: false,
        }
    } else {
        Cmd::Serve
    });
    let pause = interactive && cfg!(windows) && launched_by_double_click();

    let rt = tokio::runtime::Runtime::new()?;
    let result = match command {
        Cmd::Serve => rt.block_on(serve()),
        Cmd::Install {
            name,
            no_claude_desktop,
            no_claude_code,
        } => install::install(&install::InstallOptions {
            name,
            claude_desktop: !no_claude_desktop,
            claude_code: !no_claude_code,
        }),
        Cmd::Setup { manual } => {
            let hk = trigger::Hotkey::from_env().map_err(anyhow::Error::msg)?;
            if manual {
                println!("{}", install::manual_steps(&hk));
            } else {
                println!("{}", install::vectorworks_steps(&hk));
            }
            Ok(())
        }
        Cmd::Doctor => rt.block_on(install::doctor()),
        Cmd::Uninstall { name } => install::uninstall(&name),
    };
    if let Err(e) = &result {
        eprintln!("error: {e:#}");
    }
    if pause {
        println!("\nPress Enter to close.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    result
}

/// True when Windows opened a fresh console just for us (Explorer double-click).
fn launched_by_double_click() -> bool {
    std::env::args().len() == 1
}

async fn serve() -> Result<()> {
    let paths = paths::Paths::resolve()?;
    paths.ensure()?;
    // Keep the Vectorworks-side scripts in step with this binary.
    let written = paths::sync_python(&paths)?;
    if !written.is_empty() {
        tracing::info!(files = ?written, "updated Vectorworks-side scripts");
    }
    let hotkey = trigger::Hotkey::from_env().map_err(anyhow::Error::msg)?;
    let bridge = Arc::new(bridge::Bridge::new(
        paths.clone(),
        Arc::from(trigger::platform_trigger(hotkey)),
        bridge::BridgeConfig::default(),
        hotkey.to_string(),
    ));
    let docs = Arc::new(docs::Docs::new(paths.cache.clone()));
    let server = server::VwServer::new(bridge, docs);
    tracing::info!(home = %paths.home.display(), %hotkey, "vwmcp serving on stdio");
    let running = server.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    Ok(())
}
