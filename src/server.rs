//! The MCP tool surface: a small, code-first set of tools.
//!
//! `run_python` does the real work; the rest give the agent what it needs to
//! write correct scripts first time (API lookup, document state, a picture of
//! the window) plus resource-library management.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData as McpError, ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::bridge::{Bridge, JobResult, JobState, RunError};
use crate::docs::Docs;
use crate::library;

const INSTRUCTIONS: &str = "\
Controls a live Vectorworks session on this computer.

Workflow: call `status` if unsure the bridge works. Inspect with \
`document_overview` / `query_objects`, look up any vs.* function with \
`vs_lookup` (and `vs_docs` for remarks/examples) BEFORE using it in \
`run_python`, then check visual results with `screenshot`. Each \
`run_python` call is one undo step in Vectorworks. Resource libraries \
(.vwx files of symbols, textures, styles…): `library_list`, \
`library_browse`, `library_install`, `library_import`.";

const RUN_PYTHON_DESCRIPTION: &str = "\
Run Python inside Vectorworks (main thread, full document access; one undo step per call).

Globals: `vs` (the Vectorworks API), `params` (dict you pass), `vw` (helpers).
Return data by assigning `__result__` (JSON-able; handles become their UUID). `print()` output is returned too.
Objects are addressed by UUID: vw.uuid(h), vw.obj(uuid).

vw helpers: uuid, obj, named(name), last(), find(criteria, limit), info(h), query(criteria, limit), \
overview(), layers(), classes(), circle(cx,cy,r), arc(cx,cy,r,start,sweep), resources(type, path, filter), \
library_folders(), library_files(), import_resources(path, names, type, on_conflict).

Vectorworks 2026 traps (they fail silently):
- vs.ArcByCenter draws nothing: use vw.circle or vs.Oval(l, t, r, b).
- vs.Arc's last argument is the SWEEP angle, not the end angle.
- vs.GetObject(name) returns a type-0 dummy for unknown names: use vw.named.
- Never loop `while h: h = vs.NextObj(h)` (can hang VW): use vw.find / vs.ForEachObject.
- Don't create/delete/move objects inside a ForEachObject callback: collect first (vw.find does).
- vs.SetFillFore / vs.SetPenFore take ONE (r, g, b) tuple, not three numbers.
- LNewObj() can be wrong after ops that replace objects: name objects (vs.SetName) and look them up.
- Renamed/removed: CreateExtrude→HExtrude(h, zBottom, zTop); SetClassVisibility→ShowClass/HideClass; \
SymbolCreate→BeginSym/EndSym; HMirror→MirrorN; SetLName→SetName; GetVWVersion→GetVersion; \
GetNumberOfComponents returns (ok, count); class names via ClassNum()+ClassList(i).
- Use the …N variants of old marker/pen/wall calls (SetLSN, InsertNewComponentN).
- Callbacks passed to vs.* must declare exactly the parameters VW expects (no `lambda *a`): \
ForEachObject → def f(h); ImportResToCurFileN → def f(name): return 0 skip / 1 replace. vw.import_resources does this for you.
- Export/import menu commands open modal dialogs; avoid them in unattended scripts.
- Setting wall prefs with SetPrefReal(85)/SetPref(68) crashes VW: use SetWallWidth→Wall→SetWallThickness/SetWallHeights.
Look up exact signatures with vs_lookup before calling unfamiliar functions: a wrong argument count raises a modal error.";

#[derive(Clone)]
pub struct VwServer {
    bridge: Arc<Bridge>,
    docs: Arc<Docs>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RunPythonArgs {
    /// Python source to execute inside Vectorworks.
    pub code: String,
    /// Values made available to the script as the `params` dict.
    #[serde(default)]
    pub params: Option<Map<String, Value>>,
    /// Seconds to wait once the script has started (default 120).
    #[serde(default)]
    pub timeout_s: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct LookupArgs {
    /// A function name (e.g. "Oval", "vs.GetBBox") or words to search for (e.g. "wall height").
    pub query: String,
    /// Maximum results (default 15).
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct DocsArgs {
    /// Exact function name, e.g. "ImportResToCurFileN".
    pub name: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct OverviewArgs {
    /// Count objects per layer (slower on very large files). Default true.
    #[serde(default)]
    pub count_objects: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct QueryArgs {
    /// Vectorworks criteria string, e.g. "(T=RECT)", "(C='Walls') & (L='Ground Floor')", "(SEL=TRUE)",
    /// "((R IN ['Part Info']))". Use "(ALL)" for everything.
    pub criteria: String,
    /// Maximum objects to describe (default 200; the total is always reported).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Also report whether each object is selected.
    #[serde(default)]
    pub include_selection: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ScreenshotArgs {
    /// Maximum image width in pixels (default 1600).
    #[serde(default)]
    pub max_width: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct JobArgs {
    /// The job id from a run_python call that was still running.
    pub id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct BrowseArgs {
    /// Full path to a saved .vwx library file (see library_list).
    pub file: String,
    /// Resource type: symbol, texture, hatch, material, record, worksheet, line_type, text_style,
    /// image_fill, gradient, wall_style, folder, or a Vectorworks type number. Default symbol.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Case-insensitive substring filter on names.
    #[serde(default)]
    pub filter: Option<String>,
    /// Maximum names to return (default 500).
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ImportArgs {
    /// Full path to a saved .vwx library file.
    pub file: String,
    /// Exact resource names to import (see library_browse).
    pub names: Vec<String>,
    /// Resource type (as for library_browse). Default symbol.
    #[serde(default)]
    pub r#type: Option<String>,
    /// When the document already has a resource of that name: "skip" (default), "replace",
    /// or "rename" (keep both; the import gets a -2 suffix).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct InstallArgs {
    /// Local path or https:// URL of a .vwx resource library. Only use sources you trust:
    /// library files can contain scripts.
    pub source: String,
    /// "libraries" (default, the user Libraries folder) or "favorites".
    #[serde(default)]
    pub destination: Option<String>,
    /// Optional relative subfolder to create inside the destination, e.g. "Lighting".
    #[serde(default)]
    pub subfolder: Option<String>,
    /// File name to save as (default: taken from the source). Must end in .vwx.
    #[serde(default)]
    pub filename: Option<String>,
    /// Replace an existing file of the same name. Default false.
    #[serde(default)]
    pub overwrite: Option<bool>,
}

fn json_text(v: &Value) -> ContentBlock {
    ContentBlock::text(serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
}

fn fail(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg.into())]))
}

/// Present a job result: data on success, error + traceback otherwise.
fn job_to_result(r: JobResult) -> CallToolResult {
    let mut v = json!({ "ok": r.ok, "elapsed_ms": r.elapsed_ms });
    if r.ok {
        v["result"] = r.result;
    } else {
        v["error"] = json!(r.error);
        if let Some(tb) = r.traceback {
            v["traceback"] = json!(tb);
        }
    }
    if !r.stdout.is_empty() {
        v["stdout"] = json!(r.stdout);
    }
    if r.ok {
        CallToolResult::success(vec![json_text(&v)])
    } else {
        CallToolResult::error(vec![json_text(&v)])
    }
}

impl VwServer {
    pub fn new(bridge: Arc<Bridge>, docs: Arc<Docs>) -> Self {
        VwServer {
            bridge,
            docs,
            tool_router: Self::tool_router(),
        }
    }

    /// Run a built-in snippet and return its `__result__` as tool output.
    async fn snippet(
        &self,
        label: &str,
        code: &str,
        params: Value,
    ) -> Result<CallToolResult, McpError> {
        match self
            .bridge
            .run(label, code, params, Duration::from_secs(120))
            .await
        {
            Ok(r) => Ok(job_to_result(r)),
            Err(e) => fail(e.to_string()),
        }
    }

    /// Run a snippet and hand back its value (for tools that post-process).
    async fn snippet_value(&self, label: &str, code: &str, params: Value) -> Result<Value, String> {
        match self
            .bridge
            .run(label, code, params, Duration::from_secs(60))
            .await
        {
            Ok(r) if r.ok => Ok(r.result),
            Ok(r) => Err(r.error.unwrap_or_else(|| "script failed".into())),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[tool_router]
impl VwServer {
    #[tool(
        description = "Check the whole chain: is Vectorworks running, is a dialog blocking it, and does a test \
                       job round-trip through the vwmcp menu command. Reports versions and timings.",
        annotations(read_only_hint = true)
    )]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        let vw = self.bridge.vw_status().await;
        let mut v = json!({
            "server_version": env!("CARGO_PKG_VERSION"),
            "vectorworks": vw,
            "pump": self.bridge.pump_info(),
            "pump_last_ran_s_ago": self.bridge.pump_last_seen().map(|s| s.round()),
        });
        let can_ping = !cfg!(windows) || (vw.running && vw.open_dialogs.is_empty());
        if can_ping {
            let started = std::time::Instant::now();
            let ping = self
                .bridge
                .run_with_pickup(
                    "status",
                    "import sys\n__result__ = {'python': sys.version.split()[0], 'document': vw._safe(vs.GetFName)}",
                    json!({}),
                    Duration::from_secs(10),
                    Duration::from_secs(8),
                )
                .await;
            v["round_trip"] = match ping {
                Ok(r) if r.ok => {
                    json!({ "ok": true, "ms": started.elapsed().as_millis(), "inside_vw": r.result })
                }
                Ok(r) => json!({ "ok": false, "error": r.error, "traceback": r.traceback }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            };
        } else if !vw.running {
            v["round_trip"] = json!({ "ok": false, "error": "Vectorworks is not running" });
        } else {
            v["round_trip"] =
                json!({ "ok": false, "error": "a Vectorworks dialog is open; close it first" });
        }
        Ok(CallToolResult::success(vec![json_text(&v)]))
    }

    #[tool(description = RUN_PYTHON_DESCRIPTION)]
    async fn run_python(
        &self,
        Parameters(a): Parameters<RunPythonArgs>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(a.timeout_s.unwrap_or(120).clamp(1, 3600));
        let params = Value::Object(a.params.unwrap_or_default());
        match self
            .bridge
            .run("run_python", &a.code, params, timeout)
            .await
        {
            Ok(r) => Ok(job_to_result(r)),
            Err(e @ RunError::StillRunning { .. }) => fail(e.to_string()),
            Err(e) => fail(e.to_string()),
        }
    }

    #[tool(
        description = "Collect the result of a run_python job that outlived its timeout.",
        annotations(read_only_hint = true)
    )]
    async fn job_result(
        &self,
        Parameters(a): Parameters<JobArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.bridge.job_state(&a.id) {
            Ok(JobState::Done(r)) => Ok(job_to_result(r)),
            Ok(JobState::Running) => fail(format!("job {} is still running", a.id)),
            Ok(JobState::Unknown) => fail(format!(
                "no job {} (already collected, or unknown id)",
                a.id
            )),
            Err(e) => fail(e.to_string()),
        }
    }

    #[tool(
        description = "Look up Vectorworks script functions (vs.*): exact signature, arguments, category and summary. \
                       Give a name for one function or words to search. Works without Vectorworks running.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn vs_lookup(
        &self,
        Parameters(a): Parameters<LookupArgs>,
    ) -> Result<CallToolResult, McpError> {
        let limit = a.limit.unwrap_or(15).clamp(1, 100);
        match self.docs.lookup(&a.query, limit).await {
            Err(e) => fail(format!("{e:#}")),
            Ok(found) if found.is_empty() => {
                fail(format!("no vs.* function matches '{}'", a.query))
            }
            Ok(found) => {
                let exact = found[0]
                    .name
                    .eq_ignore_ascii_case(a.query.trim().trim_start_matches("vs."));
                let text = if exact {
                    let f = &found[0];
                    let mut s = format!(
                        "{}\nCategory: {}\n{}\n",
                        f.signature, f.category, f.description
                    );
                    if !f.args.is_empty() {
                        s.push_str("\nArguments:\n");
                        for arg in &f.args {
                            s.push_str(&format!("  {}  {}\n", arg.name, arg.doc));
                        }
                    }
                    if found.len() > 1 {
                        s.push_str("\nRelated:\n");
                        for f in &found[1..found.len().min(6)] {
                            s.push_str(&format!("  {}\n", f.one_line()));
                        }
                    }
                    s.push_str("\nFor remarks and examples: vs_docs.");
                    s
                } else {
                    found
                        .iter()
                        .map(|f| f.one_line())
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
            }
        }
    }

    #[tool(
        description = "Full Vectorworks reference page for one vs.* function: parameters, remarks (often the \
                       undocumented gotchas) and examples. Downloaded from Vectorworks' developer docs and cached.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn vs_docs(
        &self,
        Parameters(a): Parameters<DocsArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.docs.page(&a.name).await {
            Ok(t) => Ok(CallToolResult::success(vec![ContentBlock::text(t)])),
            Err(e) => fail(format!("{e:#}")),
        }
    }

    #[tool(
        description = "Summary of the active Vectorworks document: file, active layer/class, layers (design/sheet) \
                       with object counts, class count, selection size.",
        annotations(read_only_hint = true)
    )]
    async fn document_overview(
        &self,
        Parameters(a): Parameters<OverviewArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.snippet(
            "document_overview",
            "__result__ = vw.overview(params['count_objects'])",
            json!({ "count_objects": a.count_objects.unwrap_or(true) }),
        )
        .await
    }

    #[tool(
        description = "Find objects with a Vectorworks criteria string and describe them (UUID id, type, name, class, \
                       layer, bounding box, symbol/plug-in name).",
        annotations(read_only_hint = true)
    )]
    async fn query_objects(
        &self,
        Parameters(a): Parameters<QueryArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.snippet(
            "query_objects",
            "__result__ = vw.query(params['criteria'], params['limit'], params['extra'])",
            json!({
                "criteria": a.criteria,
                "limit": a.limit.unwrap_or(200).clamp(1, 5000),
                "extra": a.include_selection.unwrap_or(false),
            }),
        )
        .await
    }

    #[tool(
        description = "Capture the Vectorworks window as an image, to check visual results. Works while Vectorworks \
                       is in the background (not when minimised).",
        annotations(read_only_hint = true)
    )]
    async fn screenshot(
        &self,
        Parameters(a): Parameters<ScreenshotArgs>,
    ) -> Result<CallToolResult, McpError> {
        let max_w = a.max_width.unwrap_or(1600).clamp(256, 4096);
        let t = self.bridge.trigger().clone();
        match tokio::task::spawn_blocking(move || t.screenshot(max_w)).await {
            Ok(Ok(png)) => Ok(CallToolResult::success(vec![ContentBlock::image(
                base64::engine::general_purpose::STANDARD.encode(png),
                "image/png",
            )])),
            Ok(Err(e)) => fail(format!("{e:#}")),
            Err(e) => fail(format!("screenshot failed: {e}")),
        }
    }

    #[tool(
        description = "List resource library locations and the .vwx library files in the user Libraries and \
                       Favorites folders.",
        annotations(read_only_hint = true)
    )]
    async fn library_list(&self) -> Result<CallToolResult, McpError> {
        self.snippet(
            "library_list",
            "__result__ = {'folders': vw.library_folders(), 'files': vw.library_files()}",
            json!({}),
        )
        .await
    }

    #[tool(
        description = "List the resources of one type inside a .vwx library file (or any saved .vwx), without \
                       opening it.",
        annotations(read_only_hint = true)
    )]
    async fn library_browse(
        &self,
        Parameters(a): Parameters<BrowseArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.snippet(
            "library_browse",
            "names = vw.resources(params['type'], params['file'], params['filter'])\n\
             __result__ = {'file': params['file'], 'type': vw.resource_type(params['type']), \
             'total': len(names), 'names': names[:params['limit']]}",
            json!({
                "file": a.file,
                "type": a.r#type.unwrap_or_else(|| "symbol".into()),
                "filter": a.filter,
                "limit": a.limit.unwrap_or(500).clamp(1, 10_000),
            }),
        )
        .await
    }

    #[tool(
        description = "Import named resources from a .vwx library file into the active document.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn library_import(
        &self,
        Parameters(a): Parameters<ImportArgs>,
    ) -> Result<CallToolResult, McpError> {
        if a.names.is_empty() {
            return fail("give at least one resource name (see library_browse)");
        }
        self.snippet(
            "library_import",
            "__result__ = vw.import_resources(params['file'], params['names'], params['type'], params['on_conflict'])",
            json!({
                "file": a.file,
                "names": a.names,
                "type": a.r#type.unwrap_or_else(|| "symbol".into()),
                "on_conflict": a.on_conflict.unwrap_or_else(|| "skip".into()),
            }),
        )
        .await
    }

    #[tool(
        description = "Install a .vwx resource library (local file or https URL) into the user Libraries or \
                       Favorites folder so it appears in the Resource Manager.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
    )]
    async fn library_install(
        &self,
        Parameters(a): Parameters<InstallArgs>,
    ) -> Result<CallToolResult, McpError> {
        let dest_kind = a
            .destination
            .as_deref()
            .unwrap_or("libraries")
            .to_ascii_lowercase();
        let key = match dest_kind.as_str() {
            "libraries" | "library" => "user_libraries",
            "favorites" | "favourites" | "favorite" | "favourite" => "user_favorites",
            other => {
                return fail(format!(
                    "destination must be 'libraries' or 'favorites', not '{other}'"
                ));
            }
        };
        let folders = match self
            .snippet_value(
                "library_folders",
                "__result__ = vw.library_folders()",
                json!({}),
            )
            .await
        {
            Ok(v) => v,
            Err(e) if key == "user_libraries" => match library::fallback_user_libraries() {
                Some(p) => json!({ "user_libraries": p }),
                None => {
                    return fail(format!(
                        "could not ask Vectorworks for its library folder: {e}"
                    ));
                }
            },
            Err(e) => {
                return fail(format!(
                    "could not ask Vectorworks for its Favorites folder: {e}"
                ));
            }
        };
        let Some(root) = folders
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return fail(format!("Vectorworks reported no {key} folder"));
        };
        let req = library::InstallRequest {
            source: a.source,
            root: root.into(),
            subfolder: a.subfolder,
            filename: a.filename,
            overwrite: a.overwrite.unwrap_or(false),
        };
        match library::install(req).await {
            Ok(done) => Ok(CallToolResult::success(vec![json_text(&json!({
                "installed": done.path,
                "bytes": done.bytes,
                "note": "Refresh the Resource Manager (or reopen it) to see the library. \
                         Use library_browse on this path to list what it contains.",
            }))])),
            Err(e) => fail(format!("{e:#}")),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VwServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vwmcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
