//! Filesystem layout shared with the Python side (see python/vwmcp_pump.py).
//!
//! Everything lives under one version-independent folder, by default
//! `%LOCALAPPDATA%\vwmcp`, so a Vectorworks upgrade never strands the bridge:
//!
//! ```text
//! vwmcp/
//!   bin/vwmcp.exe       installed copy of this binary
//!   vw/                 Python executed inside Vectorworks (deployed by us)
//!   ipc/jobs, ipc/results, ipc/pump.json
//!   cache/              downloaded Vectorworks API docs
//! ```

use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Paths {
    pub home: PathBuf,
    pub vw: PathBuf,
    pub jobs: PathBuf,
    pub results: PathBuf,
    pub pump_info: PathBuf,
    pub cache: PathBuf,
    pub bin: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self> {
        let home = match std::env::var_os("VWMCP_HOME") {
            Some(h) if !h.is_empty() => PathBuf::from(h),
            _ => dirs::data_local_dir()
                .context("cannot locate the local app-data folder; set VWMCP_HOME")?
                .join("vwmcp"),
        };
        Ok(Self::at(home))
    }

    pub fn at(home: PathBuf) -> Self {
        let ipc = home.join("ipc");
        Paths {
            vw: home.join("vw"),
            jobs: ipc.join("jobs"),
            results: ipc.join("results"),
            pump_info: ipc.join("pump.json"),
            cache: home.join("cache"),
            bin: home.join("bin"),
            home,
        }
    }

    pub fn ensure(&self) -> Result<()> {
        for d in [&self.vw, &self.jobs, &self.results, &self.cache] {
            std::fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
        }
        Ok(())
    }
}

/// Python files that run inside Vectorworks, embedded in the binary.
pub const PYTHON_FILES: &[(&str, &str)] = &[
    ("entry.py", include_str!("../python/entry.py")),
    ("vwmcp_pump.py", include_str!("../python/vwmcp_pump.py")),
    ("vw.py", include_str!("../python/vw.py")),
];

/// The script pasted once into the Vectorworks menu command. It only execs
/// `entry.py`, so it never needs changing when vwmcp is upgraded.
pub const MENU_COMMAND_SCRIPT: &str = "import os;exec(open(os.path.join(os.environ.get('VWMCP_HOME') or os.path.join(os.environ['LOCALAPPDATA'],'vwmcp'),'vw','entry.py'),encoding='utf-8').read())";

/// Write the embedded Python files, touching only those whose content
/// changed (the pump hot-reloads on mtime). Returns the files written.
pub fn sync_python(paths: &Paths) -> Result<Vec<&'static str>> {
    std::fs::create_dir_all(&paths.vw)?;
    let mut written = Vec::new();
    for (name, content) in PYTHON_FILES {
        let target = paths.vw.join(name);
        let current = std::fs::read_to_string(&target).ok();
        if current.as_deref() != Some(*content) {
            let tmp = target.with_extension("py.tmp");
            std::fs::write(&tmp, content)?;
            std::fs::rename(&tmp, &target)
                .with_context(|| format!("writing {}", target.display()))?;
            written.push(*name);
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path().to_path_buf());
        p.ensure().unwrap();
        assert_eq!(sync_python(&p).unwrap().len(), PYTHON_FILES.len());
        assert!(sync_python(&p).unwrap().is_empty());
    }
}
