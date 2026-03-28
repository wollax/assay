//! `smelt watch` subcommand — poll a PR until merged or closed.
//!
//! Reads RunState from `.smelt/run-state.toml`, reconstructs the GitHub
//! client using `forge_token_env`, then polls `poll_pr_status()` on the
//! configured interval.  Exits 0 on `Merged`, exits 1 on `Closed`, loops on
//! `Open`.  Transient API errors print a warning but do not abort.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::Args;
use tracing::{error, info, warn};

use smelt_core::forge::{CiStatus, ForgeClient, GitHubForge, PrState};
use smelt_core::monitor::{JobMonitor, RunState};

/// Watch a PR until it is merged or closed.
#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Job name to watch (must match job.name in the manifest used for smelt run).
    pub job_name: String,
    /// Polling interval in seconds.
    #[arg(long, default_value_t = 30)]
    pub interval_secs: u64,
}

/// Execute the `watch` subcommand.
pub async fn execute(args: &WatchArgs) -> Result<i32> {
    let state_dir = PathBuf::from(".")
        .join(".smelt")
        .join("runs")
        .join(&args.job_name);

    // Read RunState from .smelt/runs/<job_name>/state.toml
    let state = match JobMonitor::read(&state_dir) {
        Ok(s) => s,
        Err(_) => {
            error!("No state file — has `smelt run` been called?");
            return Ok(1);
        }
    };

    // Check pr_url — surface the expected state path for diagnosability
    if state.pr_url.is_none() {
        error!(
            "No PR was created for this job (pr_url is not set in state at {}). \
             Did `smelt run` complete Phase 9?",
            state_dir.display()
        );
        return Ok(1);
    }

    // Check forge_token_env
    let token_env = match &state.forge_token_env {
        Some(t) => t.clone(),
        None => {
            error!(
                "No forge context in state (forge_token_env missing). \
                 State was written before S03."
            );
            return Ok(1);
        }
    };

    // Read token from env
    let token = match std::env::var(&token_env) {
        Ok(t) => t,
        Err(_) => {
            error!("env var `{token_env}` not set — required for PR polling");
            return Ok(1);
        }
    };

    // Parse repo
    let repo = match &state.forge_repo {
        Some(r) => r.clone(),
        None => {
            error!("No forge_repo in state — cannot determine repository to poll.");
            return Ok(1);
        }
    };

    // Parse PR number
    let pr_number = match state.pr_number {
        Some(n) => n,
        None => {
            error!("No pr_number in state — cannot determine PR to poll.");
            return Ok(1);
        }
    };

    // Construct GitHubForge
    let forge = GitHubForge::new(token)?;

    run_watch(
        &state_dir,
        forge,
        pr_number,
        &repo,
        Duration::from_secs(args.interval_secs),
    )
    .await
}

/// Inner polling loop; accepts any [`ForgeClient`] impl for testability.
///
/// On each poll:
/// - Updates RunState fields (`pr_status`, `ci_status`, `review_count`) on disk.
/// - Prints a one-liner to stderr.
/// - Returns `Ok(0)` on `Merged`, `Ok(1)` on `Closed`, loops on `Open`.
/// - Transient errors print a `[WARN]` line and retry after `interval`.
pub(crate) async fn run_watch<F: ForgeClient>(
    state_dir: &Path,
    forge: F,
    pr_number: u64,
    repo: &str,
    interval: Duration,
) -> Result<i32> {
    loop {
        match forge.poll_pr_status(repo, pr_number).await {
            Err(e) => {
                warn!("poll failed: {e:#} — retrying in {}s", interval.as_secs());
                tokio::time::sleep(interval).await;
            }
            Ok(status) => {
                // Update RunState with the latest poll data.
                if let Ok(mut state) = JobMonitor::read(state_dir) {
                    state.pr_status = Some(status.state.clone());
                    state.ci_status = Some(status.ci_status.clone());
                    state.review_count = Some(status.review_count);
                    // Best-effort: ignore write errors (observability, not blocking).
                    let _ = persist_run_state(state_dir, &state);
                }

                // Print poll line.
                let time = local_time_hms();
                let pr_state_str = pr_state_label(&status.state);
                let ci_str = ci_status_label(&status.ci_status);
                info!(
                    "[{time}] PR #{pr_number} — state: {pr_state_str} | CI: {ci_str} | reviews: {}",
                    status.review_count
                );

                match status.state {
                    PrState::Merged => {
                        info!("PR merged.");
                        return Ok(0);
                    }
                    PrState::Closed => {
                        info!("PR closed without merging.");
                        return Ok(1);
                    }
                    PrState::Open => {
                        tokio::time::sleep(interval).await;
                    }
                }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the current local time as `"HH:MM:SS"` (UTC approximation via std).
fn local_time_hms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

fn pr_state_label(state: &PrState) -> &'static str {
    match state {
        PrState::Open => "Open",
        PrState::Merged => "Merged",
        PrState::Closed => "Closed",
    }
}

fn ci_status_label(ci: &CiStatus) -> &'static str {
    match ci {
        CiStatus::Pending => "Pending",
        CiStatus::Passing => "Passing",
        CiStatus::Failing => "Failing",
        CiStatus::Unknown => "Unknown",
    }
}

/// Write updated [`RunState`] back to `{state_dir}/state.toml`.
fn persist_run_state(state_dir: &Path, state: &RunState) -> Result<()> {
    let path = state_dir.join("state.toml");
    let content = toml::to_string_pretty(state)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use smelt_core::forge::{PrHandle, PrState, PrStatus};
    use smelt_core::monitor::{JobMonitor, JobPhase, RunState};

    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use tempfile::TempDir;

    // ── MockForge ─────────────────────────────────────────────────────────────

    struct MockForge {
        responses: Mutex<VecDeque<PrStatus>>,
        default: PrStatus,
    }

    impl MockForge {
        fn new(responses: Vec<PrStatus>, default: PrStatus) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
                default,
            }
        }
    }

    impl smelt_core::forge::ForgeClient for MockForge {
        async fn create_pr(
            &self,
            _repo: &str,
            _head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> smelt_core::Result<PrHandle> {
            unimplemented!("MockForge::create_pr not used in watch tests")
        }

        async fn poll_pr_status(&self, _repo: &str, _number: u64) -> smelt_core::Result<PrStatus> {
            let mut q = self.responses.lock().unwrap();
            Ok(q.pop_front().unwrap_or_else(|| self.default.clone()))
        }
    }

    // ── PrStatus helpers ──────────────────────────────────────────────────────

    fn open_status() -> PrStatus {
        PrStatus {
            state: PrState::Open,
            ci_status: CiStatus::Pending,
            review_count: 0,
        }
    }

    fn merged_status() -> PrStatus {
        PrStatus {
            state: PrState::Merged,
            ci_status: CiStatus::Passing,
            review_count: 2,
        }
    }

    fn closed_status() -> PrStatus {
        PrStatus {
            state: PrState::Closed,
            ci_status: CiStatus::Failing,
            review_count: 0,
        }
    }

    // ── RunState helpers ──────────────────────────────────────────────────────

    fn make_run_state() -> RunState {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        RunState {
            job_name: "test-job".into(),
            phase: JobPhase::Complete,
            container_id: None,
            sessions: vec![],
            started_at: now,
            updated_at: now,
            pid: std::process::id(),
            pr_url: Some("https://github.com/owner/repo/pull/7".into()),
            pr_number: Some(7),
            pr_status: None,
            ci_status: None,
            review_count: None,
            forge_repo: Some("owner/repo".into()),
            forge_token_env: Some("GITHUB_TOKEN".into()),
        }
    }

    fn write_state_to_dir(tmp: &TempDir, state: &RunState) -> PathBuf {
        // Mirror the per-job layout: .smelt/runs/<job_name>/state.toml
        let state_dir = tmp.path().join(".smelt").join("runs").join(&state.job_name);
        std::fs::create_dir_all(&state_dir).unwrap();
        let content = toml::to_string_pretty(state).unwrap();
        std::fs::write(state_dir.join("state.toml"), content).unwrap();
        state_dir
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// run_watch returns Ok(0) when the poll sequence ends with Merged.
    #[tokio::test]
    async fn test_watch_exits_0_on_merged() {
        let tmp = TempDir::new().unwrap();
        let state_dir = write_state_to_dir(&tmp, &make_run_state());

        let forge = MockForge::new(
            vec![open_status(), open_status(), merged_status()],
            merged_status(),
        );

        let result = run_watch(&state_dir, forge, 7, "owner/repo", Duration::ZERO).await;
        assert_eq!(result.unwrap(), 0, "should exit 0 on Merged");
    }

    /// run_watch returns Ok(1) when the poll sequence ends with Closed.
    #[tokio::test]
    async fn test_watch_exits_1_on_closed() {
        let tmp = TempDir::new().unwrap();
        let state_dir = write_state_to_dir(&tmp, &make_run_state());

        let forge = MockForge::new(vec![open_status(), closed_status()], closed_status());

        let result = run_watch(&state_dir, forge, 7, "owner/repo", Duration::ZERO).await;
        assert_eq!(result.unwrap(), 1, "should exit 1 on Closed");
    }

    /// run_watch returns Ok(0) immediately when the first poll returns Merged.
    #[tokio::test]
    async fn test_watch_immediate_merged() {
        let tmp = TempDir::new().unwrap();
        let state_dir = write_state_to_dir(&tmp, &make_run_state());

        let forge = MockForge::new(vec![merged_status()], merged_status());

        let result = run_watch(&state_dir, forge, 7, "owner/repo", Duration::ZERO).await;
        assert_eq!(result.unwrap(), 0, "should exit 0 immediately on Merged");
    }

    /// run_watch updates pr_status, ci_status, review_count in RunState on each poll.
    #[tokio::test]
    async fn test_watch_updates_run_state_each_poll() {
        let tmp = TempDir::new().unwrap();
        let state_dir = write_state_to_dir(&tmp, &make_run_state());

        // Two polls: Open then Merged.
        let forge = MockForge::new(vec![open_status(), merged_status()], merged_status());

        let result = run_watch(&state_dir, forge, 7, "owner/repo", Duration::ZERO).await;
        assert_eq!(result.unwrap(), 0);

        // After run_watch returns, the last poll (Merged) must be persisted.
        let updated = JobMonitor::read(&state_dir).unwrap();
        assert_eq!(
            updated.pr_status,
            Some(PrState::Merged),
            "pr_status should be Merged after last poll"
        );
        assert!(
            updated.ci_status.is_some(),
            "ci_status should be set after polling"
        );
        assert!(
            updated.review_count.is_some(),
            "review_count should be set after polling"
        );
    }
}
