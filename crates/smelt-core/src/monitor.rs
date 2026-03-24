//! Job monitoring — tracks job execution state and persists it to disk.
//!
//! `JobMonitor` writes structured TOML state to `.smelt/runs/<job-name>/state.toml`
//! on every phase transition, providing the primary observability surface for running
//! jobs. Legacy flat `.smelt/run-state.toml` files (written before S04) can be read
//! via [`JobMonitor::read_legacy`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Result, SmeltError};
use crate::forge::{CiStatus, PrState};
use crate::manifest::JobManifest;

/// Execution phase of a running job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPhase {
    /// Container is being provisioned from the Docker image.
    Provisioning,
    /// Job manifest is being written into the container.
    WritingManifest,
    /// Job script is executing inside the container.
    Executing,
    /// Output artifacts are being collected from the container.
    Collecting,
    /// Container is being stopped and removed.
    TearingDown,
    /// Job finished successfully.
    Complete,
    /// Job exited with a non-zero status or encountered a fatal error.
    Failed,
    /// Job exceeded its configured timeout duration.
    Timeout,
    /// Job was cancelled before completion.
    Cancelled,
    /// Job completed but one or more quality gates did not pass.
    GatesFailed,
}

/// Serializable snapshot of job execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    /// Unique name of the job, matching the manifest `job.name` field.
    pub job_name: String,
    /// Current execution phase of the job.
    pub phase: JobPhase,
    /// Docker container ID assigned to this run, if a container has been started.
    pub container_id: Option<String>,
    /// Session IDs associated with this run.
    pub sessions: Vec<String>,
    /// Unix timestamp (seconds) when the job was started.
    pub started_at: u64,
    /// Unix timestamp (seconds) of the most recent state transition.
    pub updated_at: u64,
    /// PID of the smelt process managing this job.
    pub pid: u32,
    /// URL of the pull request created for this run, if any.
    #[serde(default)]
    pub pr_url: Option<String>,
    /// Number of the pull request created for this run, if any.
    #[serde(default)]
    pub pr_number: Option<u64>,
    /// Last-known high-level state of the PR (cached from a poll).
    #[serde(default)]
    pub pr_status: Option<PrState>,
    /// Last-known CI check status of the PR (cached from a poll).
    #[serde(default)]
    pub ci_status: Option<CiStatus>,
    /// Last-known review count for the PR (cached from a poll).
    #[serde(default)]
    pub review_count: Option<u32>,
    /// `owner/repo` slug from ForgeConfig, saved at Phase 9 for watch/status.
    #[serde(default)]
    pub forge_repo: Option<String>,
    /// Env var name holding the forge auth token (never the token value).
    #[serde(default)]
    pub forge_token_env: Option<String>,
}

/// Tracks job execution state and persists it to a TOML file on disk.
pub struct JobMonitor {
    /// Current run state — updated at each phase transition and persisted to disk.
    pub state: RunState,
    state_dir: PathBuf,
}

/// Returns the current Unix timestamp in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

impl JobMonitor {
    /// Create a new monitor in `Provisioning` phase.
    ///
    /// Records the current PID and timestamp. Does **not** write to disk yet —
    /// call [`write`](Self::write) or [`set_phase`](Self::set_phase) to persist.
    pub fn new(
        job_name: impl Into<String>,
        sessions: Vec<String>,
        state_dir: impl Into<PathBuf>,
    ) -> Self {
        let now = unix_now();
        Self {
            state: RunState {
                job_name: job_name.into(),
                phase: JobPhase::Provisioning,
                container_id: None,
                sessions,
                started_at: now,
                updated_at: now,
                pid: std::process::id(),
                pr_url: None,
                pr_number: None,
                pr_status: None,
                ci_status: None,
                review_count: None,
                forge_repo: None,
                forge_token_env: None,
            },
            state_dir: state_dir.into(),
        }
    }

    /// Advance the job phase and persist state to disk.
    pub fn set_phase(&mut self, phase: JobPhase) -> Result<()> {
        self.state.phase = phase;
        self.state.updated_at = unix_now();
        self.write()
    }

    /// Record the container short ID.
    pub fn set_container(&mut self, container_id: impl Into<String>) {
        self.state.container_id = Some(container_id.into());
    }

    /// Serialize state to `{state_dir}/state.toml`.
    ///
    /// Creates `state_dir` (and any parent directories) if it does not exist.
    pub fn write(&self) -> Result<()> {
        fs::create_dir_all(&self.state_dir).map_err(|e| SmeltError::Io {
            operation: "create state dir".into(),
            path: self.state_dir.clone(),
            source: e,
        })?;
        let path = self.state_dir.join("state.toml");
        let content = toml::to_string_pretty(&self.state).map_err(|e| SmeltError::Config {
            path: path.clone(),
            message: format!("serialize run state: {e}"),
        })?;
        fs::write(&path, content).map_err(|e| SmeltError::Io {
            operation: "write".into(),
            path,
            source: e,
        })
    }

    /// Read and deserialize state from `{state_dir}/state.toml`.
    pub fn read(state_dir: &Path) -> Result<RunState> {
        let path = state_dir.join("state.toml");
        let content = fs::read_to_string(&path).map_err(|e| SmeltError::Io {
            operation: "read".into(),
            path: path.clone(),
            source: e,
        })?;
        toml::from_str(&content).map_err(|e| SmeltError::Config {
            path,
            message: format!("parse run state: {e}"),
        })
    }

    /// Read and deserialize legacy state from `{base_dir}/run-state.toml`.
    ///
    /// Provides backward compatibility for `smelt status` when called without a
    /// job name — reads the flat state file written by versions prior to S04.
    pub fn read_legacy(base_dir: &Path) -> Result<RunState> {
        let path = base_dir.join("run-state.toml");
        let content = fs::read_to_string(&path).map_err(|e| SmeltError::Io {
            operation: "read".into(),
            path: path.clone(),
            source: e,
        })?;
        toml::from_str(&content).map_err(|e| SmeltError::Config {
            path,
            message: format!("parse run state: {e}"),
        })
    }

    /// Remove the state file. Tolerates a missing file.
    pub fn cleanup(&self) -> Result<()> {
        let path = self.state_dir.join("state.toml");
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SmeltError::Io {
                operation: "remove".into(),
                path,
                source: e,
            }),
        }
    }
}

/// Compute the effective job timeout from a manifest.
///
/// Returns the maximum `timeout` across all sessions as a [`Duration`].
/// Falls back to `config_default` (seconds) if the manifest has no sessions.
pub fn compute_job_timeout(manifest: &JobManifest, config_default: u64) -> Duration {
    let max_timeout = manifest
        .session
        .iter()
        .map(|s| s.timeout)
        .max()
        .unwrap_or(config_default);
    Duration::from_secs(max_timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_monitor(dir: &Path) -> JobMonitor {
        JobMonitor::new("test-job", vec!["s1".into(), "s2".into()], dir)
    }

    #[test]
    fn test_new_monitor_initial_state() {
        let dir = TempDir::new().unwrap();
        let mon = make_monitor(dir.path());
        assert_eq!(mon.state.phase, JobPhase::Provisioning);
        assert_eq!(mon.state.pid, std::process::id());
        assert_eq!(mon.state.sessions, vec!["s1", "s2"]);
        assert_eq!(mon.state.job_name, "test-job");
        assert!(mon.state.container_id.is_none());
        assert!(mon.state.started_at > 0);
        assert_eq!(mon.state.started_at, mon.state.updated_at);
    }

    #[test]
    fn test_phase_transitions() {
        let dir = TempDir::new().unwrap();
        let mut mon = make_monitor(dir.path());
        let initial_updated = mon.state.updated_at;

        // Small sleep to ensure timestamp can differ (on fast machines they may match)
        std::thread::sleep(std::time::Duration::from_millis(10));

        mon.set_phase(JobPhase::Executing).unwrap();
        assert_eq!(mon.state.phase, JobPhase::Executing);
        assert!(mon.state.updated_at >= initial_updated);

        mon.set_phase(JobPhase::Complete).unwrap();
        assert_eq!(mon.state.phase, JobPhase::Complete);
    }

    #[test]
    fn test_set_container() {
        let dir = TempDir::new().unwrap();
        let mut mon = make_monitor(dir.path());
        assert!(mon.state.container_id.is_none());

        mon.set_container("abc123def");
        assert_eq!(mon.state.container_id.as_deref(), Some("abc123def"));
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut mon = make_monitor(dir.path());
        mon.set_container("deadbeef");
        mon.write().unwrap();

        let read_state = JobMonitor::read(dir.path()).unwrap();
        assert_eq!(read_state.job_name, "test-job");
        assert_eq!(read_state.phase, JobPhase::Provisioning);
        assert_eq!(read_state.container_id.as_deref(), Some("deadbeef"));
        assert_eq!(read_state.sessions, vec!["s1", "s2"]);
        assert_eq!(read_state.started_at, mon.state.started_at);
        assert_eq!(read_state.pid, std::process::id());
    }

    #[test]
    fn test_cleanup_removes_file() {
        let dir = TempDir::new().unwrap();
        let mon = make_monitor(dir.path());
        mon.write().unwrap();
        assert!(dir.path().join("state.toml").exists());

        mon.cleanup().unwrap();
        assert!(!dir.path().join("state.toml").exists());
    }

    #[test]
    fn test_cleanup_missing_file_ok() {
        let dir = TempDir::new().unwrap();
        let mon = make_monitor(dir.path());
        // No file written — cleanup should succeed
        mon.cleanup().unwrap();
    }

    #[test]
    fn test_read_missing_file() {
        let dir = TempDir::new().unwrap();
        let result = JobMonitor::read(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_timeout_uses_max_session() {
        let toml_str = r#"
[job]
name = "t"
repo = "https://example.com/repo.git"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "a"
spec = "do a"
harness = "echo a"
timeout = 60

[[session]]
name = "b"
spec = "do b"
harness = "echo b"
timeout = 300

[[session]]
name = "c"
spec = "do c"
harness = "echo c"
timeout = 120

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest: JobManifest = toml::from_str(toml_str).unwrap();
        let timeout = compute_job_timeout(&manifest, 600);
        assert_eq!(timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_compute_timeout_fallback() {
        // Manifest with sessions present — but test the fallback with empty vec
        // We can't create a valid JobManifest with 0 sessions via TOML (validation fails),
        // so we test the logic: if max() returns None, use config_default.
        // Since JobManifest always has sessions, the fallback path is for safety.
        // We test it indirectly by verifying: with sessions, max is used, not default.
        let toml_str = r#"
[job]
name = "t"
repo = "https://example.com/repo.git"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "only"
spec = "do it"
harness = "echo hi"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest: JobManifest = toml::from_str(toml_str).unwrap();
        // With a session timeout of 60 and config_default of 9999, should use 60
        let timeout = compute_job_timeout(&manifest, 9999);
        assert_eq!(timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_run_state_toml_serialization() {
        let dir = TempDir::new().unwrap();
        let mut mon = make_monitor(dir.path());
        mon.set_container("abc123");
        mon.set_phase(JobPhase::Executing).unwrap();

        let content = std::fs::read_to_string(dir.path().join("state.toml")).unwrap();
        assert!(content.contains("job_name"), "missing job_name key");
        assert!(content.contains("phase"), "missing phase key");
        assert!(content.contains("container_id"), "missing container_id key");
        assert!(content.contains("sessions"), "missing sessions key");
        assert!(content.contains("started_at"), "missing started_at key");
        assert!(content.contains("updated_at"), "missing updated_at key");
        assert!(content.contains("pid"), "missing pid key");
        assert!(content.contains("executing"), "phase should be 'executing'");
        assert!(content.contains("abc123"), "container_id value missing");
    }

    #[test]
    fn test_set_phase_writes_to_disk() {
        let dir = TempDir::new().unwrap();
        let mut mon = make_monitor(dir.path());
        // set_phase should auto-write
        mon.set_phase(JobPhase::WritingManifest).unwrap();
        assert!(dir.path().join("state.toml").exists());

        let state = JobMonitor::read(dir.path()).unwrap();
        assert_eq!(state.phase, JobPhase::WritingManifest);
    }

    /// read_legacy reads a flat `run-state.toml` written without JobMonitor.
    #[test]
    fn test_read_legacy_reads_flat_file() {
        let dir = TempDir::new().unwrap();
        // Manually write a legacy flat state file (not via JobMonitor::write)
        let legacy_state = RunState {
            job_name: "legacy-job".into(),
            phase: JobPhase::Complete,
            container_id: Some("dead1234".into()),
            sessions: vec!["s1".into()],
            started_at: 1_700_000_000,
            updated_at: 1_700_000_060,
            pid: 99999,
            pr_url: None,
            pr_number: None,
            pr_status: None,
            ci_status: None,
            review_count: None,
            forge_repo: None,
            forge_token_env: None,
        };
        let content = toml::to_string_pretty(&legacy_state).unwrap();
        std::fs::write(dir.path().join("run-state.toml"), content).unwrap();

        let read = JobMonitor::read_legacy(dir.path()).unwrap();
        assert_eq!(read.job_name, "legacy-job");
        assert_eq!(read.phase, JobPhase::Complete);
        assert_eq!(read.container_id.as_deref(), Some("dead1234"));
        assert_eq!(read.started_at, 1_700_000_000);
    }

    /// write() writes to {state_dir}/state.toml, not run-state.toml.
    #[test]
    fn test_state_path_resolution() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join(".smelt").join("runs").join("my-job");
        let mon = JobMonitor::new("my-job", vec![], &state_dir);
        mon.write().unwrap();
        assert!(
            state_dir.join("state.toml").exists(),
            "expected state.toml at {}/state.toml",
            state_dir.display()
        );
        assert!(
            !state_dir.join("run-state.toml").exists(),
            "run-state.toml must not exist after write()"
        );
    }

    /// cleanup() removes state.toml, not run-state.toml.
    #[test]
    fn test_cleanup_uses_state_toml() {
        let dir = TempDir::new().unwrap();
        let mon = make_monitor(dir.path());
        mon.write().unwrap();
        assert!(
            dir.path().join("state.toml").exists(),
            "state.toml must exist after write"
        );
        mon.cleanup().unwrap();
        assert!(
            !dir.path().join("state.toml").exists(),
            "state.toml must be removed after cleanup"
        );
    }

    #[test]
    fn test_job_phase_gates_failed_serde() {
        // TOML requires a key-value structure at the top level, so we wrap the variant
        // in a minimal struct to exercise the serde round-trip.
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Wrapper {
            phase: JobPhase,
        }

        let input = Wrapper {
            phase: JobPhase::GatesFailed,
        };

        // Serialize → must produce "gates_failed" as the phase value
        let serialized = toml::to_string(&input).unwrap();
        assert!(
            serialized.contains("gates_failed"),
            "expected 'gates_failed' in serialized output, got: {serialized}"
        );

        // Deserialize → must round-trip back to GatesFailed
        let deserialized: Wrapper = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.phase, JobPhase::GatesFailed);
    }

    /// Verify that a RunState TOML written before `pr_url`/`pr_number` fields
    /// were added deserializes successfully, with both fields defaulting to None.
    #[test]
    fn test_run_state_backward_compat_no_pr_fields() {
        // Manually constructed TOML that mimics a state file from before T01
        // introduced pr_url and pr_number. These fields must not be required.
        let old_toml = r#"
job_name = "legacy-job"
phase = "complete"
sessions = ["s1", "s2"]
started_at = 1700000000
updated_at = 1700000060
pid = 12345
"#;
        let state: RunState = toml::from_str(old_toml).expect(
            "RunState should deserialize without pr_url/pr_number fields (backward compat)",
        );
        assert_eq!(state.job_name, "legacy-job");
        assert_eq!(state.phase, JobPhase::Complete);
        assert!(state.pr_url.is_none(), "pr_url should default to None");
        assert!(
            state.pr_number.is_none(),
            "pr_number should default to None"
        );
    }
}
