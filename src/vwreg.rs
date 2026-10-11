//! Registering the `vwmcp` menu command with Vectorworks.
//!
//! Vectorworks can only be scripted from inside, so the very first
//! registration goes through its own **Install Partner Products** feature:
//! we drop a package into `Plug-ins/Data/PartnerInstallerDev` (the folder
//! Vectorworks documents for locally provided packages), and installing it
//! there copies the plug-in and runs `add_to_workspace.py`, which adds the
//! menu item and its shortcut through the workspace API.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::trigger::Hotkey;

/// Unlocked script plug-in: a menu command whose script is
/// `paths::MENU_COMMAND_SCRIPT`. Unlocked script plug-ins need no developer
/// credentials, so Vectorworks loads it without the "unknown developer" prompt.
const PLUGIN: &[u8] = include_bytes!("../assets/vwmcp.vsm");
/// Listing image shown in Install Partner Products (from icons/vwmcp-installer.svg).
const LISTING_PNG: &[u8] = include_bytes!("../assets/icons/vwmcp-installer.png");
const ADD_SCRIPT: &str = include_str!("../assets/add_to_workspace.py");
const REMOVE_SCRIPT: &str = include_str!("../assets/remove_from_workspace.py");
const PRODUCT_ID: &str = "7C1E2A5B-3F44-4D8E-9B0A-6E2D51C9A4F7";
const PACKAGE: &str = "vwmcp.zip";
const MANIFEST: &str = "vwmcp.xml";

#[derive(Debug)]
pub enum VersionState {
    /// A plug-in running vwmcp's entry script is already installed.
    Registered { plugin: PathBuf },
    /// The partner package was written; the user installs it in Vectorworks.
    PackageReady,
}

#[derive(Debug)]
pub struct VersionReport {
    pub year: String,
    pub state: VersionState,
}

/// `%APPDATA%\Nemetschek\Vectorworks\<year>` folders that Vectorworks has
/// created (i.e. versions that have been run at least once), newest first.
pub fn installed_versions() -> Vec<(String, PathBuf)> {
    let Some(root) = dirs::config_dir().map(|d| d.join("Nemetschek").join("Vectorworks")) else {
        return Vec::new();
    };
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(&root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let ok = name.len() == 4
                        && name.chars().all(|c| c.is_ascii_digit())
                        && name.as_str() >= "2026"
                        && e.path().join("Plug-ins").is_dir();
                    ok.then(|| (name, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

/// A `.vsm` anywhere under `plugins` (two levels deep) whose script loads
/// vwmcp's entry point.
fn find_existing(plugins: &Path) -> Option<PathBuf> {
    fn scan(dir: &Path, depth: u8) -> Option<PathBuf> {
        for e in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                if depth < 2
                    && e.file_name() != "Data"
                    && let Some(f) = scan(&p, depth + 1)
                {
                    return Some(f);
                }
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("vsm")) {
                let data = std::fs::read(&p).unwrap_or_default();
                let hit = |needle: &[u8]| data.windows(needle.len()).any(|w| w == needle);
                if hit(b"vwmcp") && hit(b"entry.py") {
                    return Some(p);
                }
            }
        }
        None
    }
    scan(plugins, 0)
}

/// Key and modifier mask as Vectorworks' workspace API expects them.
fn workspace_shortcut(h: &Hotkey) -> Option<(char, u8)> {
    let key = h.vk as u8 as char;
    if !key.is_ascii_alphanumeric() || h.vk > 0x5A {
        return None; // function keys: not expressible as a single CHAR
    }
    let mods = (h.ctrl as u8) | ((h.shift as u8) << 1) | ((h.alt as u8) << 2);
    Some((key, mods))
}

fn manifest() -> String {
    use base64::Engine;
    let img = base64::engine::general_purpose::STANDARD.encode(LISTING_PNG);
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"<PartnerProducts>
	<Product id="{PRODUCT_ID}">
		<Subfolder>vwmcp</Subfolder>
		<Title>vwmcp (MCP bridge for Claude)</Title>
		<Category>vwmcp</Category>
		<Info>Adds the vwmcp command (Tools &gt; Utilities) and its keyboard shortcut, which let Claude and other MCP clients run scripts in Vectorworks through vwmcp.exe. Installed locally by vwmcp.exe; nothing is downloaded.</Info>
		<Image>data:image/png;base64, {img}</Image>
		<Developer>vwmcp</Developer>
		<License>MIT</License>
		<Version>{version}</Version>
		<Packages>
			<Package os="win">{PACKAGE}</Package>
		</Packages>
	</Product>
</PartnerProducts>
"#
    )
}

fn package(key: char, mods: u8) -> Result<Vec<u8>> {
    let add = ADD_SCRIPT
        .replace("{{KEY}}", &key.to_string())
        .replace("{{MODS}}", &mods.to_string());
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        // Same layout as Vectorworks' example packages: one top-level folder.
        z.add_directory("vwmcp/", opts)?;
        z.add_directory("vwmcp/Plug-ins/", opts)?;
        z.start_file("vwmcp/Plug-ins/vwmcp.vsm", opts)?;
        z.write_all(PLUGIN)?;
        z.start_file("vwmcp/add_to_workspace.py", opts)?;
        z.write_all(add.as_bytes())?;
        z.start_file("vwmcp/remove_from_workspace.py", opts)?;
        z.write_all(REMOVE_SCRIPT.as_bytes())?;
        z.finish()?;
    }
    Ok(buf.into_inner())
}

fn dev_folder(version_dir: &Path) -> PathBuf {
    version_dir
        .join("Plug-ins")
        .join("Data")
        .join("PartnerInstallerDev")
}

/// Prepare every installed Vectorworks version (2026+).
pub fn prepare(hotkey: &Hotkey) -> Result<Vec<VersionReport>> {
    let (key, mods) = workspace_shortcut(hotkey).with_context(|| {
        format!("automatic registration needs a letter or digit shortcut, not {hotkey}; use `vwmcp setup` instead")
    })?;
    let zip = package(key, mods)?;
    let xml = manifest();
    let mut out = Vec::new();
    for (year, dir) in installed_versions() {
        let plugins = dir.join("Plug-ins");
        if let Some(plugin) = find_existing(&plugins) {
            out.push(VersionReport {
                year,
                state: VersionState::Registered { plugin },
            });
            continue;
        }
        let dev = dev_folder(&dir);
        std::fs::create_dir_all(&dev).with_context(|| format!("creating {}", dev.display()))?;
        std::fs::write(dev.join(PACKAGE), &zip)?;
        std::fs::write(dev.join(MANIFEST), &xml)?;
        out.push(VersionReport {
            year,
            state: VersionState::PackageReady,
        });
    }
    Ok(out)
}

/// Remove the partner package files (not an installed plug-in: Vectorworks
/// uninstalls that itself from Install Partner Products).
pub fn remove_packages() -> usize {
    let mut n = 0;
    for (_, dir) in installed_versions() {
        let dev = dev_folder(&dir);
        for f in [PACKAGE, MANIFEST] {
            if std::fs::remove_file(dev.join(f)).is_ok() {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_carries_the_menu_script() {
        let script = crate::paths::MENU_COMMAND_SCRIPT.as_bytes();
        assert!(PLUGIN.windows(script.len()).any(|w| w == script));
    }

    #[test]
    fn shortcut_mapping() {
        assert_eq!(
            workspace_shortcut(&Hotkey::parse("ctrl+shift+b").unwrap()),
            Some(('B', 3))
        );
        assert_eq!(
            workspace_shortcut(&Hotkey::parse("ctrl+alt+7").unwrap()),
            Some(('7', 5))
        );
        assert_eq!(workspace_shortcut(&Hotkey::parse("ctrl+f9").unwrap()), None);
    }

    #[test]
    fn package_layout_and_script() {
        let bytes = package('B', 3).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = z.file_names().map(|n| n.unwrap().to_string()).collect();
        for want in [
            "vwmcp/Plug-ins/vwmcp.vsm",
            "vwmcp/add_to_workspace.py",
            "vwmcp/remove_from_workspace.py",
        ] {
            assert!(names.iter().any(|n| n == want), "{names:?}");
        }
        let mut s = String::new();
        std::io::Read::read_to_string(&mut z.by_name("vwmcp/add_to_workspace.py").unwrap(), &mut s)
            .unwrap();
        assert!(s.contains("KEY = 'B'") && s.contains("MODS = 3"));
        assert!(!s.contains("{{"));
        assert!(manifest().contains(PRODUCT_ID));
    }

    #[test]
    fn detects_existing_plugin() {
        let d = tempfile::tempdir().unwrap();
        assert!(find_existing(d.path()).is_none());
        std::fs::write(d.path().join("VW MCP.vsm"), b"").unwrap();
        assert!(find_existing(d.path()).is_none());
        std::fs::write(d.path().join("vwmcp.vsm"), PLUGIN).unwrap();
        assert!(find_existing(d.path()).is_some());
    }
}
