//! Vectorworks script API reference, downloaded from Vectorworks' own public
//! GitHub repository (Vectorworks/developer-scripting) and cached locally.
//!
//! Nothing from it is bundled in this binary: the stub and pages are
//! Nemetschek's, and fetching them keeps the reference current with each SDK.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tokio::sync::Mutex;

const DEFAULT_BASE: &str = "https://raw.githubusercontent.com/Vectorworks/developer-scripting/main";
const STUB_PATH: &str = "Python/pages/files/vs.py";
const REFRESH_AFTER: Duration = Duration::from_secs(30 * 24 * 3600);
const MAX_PAGE_CHARS: usize = 16_000;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Arg {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VsFn {
    pub name: String,
    pub signature: String,
    pub args: Vec<Arg>,
    pub category: String,
    pub description: String,
}

impl VsFn {
    pub fn one_line(&self) -> String {
        let mut d = self.description.clone();
        if d.chars().count() > 140 {
            d = d.chars().take(140).collect::<String>() + "…";
        }
        format!("{}  [{}]  {}", self.signature, self.category, d)
    }
}

pub struct Docs {
    cache: PathBuf,
    base: String,
    client: reqwest::Client,
    index: Mutex<Option<Arc<Vec<VsFn>>>>,
}

impl Docs {
    pub fn new(cache: PathBuf) -> Self {
        Docs {
            cache,
            base: std::env::var("VWMCP_DOCS_BASE").unwrap_or_else(|_| DEFAULT_BASE.into()),
            client: reqwest::Client::builder()
                .user_agent(concat!("vwmcp/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("http client"),
            index: Mutex::new(None),
        }
    }

    async fn fetch(&self, rel: &str) -> Result<String> {
        let url = format!("{}/{}", self.base, rel.replace(' ', "%20"));
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("fetching {url}"))?;
        if !resp.status().is_success() {
            bail!("{url} returned {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    pub async fn index(&self) -> Result<Arc<Vec<VsFn>>> {
        let mut guard = self.index.lock().await;
        if let Some(i) = guard.as_ref() {
            return Ok(i.clone());
        }
        let path = self.cache.join("vs.py");
        let stale = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().unwrap_or_default() > REFRESH_AFTER)
            .unwrap_or(true);
        let text = if stale {
            match self.fetch(STUB_PATH).await {
                Ok(t) => {
                    std::fs::create_dir_all(&self.cache)?;
                    std::fs::write(&path, &t)?;
                    t
                }
                Err(e) => std::fs::read_to_string(&path).map_err(|_| {
                    e.context("the Vectorworks API reference is not cached yet and could not be downloaded")
                })?,
            }
        } else {
            std::fs::read_to_string(&path)?
        };
        let idx = Arc::new(parse_stub(&text));
        if idx.is_empty() {
            bail!("the downloaded vs.py stub contained no functions");
        }
        *guard = Some(idx.clone());
        Ok(idx)
    }

    pub async fn lookup(&self, query: &str, limit: usize) -> Result<Vec<VsFn>> {
        Ok(search(&self.index().await?, query, limit))
    }

    /// Full reference page (description, parameters, remarks, examples).
    pub async fn page(&self, name: &str) -> Result<String> {
        let idx = self.index().await?;
        let canonical = idx
            .iter()
            .find(|f| {
                f.name
                    .eq_ignore_ascii_case(name.trim().trim_start_matches("vs."))
            })
            .map(|f| f.name.clone())
            .with_context(|| format!("vs.{name} is not in the Vectorworks API index"))?;
        let path = self.cache.join("pages").join(format!("{canonical}.md"));
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                let t = self
                    .fetch(&format!("Function Reference/Functions/{canonical}.md"))
                    .await?;
                std::fs::create_dir_all(path.parent().unwrap())?;
                std::fs::write(&path, &t)?;
                t
            }
        };
        let cleaned: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("![")) // images are useless here
            .collect::<Vec<_>>()
            .join("\n");
        Ok(if cleaned.chars().count() > MAX_PAGE_CHARS {
            cleaned.chars().take(MAX_PAGE_CHARS).collect::<String>() + "\n\n… [page truncated]"
        } else {
            cleaned
        })
    }
}

/// Parse Vectorworks' `vs.py` help stub into function entries.
pub fn parse_stub(text: &str) -> Vec<VsFn> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let Some(rest) = line.strip_prefix("def ") else {
            i += 1;
            continue;
        };
        let Some(paren) = rest.find('(') else {
            i += 1;
            continue;
        };
        let name = rest[..paren].trim().to_string();
        // Arguments: either inline `def F(a, b):` or one per line with `# doc`.
        let mut args = Vec::new();
        let inline = &rest[paren + 1..];
        if let Some(close) = inline.find(')') {
            for a in inline[..close]
                .split(',')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                args.push(Arg {
                    name: a.to_string(),
                    doc: String::new(),
                });
            }
            i += 1;
        } else {
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                i += 1;
                if l.starts_with(')') {
                    break;
                }
                let (decl, doc) = match l.split_once('#') {
                    Some((d, c)) => (d.trim(), c.trim()),
                    None => (l, ""),
                };
                let decl = decl.trim_end_matches(',').trim();
                let decl = decl.split('=').next().unwrap_or(decl).trim();
                if !decl.is_empty() {
                    args.push(Arg {
                        name: decl.to_string(),
                        doc: doc.to_string(),
                    });
                }
            }
        }
        // Docstring.
        let (mut signature, mut category, mut description) =
            (String::new(), String::new(), Vec::new());
        while i < lines.len()
            && !lines[i].trim().starts_with("'''")
            && !lines[i].starts_with("def ")
        {
            i += 1;
        }
        if i < lines.len() && lines[i].trim().starts_with("'''") {
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("'''") {
                let l = lines[i].trim();
                if let Some(s) = l.strip_prefix("Python:") {
                    signature = s.trim().to_string();
                } else if let Some(c) = l.strip_prefix("Category:") {
                    category = c.trim().to_string();
                } else if !l.is_empty() && !l.starts_with("VectorScript:") {
                    description.push(l);
                }
                i += 1;
            }
        }
        if signature.is_empty() {
            let names: Vec<&str> = args.iter().map(|a| a.name.as_str()).collect();
            signature = format!("vs.{name}({})", names.join(", "));
        }
        out.push(VsFn {
            name,
            signature,
            args,
            category,
            description: description.join(" "),
        });
    }
    out
}

pub fn search(index: &[VsFn], query: &str, limit: usize) -> Vec<VsFn> {
    let q = query.trim().trim_start_matches("vs.").to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    let mut scored: Vec<(i64, &VsFn)> = index
        .iter()
        .filter_map(|f| {
            let name = f.name.to_lowercase();
            let hay = format!(
                "{name} {} {}",
                f.category.to_lowercase(),
                f.description.to_lowercase()
            );
            let score = if name == q {
                10_000
            } else if name.starts_with(&q) {
                5_000 - name.len() as i64
            } else if name.contains(&q) {
                3_000 - name.len() as i64
            } else if words.iter().all(|w| hay.contains(w)) {
                1_000 + words.iter().filter(|w| name.contains(*w)).count() as i64 * 100
                    - name.len() as i64
            } else {
                return None;
            };
            Some((score, f))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, f)| f.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STUB: &str = r#"
class Handle:
	def type():
		'''
			Access the type.
		'''
		pass

def GetFolderPath(
		whichPath  # INTEGER - Path constant.
		):
	'''
		Python: DYNARRAY of CHAR = vs.GetFolderPath(whichPath)
		VectorScript: FUNCTION GetFolderPath(whichPath:INTEGER) : DYNARRAY of CHAR;

		Category: File I/O
		Function GetFolderPath returns the full path to the requested folder.
	'''
	pass
	return 'DYNARRAY[] of CHAR' #


def FLayer():
	'''
		Python: HANDLE = vs.FLayer()
		Category: Layers
		Returns the first layer.
	'''
	pass

def ImportResToCurFileN(
		listID,  # LONGINT
		index,  # LONGINT
		callback  # PROCEDURE
		):
	'''
		Python: HANDLE = vs.ImportResToCurFileN(listID, index, callback)
		Category: Document List Handling
		Imports the indicated resource.
	'''
	pass
"#;

    #[test]
    fn parses_stub() {
        let idx = parse_stub(STUB);
        assert_eq!(idx.len(), 3, "{idx:#?}");
        let g = &idx[0];
        assert_eq!(g.name, "GetFolderPath");
        assert_eq!(
            g.args,
            vec![Arg {
                name: "whichPath".into(),
                doc: "INTEGER - Path constant.".into()
            }]
        );
        assert_eq!(g.category, "File I/O");
        assert_eq!(
            g.signature,
            "DYNARRAY of CHAR = vs.GetFolderPath(whichPath)"
        );
        assert!(g.description.starts_with("Function GetFolderPath"));
        assert!(idx[1].args.is_empty());
        assert_eq!(idx[2].args.len(), 3);
        assert_eq!(idx[2].args[2].name, "callback");
    }

    #[test]
    fn search_ranks_exact_then_prefix_then_words() {
        let idx = parse_stub(STUB);
        assert_eq!(search(&idx, "vs.getfolderpath", 5)[0].name, "GetFolderPath");
        assert_eq!(search(&idx, "Import", 5)[0].name, "ImportResToCurFileN");
        assert_eq!(search(&idx, "first layer", 5)[0].name, "FLayer");
        assert!(search(&idx, "nothing matches this", 5).is_empty());
    }
}
