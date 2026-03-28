//! Tracker source trait, template manifest loading, and issue-to-manifest
//! injection.
//!
//! Implementations talk to a specific tracker provider (GitHub, Linear, etc.)
//! and normalize results into `TrackerIssue` / `TrackerState` (from
//! `smelt_core::tracker`).

use std::path::Path;

use anyhow::Context;
use smelt_core::manifest::{JobManifest, SessionDef};
use smelt_core::tracker::{TrackerIssue, TrackerState};

use super::config::TrackerConfig;

/// Async trait for a tracker source that can poll for ready issues and
/// transition issue lifecycle state.
///
/// Uses RPITIT (return-position `impl Trait` in trait) per D019 — no
/// `#[async_trait]` macro needed on Rust edition 2024.
pub trait TrackerSource: Send + Sync {
    /// Poll the tracker for issues in the `Ready` state.
    ///
    /// Returns all issues that are ready to be picked up for execution.
    fn poll_ready_issues(&self) -> impl Future<Output = anyhow::Result<Vec<TrackerIssue>>> + Send;

    /// Transition an issue from one lifecycle state to another.
    ///
    /// Implementations should update the issue's labels in the external
    /// tracker to reflect the new state.
    fn transition_state(
        &self,
        issue_id: &str,
        from: TrackerState,
        to: TrackerState,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// Load a template manifest from disk and verify it contains no `[[session]]`
/// entries.
///
/// Template manifests are loaded at startup (D017) and used as the base for
/// issue-to-manifest injection. They must not contain session entries because
/// sessions are injected dynamically from tracker issues.
///
/// Note: `JobManifest::validate()` is intentionally *not* called because it
/// requires at least one session — which templates must not have. The TOML
/// parse via `JobManifest::load()` already enforces structural correctness
/// (`deny_unknown_fields`).
pub fn load_template_manifest(path: &Path) -> anyhow::Result<JobManifest> {
    let manifest = JobManifest::load(path)
        .with_context(|| format!("failed to load template manifest {}", path.display()))?;
    if !manifest.session.is_empty() {
        anyhow::bail!(
            "template manifest {} must not contain [[session]] entries",
            path.display()
        );
    }
    tracing::info!(path = %path.display(), "loaded template manifest");
    Ok(manifest)
}

/// Inject a tracker issue into a template manifest as a new session.
///
/// Clones the template, creates a `SessionDef` from the issue metadata
/// and tracker config defaults, and pushes it into the manifest's session
/// list.
///
/// # Errors
///
/// Returns an error if the issue title produces an empty session name after
/// sanitization (e.g. a title consisting entirely of special characters).
pub fn issue_to_manifest(
    template: &JobManifest,
    issue: &TrackerIssue,
    config: &TrackerConfig,
) -> anyhow::Result<JobManifest> {
    let name = sanitize(&issue.title);
    if name.is_empty() {
        anyhow::bail!(
            "tracker issue {} has a title that produces an empty session name after sanitization: {:?}",
            issue.id,
            issue.title
        );
    }
    let mut manifest = template.clone();
    let session = SessionDef {
        name,
        spec: issue.body.clone(),
        harness: config.default_harness.clone(),
        timeout: config.default_timeout,
        depends_on: vec![],
    };
    manifest.session.push(session);
    Ok(manifest)
}

/// Sanitize a string for use as a session name: lowercase, replace
/// non-alphanumeric characters with hyphens, collapse consecutive hyphens,
/// and trim leading/trailing hyphens.
fn sanitize(input: &str) -> String {
    let lowered = input.to_lowercase();
    let replaced: String = lowered
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse consecutive hyphens and trim leading/trailing hyphens.
    let mut result = String::with_capacity(replaced.len());
    let mut prev_hyphen = true; // treat start as hyphen to trim leading
    for c in replaced.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }
    result
}

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// A test double for [`TrackerSource`] that returns pre-configured results
    /// from VecDeque queues, following the `MockSshClient` pattern.
    pub struct MockTrackerSource {
        poll_results: Arc<Mutex<VecDeque<anyhow::Result<Vec<TrackerIssue>>>>>,
        transition_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
    }

    impl MockTrackerSource {
        /// Create a new `MockTrackerSource` with empty result queues.
        pub fn new() -> Self {
            Self {
                poll_results: Arc::new(Mutex::new(VecDeque::new())),
                transition_results: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        /// Enqueue a poll result.
        pub fn with_poll_result(self, result: anyhow::Result<Vec<TrackerIssue>>) -> Self {
            self.poll_results.lock().unwrap().push_back(result);
            self
        }

        /// Enqueue a transition result.
        pub fn with_transition_result(self, result: anyhow::Result<()>) -> Self {
            self.transition_results.lock().unwrap().push_back(result);
            self
        }
    }

    impl TrackerSource for MockTrackerSource {
        async fn poll_ready_issues(&self) -> anyhow::Result<Vec<TrackerIssue>> {
            self.poll_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(anyhow::anyhow!("MockTrackerSource: no poll results queued"))
                })
        }

        async fn transition_state(
            &self,
            _issue_id: &str,
            _from: TrackerState,
            _to: TrackerState,
        ) -> anyhow::Result<()> {
            self.transition_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(anyhow::anyhow!(
                        "MockTrackerSource: no transition results queued"
                    ))
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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

    /// Template manifest with a `[[session]]` entry — must be rejected.
    const TEMPLATE_WITH_SESSION_TOML: &str = r#"
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

[[session]]
name = "bad"
spec = "should not be here"
harness = "bash"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;

    fn write_temp_toml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    fn make_tracker_config() -> TrackerConfig {
        TrackerConfig {
            provider: "github".into(),
            repo: Some("owner/repo".into()),
            manifest_template: "/tmp/unused.toml".into(),
            poll_interval_secs: 30,
            label_prefix: "smelt".into(),
            default_harness: "bash".into(),
            default_timeout: 600,
        }
    }

    fn make_issue(title: &str, body: &str) -> TrackerIssue {
        TrackerIssue {
            id: "TEST-1".into(),
            title: title.into(),
            body: body.into(),
            source_url: "https://example.com/TEST-1".into(),
        }
    }

    // ── load_template_manifest tests ────────────────────────────

    #[test]
    fn test_load_template_manifest_valid() {
        let f = write_temp_toml(TEMPLATE_TOML);
        let manifest = load_template_manifest(f.path()).expect("should load valid template");
        assert_eq!(manifest.job.name, "template");
        assert!(manifest.session.is_empty());
    }

    #[test]
    fn test_load_template_manifest_rejects_sessions() {
        let f = write_temp_toml(TEMPLATE_WITH_SESSION_TOML);
        let err = load_template_manifest(f.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must not contain [[session]] entries"),
            "got: {msg}"
        );
        // Error message should include the file path.
        assert!(
            msg.contains(f.path().to_str().unwrap()),
            "error should mention file path, got: {msg}"
        );
    }

    #[test]
    fn test_load_template_manifest_nonexistent_file() {
        let err = load_template_manifest(Path::new("/nonexistent/path.toml")).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("failed to load template manifest"),
            "got: {msg}"
        );
    }

    // ── issue_to_manifest tests ─────────────────────────────────

    #[test]
    fn test_issue_to_manifest_injects_session() {
        let f = write_temp_toml(TEMPLATE_TOML);
        let template = load_template_manifest(f.path()).unwrap();
        let config = make_tracker_config();
        let issue = make_issue("My Test Issue", "Run the test suite");

        let result = issue_to_manifest(&template, &issue, &config).unwrap();
        assert_eq!(result.session.len(), 1);
        let session = &result.session[0];
        assert_eq!(session.name, "my-test-issue");
        assert_eq!(session.spec, "Run the test suite");
        assert_eq!(session.harness, "bash");
        assert_eq!(session.timeout, 600);
        assert!(session.depends_on.is_empty());
    }

    #[test]
    fn test_issue_to_manifest_sanitizes_title() {
        let f = write_temp_toml(TEMPLATE_TOML);
        let template = load_template_manifest(f.path()).unwrap();
        let config = make_tracker_config();
        let issue = make_issue("Fix: Bug #42 (urgent!!)", "fix it");

        let result = issue_to_manifest(&template, &issue, &config).unwrap();
        assert_eq!(result.session[0].name, "fix-bug-42-urgent");
    }

    #[test]
    fn test_issue_to_manifest_preserves_template() {
        let f = write_temp_toml(TEMPLATE_TOML);
        let template = load_template_manifest(f.path()).unwrap();
        let config = make_tracker_config();
        let issue = make_issue("Test", "spec");

        let result = issue_to_manifest(&template, &issue, &config).unwrap();
        // Template fields preserved
        assert_eq!(result.job.name, "template");
        assert_eq!(result.environment.image, "ubuntu:22.04");
        // Original template unchanged
        assert!(template.session.is_empty());
    }

    // ── sanitize tests ──────────────────────────────────────────

    #[test]
    fn test_sanitize_basic() {
        assert_eq!(sanitize("Hello World"), "hello-world");
    }

    #[test]
    fn test_sanitize_special_chars() {
        assert_eq!(sanitize("Fix: Bug #42 (urgent!!)"), "fix-bug-42-urgent");
    }

    #[test]
    fn test_sanitize_consecutive_specials() {
        assert_eq!(sanitize("a---b___c"), "a-b-c");
    }

    #[test]
    fn test_sanitize_leading_trailing() {
        assert_eq!(sanitize("--hello--"), "hello");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize(""), "");
    }

    #[test]
    fn test_sanitize_all_special() {
        assert_eq!(sanitize("!@#$%"), "");
    }

    #[test]
    fn test_issue_to_manifest_rejects_empty_name() {
        let f = write_temp_toml(TEMPLATE_TOML);
        let template = load_template_manifest(f.path()).unwrap();
        let config = make_tracker_config();
        let issue = make_issue("!@#$%", "spec text");

        let err = issue_to_manifest(&template, &issue, &config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("empty session name"), "got: {msg}");
        assert!(
            msg.contains("TEST-1"),
            "should mention issue id, got: {msg}"
        );
    }

    // ── MockTrackerSource tests ─────────────────────────────────

    #[tokio::test]
    async fn test_mock_tracker_poll_and_transition() {
        use mock::MockTrackerSource;

        let issue = TrackerIssue {
            id: "KAT-1".into(),
            title: "Test issue".into(),
            body: "Test body".into(),
            source_url: "https://example.com/KAT-1".into(),
        };

        let source = MockTrackerSource::new()
            .with_poll_result(Ok(vec![issue.clone()]))
            .with_poll_result(Ok(vec![]))
            .with_transition_result(Ok(()))
            .with_transition_result(Err(anyhow::anyhow!("transition failed")));

        // First poll: returns one issue
        let issues = source.poll_ready_issues().await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "KAT-1");

        // Second poll: returns empty
        let issues = source.poll_ready_issues().await.unwrap();
        assert!(issues.is_empty());

        // Third poll: no more queued results → error
        assert!(source.poll_ready_issues().await.is_err());

        // First transition: succeeds
        source
            .transition_state("KAT-1", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap();

        // Second transition: fails
        let err = source
            .transition_state("KAT-1", TrackerState::Queued, TrackerState::Running)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("transition failed"));
    }

    // ── Compile-test for the trait ──────────────────────────────

    /// Compile-test: a trivial implementation of `TrackerSource` proves the
    /// trait definition compiles and can be implemented.
    struct DummySource;

    impl TrackerSource for DummySource {
        async fn poll_ready_issues(&self) -> anyhow::Result<Vec<TrackerIssue>> {
            Ok(vec![])
        }

        async fn transition_state(
            &self,
            _issue_id: &str,
            _from: TrackerState,
            _to: TrackerState,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn tracker_source_trait_compiles_and_works() {
        let source = DummySource;
        let issues = source.poll_ready_issues().await.unwrap();
        assert!(issues.is_empty());

        source
            .transition_state("TEST-1", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap();
    }
}
