//! File-queue bridge to the pump running inside Vectorworks.
//!
//! A job is written atomically to `ipc/jobs`, the trigger nudges Vectorworks
//! until the pump claims it (renames it to `.working`), and the result is
//! read from `ipc/results/<id>.json`. An unclaimed job is withdrawn by
//! renaming it away, which races cleanly with the pump's own claiming rename:
//! exactly one of the two wins, so a job can never run after a timeout was
//! reported for it.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths::Paths;
use crate::trigger::{Fire, Trigger, VwStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub result: Value,
    #[serde(default)]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug)]
pub enum RunError {
    NotPickedUp {
        fire: Fire,
        pump_last_seen_s: Option<f64>,
        hotkey: String,
    },
    StillRunning {
        id: String,
        waited_s: u64,
    },
    Io(anyhow::Error),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NotPickedUp {
                fire,
                pump_last_seen_s,
                hotkey,
            } => {
                let seen = match pump_last_seen_s {
                    Some(s) => format!("The pump last ran {s:.0}s ago."),
                    None => "The pump has never run on this machine.".into(),
                };
                match fire {
                    Fire::NotRunning => write!(
                        f,
                        "Vectorworks is not running (no window found). Nothing was executed."
                    ),
                    Fire::DialogOpen { titles } => write!(
                        f,
                        "A Vectorworks dialog is open ({}). Close it and try again. Nothing was executed.",
                        titles.join(", ")
                    ),
                    Fire::Sent { .. } => write!(
                        f,
                        "Vectorworks received the {hotkey} shortcut but the vwmcp menu command did not run. \
                         Check that the command exists and that {hotkey} is assigned to it \
                         (run `vwmcp setup` for the steps). {seen} Nothing was executed."
                    ),
                    Fire::Unavailable { reason } => write!(
                        f,
                        "No pump picked the job up ({reason}). {seen} Nothing was executed."
                    ),
                }
            }
            RunError::StillRunning { id, waited_s } => write!(
                f,
                "The script is still running in Vectorworks after {waited_s}s. \
                 Call job_result with id '{id}' to collect the result later."
            ),
            RunError::Io(e) => write!(f, "bridge error: {e:#}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<std::io::Error> for RunError {
    fn from(e: std::io::Error) -> Self {
        RunError::Io(e.into())
    }
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// How long to keep nudging Vectorworks before giving up on a job.
    pub pickup_timeout: Duration,
    pub poll: Duration,
    pub retrigger: Duration,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        BridgeConfig {
            pickup_timeout: Duration::from_secs(15),
            poll: Duration::from_millis(25),
            retrigger: Duration::from_millis(400),
        }
    }
}

pub struct Bridge {
    paths: Paths,
    trigger: Arc<dyn Trigger>,
    cfg: BridgeConfig,
    hotkey: String,
}

pub enum JobState {
    Done(JobResult),
    Running,
    Unknown,
}

impl Bridge {
    pub fn new(paths: Paths, trigger: Arc<dyn Trigger>, cfg: BridgeConfig, hotkey: String) -> Self {
        Bridge {
            paths,
            trigger,
            cfg,
            hotkey,
        }
    }

    pub fn trigger(&self) -> &Arc<dyn Trigger> {
        &self.trigger
    }

    /// Seconds since the pump last ran, from its heartbeat file.
    pub fn pump_last_seen(&self) -> Option<f64> {
        let info: Value =
            serde_json::from_slice(&std::fs::read(&self.paths.pump_info).ok()?).ok()?;
        let last = info.get("last_drain")?.as_f64()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs_f64();
        Some((now - last).max(0.0))
    }

    pub fn pump_info(&self) -> Option<Value> {
        serde_json::from_slice(&std::fs::read(&self.paths.pump_info).ok()?).ok()
    }

    pub async fn vw_status(&self) -> VwStatus {
        let t = self.trigger.clone();
        tokio::task::spawn_blocking(move || t.status())
            .await
            .unwrap_or_default()
    }

    /// Run `code` inside Vectorworks with `params` bound to the global
    /// `params`. Waits up to `timeout` once the pump has started the job.
    pub async fn run(
        &self,
        label: &str,
        code: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<JobResult, RunError> {
        self.run_with_pickup(label, code, params, timeout, self.cfg.pickup_timeout)
            .await
    }

    /// As [`run`](Self::run), with a custom limit on how long to wait for
    /// Vectorworks to pick the job up.
    pub async fn run_with_pickup(
        &self,
        label: &str,
        code: &str,
        params: Value,
        timeout: Duration,
        pickup_timeout: Duration,
    ) -> Result<JobResult, RunError> {
        std::fs::create_dir_all(&self.paths.jobs)?;
        std::fs::create_dir_all(&self.paths.results)?;

        let id = uuid::Uuid::new_v4().simple().to_string();
        let micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        let stem = format!("{micros:020}-{id}");
        let job_path = self.paths.jobs.join(format!("{stem}.json"));
        let working = self.paths.jobs.join(format!("{stem}.working"));
        let result_path = self.paths.results.join(format!("{id}.json"));

        let job = serde_json::json!({ "id": id, "label": label, "code": code, "params": params });
        let tmp = self.paths.jobs.join(format!("{stem}.tmp"));
        std::fs::write(
            &tmp,
            serde_json::to_vec(&job).map_err(|e| RunError::Io(e.into()))?,
        )?;
        std::fs::rename(&tmp, &job_path)?;

        let start = Instant::now();
        let mut last_fire: Option<Instant> = None;
        let mut last_state = Fire::Unavailable {
            reason: "not triggered yet".into(),
        };
        let mut claimed_at: Option<Instant> = None;

        loop {
            if let Some(r) = take_result(&result_path)? {
                self.sweep_dialogs_after_job();
                return Ok(r);
            }
            if job_path.exists() {
                if last_fire.is_none_or(|t| t.elapsed() >= self.cfg.retrigger) {
                    last_fire = Some(Instant::now());
                    let t = self.trigger.clone();
                    last_state = tokio::task::spawn_blocking(move || {
                        t.dismiss_error_dialogs();
                        t.fire()
                    })
                    .await
                    .unwrap_or(Fire::Unavailable {
                        reason: "trigger thread panicked".into(),
                    });
                }
                if start.elapsed() >= pickup_timeout {
                    let withdrawn = job_path.with_extension("cancelled");
                    if std::fs::rename(&job_path, &withdrawn).is_ok() {
                        let _ = std::fs::remove_file(&withdrawn);
                        return Err(RunError::NotPickedUp {
                            fire: last_state,
                            pump_last_seen_s: self.pump_last_seen(),
                            hotkey: self.hotkey.clone(),
                        });
                    }
                    // Lost the race: the pump claimed it just now. Keep waiting.
                }
            } else if working.exists() {
                let since = *claimed_at.get_or_insert_with(Instant::now);
                if last_fire.is_none_or(|t| t.elapsed() >= self.cfg.retrigger) {
                    // A running script can raise a modal VW error dialog that
                    // would otherwise block it forever.
                    last_fire = Some(Instant::now());
                    let t = self.trigger.clone();
                    let _ = tokio::task::spawn_blocking(move || t.dismiss_error_dialogs()).await;
                }
                if since.elapsed() >= timeout {
                    return Err(RunError::StillRunning {
                        id,
                        waited_s: since.elapsed().as_secs(),
                    });
                }
            } else {
                // Neither queued nor running: the result may have landed
                // between our two checks.
                if let Some(r) = take_result(&result_path)? {
                    self.sweep_dialogs_after_job();
                    return Ok(r);
                }
                return Err(RunError::Io(anyhow::anyhow!(
                    "job {id} disappeared from the queue without a result"
                )));
            }
            tokio::time::sleep(self.cfg.poll).await;
        }
    }

    /// Vectorworks reports some engine errors (e.g. a bad callback) in a
    /// modal dialog only after the script has returned, when no job is in
    /// flight any more. Keep clearing error dialogs for a few seconds.
    fn sweep_dialogs_after_job(&self) {
        let t = self.trigger.clone();
        let jobs = self.paths.jobs.clone();
        tokio::spawn(async move {
            for _ in 0..10 {
                tokio::time::sleep(Duration::from_millis(300)).await;
                let t = t.clone();
                let _ = tokio::task::spawn_blocking(move || t.dismiss_error_dialogs()).await;
            }
            // Leave Vectorworks alone while other jobs are queued or running.
            let busy = std::fs::read_dir(&jobs)
                .map(|rd| {
                    rd.filter_map(|e| e.ok()).any(|e| {
                        let n = e.file_name();
                        let n = n.to_string_lossy();
                        n.ends_with(".json") || n.ends_with(".working")
                    })
                })
                .unwrap_or(false);
            if !busy {
                let _ = tokio::task::spawn_blocking(move || t.settle()).await;
            }
        });
    }

    /// Collect the result of a job that outlived its call's timeout.
    pub fn job_state(&self, id: &str) -> Result<JobState, RunError> {
        if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.is_empty() {
            return Ok(JobState::Unknown);
        }
        if let Some(r) = take_result(&self.paths.results.join(format!("{id}.json")))? {
            return Ok(JobState::Done(r));
        }
        let running = std::fs::read_dir(&self.paths.jobs)?
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains(id));
        Ok(if running {
            JobState::Running
        } else {
            JobState::Unknown
        })
    }
}

fn take_result(path: &Path) -> Result<Option<JobResult>, RunError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let r: JobResult = serde_json::from_slice(&bytes)
        .map_err(|e| RunError::Io(anyhow::anyhow!("unreadable result {}: {e}", path.display())))?;
    let _ = std::fs::remove_file(path);
    Ok(Some(r))
}
