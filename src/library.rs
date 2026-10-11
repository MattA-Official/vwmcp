//! Installing .vwx resource libraries into the user's library folders.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

const MAX_DOWNLOAD_BYTES: u64 = 4 * 1024 * 1024 * 1024;

pub struct InstallRequest {
    pub source: String,
    pub root: PathBuf,
    pub subfolder: Option<String>,
    pub filename: Option<String>,
    pub overwrite: bool,
}

pub struct Installed {
    pub path: String,
    pub bytes: u64,
}

/// `%APPDATA%\Nemetschek\Vectorworks\<newest year>\Libraries`, used only when
/// Vectorworks itself cannot be asked.
pub fn fallback_user_libraries() -> Option<String> {
    let root = dirs::config_dir()?.join("Nemetschek").join("Vectorworks");
    let newest = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (n.len() == 4 && n.chars().all(|c| c.is_ascii_digit())).then_some(n)
        })
        .max()?;
    Some(
        root.join(newest)
            .join("Libraries")
            .to_string_lossy()
            .into_owned(),
    )
}

fn is_url(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

/// A relative subfolder with no `..`, root or drive components.
fn safe_subfolder(s: &str) -> Result<PathBuf> {
    let p = Path::new(s);
    if p.components().all(|c| matches!(c, Component::Normal(_))) && p.components().next().is_some()
    {
        Ok(p.to_path_buf())
    } else {
        bail!("subfolder must be a plain relative path like 'Lighting' or 'Stage/Trusses'")
    }
}

fn valid_filename(name: &str) -> Result<String> {
    let name = name.trim();
    let bad = name.is_empty()
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || name.starts_with('.')
        || !name.to_ascii_lowercase().ends_with(".vwx");
    if bad {
        bail!(
            "'{name}' is not a usable library file name (it must be a plain name ending in .vwx)"
        );
    }
    Ok(name.to_string())
}

fn name_from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let last = path.rsplit('/').next()?;
    Some(percent_decode(last))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reject obvious non-library downloads (an HTML page instead of the file).
fn looks_like_html(head: &[u8]) -> bool {
    let start = String::from_utf8_lossy(&head[..head.len().min(256)])
        .trim_start()
        .to_ascii_lowercase();
    start.starts_with("<!doctype html") || start.starts_with("<html")
}

pub async fn install(req: InstallRequest) -> Result<Installed> {
    let mut dir = req.root.clone();
    if let Some(sub) = req.subfolder.as_deref().filter(|s| !s.trim().is_empty()) {
        dir.push(safe_subfolder(sub.trim())?);
    }
    let source_name = if is_url(&req.source) {
        name_from_url(&req.source)
    } else {
        Path::new(&req.source)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
    };
    let filename = valid_filename(
        req.filename
            .as_deref()
            .or(source_name.as_deref())
            .context("cannot tell the file name from the source; pass `filename`")?,
    )?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let target = dir.join(&filename);
    if target.exists() && !req.overwrite {
        bail!(
            "{} already exists; pass overwrite=true to replace it",
            target.display()
        );
    }
    let part = dir.join(format!(".{filename}.part"));

    let bytes = if is_url(&req.source) {
        if req.source.starts_with("http://") {
            bail!("only https:// downloads are allowed");
        }
        download(&req.source, &part).await?
    } else {
        let src = Path::new(&req.source);
        if !src.is_file() {
            bail!("{} is not a file", src.display());
        }
        let head = std::fs::read(src).map(|b| b.into_iter().take(256).collect::<Vec<u8>>())?;
        if looks_like_html(&head) {
            bail!("{} is an HTML page, not a Vectorworks file", src.display());
        }
        std::fs::copy(src, &part).with_context(|| format!("copying {}", src.display()))?
    };
    std::fs::rename(&part, &target).with_context(|| format!("saving {}", target.display()))?;
    Ok(Installed {
        path: target.to_string_lossy().into_owned(),
        bytes,
    })
}

async fn download(url: &str, part: &Path) -> Result<u64> {
    use tokio::io::AsyncWriteExt;
    let client = reqwest::Client::builder()
        .user_agent(concat!("vwmcp/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let mut resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("downloading {url}"))?;
    if !resp.status().is_success() {
        bail!("{url} returned {}", resp.status());
    }
    if resp
        .content_length()
        .is_some_and(|n| n > MAX_DOWNLOAD_BYTES)
    {
        bail!("the download is larger than 4 GB");
    }
    let mut file = tokio::fs::File::create(part).await?;
    let mut total = 0u64;
    let mut first = true;
    let result: Result<()> = async {
        while let Some(chunk) = resp.chunk().await? {
            if first {
                first = false;
                if looks_like_html(&chunk) {
                    bail!("the URL returned a web page, not a .vwx file (use the direct download link)");
                }
            }
            total += chunk.len() as u64;
            if total > MAX_DOWNLOAD_BYTES {
                bail!("the download is larger than 4 GB");
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }
    .await;
    if let Err(e) = result {
        drop(file);
        let _ = tokio::fs::remove_file(part).await;
        return Err(e);
    }
    if total == 0 {
        let _ = tokio::fs::remove_file(part).await;
        bail!("the download was empty");
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_subfolders_and_names() {
        assert!(safe_subfolder("Lighting/Trusses").is_ok());
        assert!(safe_subfolder("../x").is_err());
        assert!(safe_subfolder("/abs").is_err());
        assert!(valid_filename("Rig.vwx").is_ok());
        assert!(valid_filename("Rig.VWX").is_ok());
        assert!(valid_filename("rig.exe").is_err());
        assert!(valid_filename("a/b.vwx").is_err());
        assert!(valid_filename("..vwx").is_err());
    }

    #[test]
    fn names_from_urls() {
        assert_eq!(
            name_from_url("https://x.org/f/My%20Lib.vwx?dl=1").unwrap(),
            "My Lib.vwx"
        );
        assert_eq!(percent_decode("a%2"), "a%2");
        assert!(looks_like_html(b"  <!DOCTYPE html><html>"));
        assert!(!looks_like_html(b"\x00\x01binary"));
    }

    #[tokio::test]
    async fn installs_local_file_and_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Truss Library.vwx");
        std::fs::write(&src, b"\x00fake vwx").unwrap();
        let root = dir.path().join("Libraries");
        let req = |overwrite| InstallRequest {
            source: src.to_string_lossy().into(),
            root: root.clone(),
            subfolder: Some("Stage".into()),
            filename: None,
            overwrite,
        };
        let done = install(req(false)).await.unwrap();
        assert!(done.path.ends_with("Truss Library.vwx"));
        assert_eq!(done.bytes, 9);
        assert!(install(req(false)).await.is_err());
        assert!(install(req(true)).await.is_ok());
    }
}
