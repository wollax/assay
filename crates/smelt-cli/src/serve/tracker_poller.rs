//! `TrackerPoller` — background task that polls tracker backends for ready
//! issues and enqueues them as jobs in `ServerState`.
//!
//! The `AnyTrackerSource` enum dispatches to concrete tracker source
//! implementations (GitHub, Linear, Mock) without object-safety problems
//! from the RPITIT-based `TrackerSource` trait (D084 pattern).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use smelt_core::manifest::JobManifest;
use smelt_core::tracker::{TrackerIssue, TrackerState};

use super::config::TrackerConfig;
use super::github::GithubTrackerSource;
use super::github::SubprocessGhClient;
use super::linear::LinearTrackerSource;
use super::linear::ReqwestLinearClient;
use super::queue::ServerState;
use super::tracker::TrackerSource;
use super::tracker::issue_to_manifest;
use super::types::JobSource;

/// Enum dispatcher for tracker source implementations.
///
/// Solves the non-object-safe RPITIT trait problem (D084): `TrackerSource`
/// uses `impl Future` in return position, which makes `dyn TrackerSource`
/// impossible. This enum dispatches to concrete implementations instead.
#[allow(dead_code)] // Variants constructed in T05 when wiring into `smelt serve`.
pub enum AnyTrackerSource {
    GitHub(GithubTrackerSource<SubprocessGhClient>),
    Linear(LinearTrackerSource<ReqwestLinearClient>),
    #[cfg(test)]
    Mock(super::tracker::mock::MockTrackerSource),
}

#[allow(dead_code)] // Wired into `smelt serve` in T05.
impl AnyTrackerSource {
    /// Poll for issues in the `Ready` state.
    pub async fn poll_ready_issues(&self) -> anyhow::Result<Vec<TrackerIssue>> {
        match self {
            AnyTrackerSource::GitHub(s) => s.poll_ready_issues().await,
            AnyTrackerSource::Linear(s) => s.poll_ready_issues().await,
            #[cfg(test)]
            AnyTrackerSource::Mock(s) => s.poll_ready_issues().await,
        }
    }

    /// Transition an issue from one lifecycle state to another.
    pub async fn transition_state(
        &self,
        issue_id: &str,
        from: TrackerState,
        to: TrackerState,
    ) -> anyhow::Result<()> {
        match self {
            AnyTrackerSource::GitHub(s) => s.transition_state(issue_id, from, to).await,
            AnyTrackerSource::Linear(s) => s.transition_state(issue_id, from, to).await,
            #[cfg(test)]
            AnyTrackerSource::Mock(s) => s.transition_state(issue_id, from, to).await,
        }
    }

    /// Ensure all lifecycle labels exist in the tracker backend.
    ///
    /// Takes `&mut self` because `LinearTrackerSource::ensure_labels` mutates
    /// internal label-cache state.
    pub async fn ensure_labels(&mut self) -> anyhow::Result<()> {
        match self {
            AnyTrackerSource::GitHub(s) => s.ensure_labels().await,
            AnyTrackerSource::Linear(s) => s.ensure_labels().await,
            #[cfg(test)]
            AnyTrackerSource::Mock(_) => Ok(()),
        }
    }
}

/// Background task that polls a tracker source for ready issues and enqueues
/// them as jobs in `ServerState`.
///
/// # Lifecycle
///
/// 1. Calls `ensure_labels()` once at startup (propagates error on failure).
/// 2. Polls on the configured interval using `tokio::time::interval`.
/// 3. For each ready issue: transitions `Ready→Queued` (D157), generates a
///    manifest via `issue_to_manifest()`, writes it to a temp file (D105),
///    and enqueues into `ServerState`.
/// 4. Exits cleanly when the cancellation token fires.
#[allow(dead_code)] // Wired into `smelt serve` in T05.
pub struct TrackerPoller {
    pub source: AnyTrackerSource,
    pub template: JobManifest,
    /// Raw TOML string of the template manifest, used for serialization
    /// since `JobManifest` does not implement `Serialize`.
    pub template_toml: String,
    pub config: TrackerConfig,
    pub state: Arc<Mutex<ServerState>>,
    pub cancel: CancellationToken,
    pub interval: Duration,
}

#[allow(dead_code)] // Wired into `smelt serve` in T05.
impl TrackerPoller {
    /// Run the poller loop until cancellation.
    ///
    /// Calls `ensure_labels()` once, then polls on the configured interval.
    /// Errors from `ensure_labels()` are fatal (propagated). Poll and
    /// transition errors are logged and skipped — the poller continues.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!(
            provider = %self.config.provider,
            interval_secs = self.interval.as_secs(),
            "tracker poller starting"
        );

        self.source.ensure_labels().await?;

        let mut ticker = tokio::time::interval(self.interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    tracing::info!("tracker poller cancelled, exiting");
                    return Ok(());
                }
                _ = ticker.tick() => {
                    self.poll_once().await;
                }
            }
        }
    }

    /// Execute a single poll cycle: fetch ready issues, transition, generate
    /// manifest, write to temp file, enqueue.
    async fn poll_once(&mut self) {
        let issues = match self.source.poll_ready_issues().await {
            Ok(issues) => {
                tracing::debug!(issues_found = issues.len(), "poll cycle complete");
                issues
            }
            Err(e) => {
                tracing::warn!(error = %e, "poll_ready_issues failed, skipping cycle");
                return;
            }
        };

        for issue in &issues {
            // D157: transition Ready→Queued before enqueue to prevent
            // double-dispatch on the next poll cycle.
            if let Err(e) = self
                .source
                .transition_state(&issue.id, TrackerState::Ready, TrackerState::Queued)
                .await
            {
                tracing::warn!(
                    issue_id = %issue.id,
                    error = %e,
                    "transition_state(Ready→Queued) failed, skipping issue"
                );
                continue;
            }

            // Generate the job manifest from the template + issue.
            let manifest = match issue_to_manifest(&self.template, issue, &self.config) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(
                        issue_id = %issue.id,
                        error = %e,
                        "issue_to_manifest failed, skipping issue"
                    );
                    continue;
                }
            };

            // Write the manifest to a temp file (D105 pattern).
            let path = match write_manifest_temp(&self.template_toml, &manifest, issue) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        issue_id = %issue.id,
                        error = %e,
                        "failed to write manifest temp file, skipping issue"
                    );
                    continue;
                }
            };

            // Enqueue the job.
            let job_id = self.state.lock().unwrap().enqueue(path, JobSource::Tracker);

            tracing::info!(
                issue_id = %issue.id,
                job_id = %job_id,
                "enqueued tracker issue as job"
            );
        }
    }
}

/// Write a manifest to a `NamedTempFile` and leak the path (D105 pattern).
///
/// Builds the TOML by parsing the template string as a `toml::Value`,
/// appending the session entry from the generated manifest, and serializing
/// back. The `NamedTempFile` path is leaked via `std::mem::forget` so the
/// file persists until the dispatch loop reads and cleans it up.
#[allow(dead_code)] // Used by TrackerPoller::poll_once, wired in T05.
fn write_manifest_temp(
    template_toml: &str,
    manifest: &JobManifest,
    issue: &TrackerIssue,
) -> anyhow::Result<PathBuf> {
    use std::io::Write;

    // Build the TOML content by combining the template with the session.
    let toml_content = build_manifest_toml(template_toml, manifest, issue)?;

    let mut tmp = tempfile::NamedTempFile::with_suffix(".smelt.toml")?;
    tmp.write_all(toml_content.as_bytes())?;
    tmp.flush()?;

    let path = tmp.into_temp_path();
    let path_buf = path.to_path_buf();

    // D105: leak the TempPath so the file is not auto-deleted.
    std::mem::forget(path);

    Ok(path_buf)
}

/// Build a TOML string for the manifest by appending the session from
/// `issue_to_manifest()` to the template TOML.
///
/// Uses `toml::Value` manipulation since `JobManifest` doesn't implement
/// `Serialize`. Parses the template as a TOML table, adds the session
/// entry, and serializes back.
fn build_manifest_toml(
    template_toml: &str,
    manifest: &JobManifest,
    _issue: &TrackerIssue,
) -> anyhow::Result<String> {
    let mut doc: toml::Value =
        toml::from_str(template_toml).map_err(|e| anyhow::anyhow!("template re-parse: {e}"))?;

    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("template TOML is not a table"))?;

    // The manifest from issue_to_manifest() has exactly one session.
    let session = manifest
        .session
        .first()
        .ok_or_else(|| anyhow::anyhow!("manifest has no session after issue_to_manifest"))?;

    let mut session_table = toml::value::Table::new();
    session_table.insert("name".into(), toml::Value::String(session.name.clone()));
    session_table.insert("spec".into(), toml::Value::String(session.spec.clone()));
    session_table.insert(
        "harness".into(),
        toml::Value::String(session.harness.clone()),
    );
    session_table.insert(
        "timeout".into(),
        toml::Value::Integer(session.timeout as i64),
    );
    if !session.depends_on.is_empty() {
        let deps: Vec<toml::Value> = session
            .depends_on
            .iter()
            .map(|d| toml::Value::String(d.clone()))
            .collect();
        session_table.insert("depends_on".into(), toml::Value::Array(deps));
    }

    table.insert(
        "session".into(),
        toml::Value::Array(vec![toml::Value::Table(session_table)]),
    );

    let out = toml::to_string_pretty(&doc)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::tracker::mock::MockTrackerSource;
    use super::*;

    use std::io::Write as _;

    /// Minimal valid template manifest TOML with zero sessions.
    const TEMPLATE_TOML: &str = r#"
[job]
name = "template"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[merge]
strategy = "sequential"
target = "main"
"#;

    fn make_tracker_config() -> TrackerConfig {
        TrackerConfig {
            provider: "mock".into(),
            repo: None,
            api_key_env: None,
            team_id: None,
            manifest_template: "/tmp/unused.toml".into(),
            poll_interval_secs: 1,
            label_prefix: "smelt".into(),
            default_harness: "bash".into(),
            default_timeout: 600,
        }
    }

    fn make_issue(id: &str, title: &str, body: &str) -> TrackerIssue {
        TrackerIssue {
            id: id.into(),
            title: title.into(),
            body: body.into(),
            source_url: format!("https://example.com/{id}"),
        }
    }

    fn load_template() -> JobManifest {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(TEMPLATE_TOML.as_bytes()).unwrap();
        super::super::tracker::load_template_manifest(f.path()).unwrap()
    }

    fn make_poller(
        source: MockTrackerSource,
        state: Arc<Mutex<ServerState>>,
        cancel: CancellationToken,
    ) -> TrackerPoller {
        TrackerPoller {
            source: AnyTrackerSource::Mock(source),
            template: load_template(),
            template_toml: TEMPLATE_TOML.to_string(),
            config: make_tracker_config(),
            state,
            cancel,
            interval: Duration::from_millis(50),
        }
    }

    fn make_state() -> Arc<Mutex<ServerState>> {
        Arc::new(Mutex::new(ServerState::new(10)))
    }

    // ── (a) test_poller_enqueues_issues ─────────────────────────

    #[tokio::test]
    async fn test_poller_enqueues_issues() {
        let issue1 = make_issue("ISS-1", "First Issue", "body one");
        let issue2 = make_issue("ISS-2", "Second Issue", "body two");

        let source = MockTrackerSource::new()
            .with_poll_result(Ok(vec![issue1, issue2]))
            .with_transition_result(Ok(()))
            .with_transition_result(Ok(()))
            // Second tick returns empty so the loop doesn't error on no results.
            .with_poll_result(Ok(vec![]));

        let state = make_state();
        let cancel = CancellationToken::new();
        let mut poller = make_poller(source, Arc::clone(&state), cancel.clone());

        // Run the poller in a spawned task, cancel after a short delay.
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move { poller.run().await });

        // Wait for the poller to process the first tick.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel.cancel();
        handle.await.unwrap().unwrap();

        let locked = state_clone.lock().unwrap();
        let queued: Vec<_> = locked
            .jobs
            .iter()
            .filter(|j| matches!(j.status, super::super::types::JobStatus::Queued))
            .collect();
        assert_eq!(
            queued.len(),
            2,
            "expected 2 queued jobs, got {}",
            queued.len()
        );
        // Verify source is Tracker
        for job in &queued {
            assert!(
                matches!(job.source, JobSource::Tracker),
                "expected Tracker source"
            );
        }
    }

    // ── (b) test_poller_skips_on_transition_error ───────────────

    #[tokio::test]
    async fn test_poller_skips_on_transition_error() {
        let issue = make_issue("ISS-1", "Issue One", "body");

        let source = MockTrackerSource::new()
            .with_poll_result(Ok(vec![issue]))
            .with_transition_result(Err(anyhow::anyhow!("transition failed")))
            .with_poll_result(Ok(vec![]));

        let state = make_state();
        let cancel = CancellationToken::new();
        let mut poller = make_poller(source, Arc::clone(&state), cancel.clone());

        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move { poller.run().await });

        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel.cancel();
        handle.await.unwrap().unwrap();

        let locked = state_clone.lock().unwrap();
        assert_eq!(
            locked.jobs.len(),
            0,
            "expected 0 jobs when transition fails"
        );
    }

    // ── (c) test_poller_continues_on_poll_error ─────────────────

    #[tokio::test]
    async fn test_poller_continues_on_poll_error() {
        let issue = make_issue("ISS-1", "Issue One", "body");

        let source = MockTrackerSource::new()
            .with_poll_result(Err(anyhow::anyhow!("poll error")))
            .with_poll_result(Ok(vec![issue]))
            .with_transition_result(Ok(()))
            .with_poll_result(Ok(vec![]));

        let state = make_state();
        let cancel = CancellationToken::new();
        let mut poller = make_poller(source, Arc::clone(&state), cancel.clone());

        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move { poller.run().await });

        // Give enough time for 2+ ticks
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel();
        handle.await.unwrap().unwrap();

        let locked = state_clone.lock().unwrap();
        assert_eq!(
            locked.jobs.len(),
            1,
            "expected 1 job after poll error recovery"
        );
    }

    // ── (d) test_poller_exits_on_cancellation ───────────────────

    #[tokio::test]
    async fn test_poller_exits_on_cancellation() {
        let source = MockTrackerSource::new()
            // Queue an empty result so if tick fires first it doesn't panic.
            .with_poll_result(Ok(vec![]));

        let state = make_state();
        let cancel = CancellationToken::new();
        let mut poller = make_poller(source, Arc::clone(&state), cancel.clone());

        // Cancel immediately before running.
        cancel.cancel();

        let result = poller.run().await;
        assert!(result.is_ok(), "poller should exit cleanly on cancellation");
    }

    // ── build_manifest_toml produces valid TOML ─────────────────

    #[test]
    fn test_build_manifest_toml_roundtrips() {
        let template = load_template();
        let config = make_tracker_config();
        let issue = make_issue("ISS-1", "Test Issue", "test body");

        let manifest = issue_to_manifest(&template, &issue, &config).unwrap();
        let toml_str = build_manifest_toml(TEMPLATE_TOML, &manifest, &issue).unwrap();

        // The output should parse back as a valid JobManifest.
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(toml_str.as_bytes()).unwrap();
        let reloaded = JobManifest::load(f.path()).unwrap();

        assert_eq!(reloaded.session.len(), 1);
        assert_eq!(reloaded.session[0].name, "test-issue");
        assert_eq!(reloaded.session[0].spec, "test body");
        assert_eq!(reloaded.session[0].harness, "bash");
        assert_eq!(reloaded.session[0].timeout, 600);
    }

    // ── write_manifest_temp creates file that persists ──────────

    #[test]
    fn test_write_manifest_temp_creates_file() {
        let template = load_template();
        let config = make_tracker_config();
        let issue = make_issue("ISS-1", "Temp File Test", "body");
        let manifest = issue_to_manifest(&template, &issue, &config).unwrap();

        let path = write_manifest_temp(TEMPLATE_TOML, &manifest, &issue).unwrap();

        // File should exist (not auto-deleted due to D105 forget).
        assert!(path.exists(), "temp file should exist at {:?}", path);

        // Should be loadable as a manifest.
        let loaded = JobManifest::load(&path).unwrap();
        assert_eq!(loaded.session.len(), 1);
        assert_eq!(loaded.session[0].name, "temp-file-test");

        // Clean up.
        let _ = std::fs::remove_file(&path);
    }
}
