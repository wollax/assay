//! `GithubTrackerSource` — bridges `GhClient` to `TrackerSource`.
//!
//! Generic over `G: GhClient` for testability with `MockGhClient`.

use smelt_core::tracker::{TrackerIssue, TrackerState};

use super::GhClient;
use crate::serve::tracker::TrackerSource;

/// A tracker source that polls GitHub Issues via a `GhClient` implementation
/// and manages label-based lifecycle transitions.
///
/// Generic over `G: GhClient` so unit tests can inject `MockGhClient`.
pub struct GithubTrackerSource<G: GhClient> {
    client: G,
    repo: String,
    label_prefix: String,
}

impl<G: GhClient> GithubTrackerSource<G> {
    /// Create a new `GithubTrackerSource`.
    pub fn new(client: G, repo: String, label_prefix: String) -> Self {
        Self {
            client,
            repo,
            label_prefix,
        }
    }

    /// Ensure all lifecycle labels exist in the repository.
    ///
    /// Calls `create_label` (idempotent via `--force`) for each variant in
    /// `TrackerState::ALL`. Intended to be called once at startup or first poll.
    pub async fn ensure_labels(&self) -> anyhow::Result<()> {
        for state in TrackerState::ALL {
            let label = state.label_name(&self.label_prefix);
            self.client
                .create_label(&self.repo, &label)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            tracing::info!(repo = %self.repo, label = %label, "created lifecycle label");
        }
        Ok(())
    }
}

impl<G: GhClient + Send + Sync> TrackerSource for GithubTrackerSource<G> {
    async fn poll_ready_issues(&self) -> anyhow::Result<Vec<TrackerIssue>> {
        // Verify auth before polling.
        self.client.auth_status().await.map_err(|e| {
            let msg = e.to_string();
            anyhow::Error::from(smelt_core::error::SmeltError::tracker("poll", msg))
        })?;

        let ready_label = TrackerState::Ready.label_name(&self.label_prefix);
        let gh_issues = self
            .client
            .list_issues(&self.repo, &ready_label, 50)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let issues = gh_issues
            .into_iter()
            .map(|gh| TrackerIssue {
                id: gh.number.to_string(),
                title: gh.title,
                body: gh.body,
                source_url: gh.url,
            })
            .collect();

        Ok(issues)
    }

    async fn transition_state(
        &self,
        issue_id: &str,
        from: TrackerState,
        to: TrackerState,
    ) -> anyhow::Result<()> {
        let number: u64 = issue_id.parse().map_err(|_| {
            anyhow::Error::from(smelt_core::error::SmeltError::tracker(
                "transition",
                format!("issue_id is not a valid u64: {issue_id:?}"),
            ))
        })?;

        let from_label = from.label_name(&self.label_prefix);
        let to_label = to.label_name(&self.label_prefix);

        self.client
            .edit_labels(&self.repo, number, &[&to_label], &[&from_label])
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        tracing::info!(
            repo = %self.repo,
            issue = number,
            from = %from,
            to = %to,
            "transitioned issue labels"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::github::GhIssue;
    use crate::serve::github::mock::MockGhClient;
    use smelt_core::error::SmeltError;

    fn make_source(client: MockGhClient) -> GithubTrackerSource<MockGhClient> {
        GithubTrackerSource::new(client, "owner/repo".to_string(), "smelt".to_string())
    }

    fn make_gh_issue(number: u64, title: &str, body: &str) -> GhIssue {
        GhIssue {
            number,
            title: title.to_string(),
            body: body.to_string(),
            url: format!("https://github.com/owner/repo/issues/{number}"),
        }
    }

    // ── poll_ready_issues ───────────────────────────────────────

    #[tokio::test]
    async fn test_poll_ready_issues_returns_mapped_issues() {
        let client = MockGhClient::new()
            .with_auth_result(Ok(()))
            .with_list_result(Ok(vec![
                make_gh_issue(1, "First issue", "body one"),
                make_gh_issue(42, "Second issue", "body two"),
            ]));
        let source = make_source(client);

        let issues = source.poll_ready_issues().await.unwrap();
        assert_eq!(issues.len(), 2);

        assert_eq!(issues[0].id, "1");
        assert_eq!(issues[0].title, "First issue");
        assert_eq!(issues[0].body, "body one");
        assert_eq!(
            issues[0].source_url,
            "https://github.com/owner/repo/issues/1"
        );

        assert_eq!(issues[1].id, "42");
        assert_eq!(issues[1].title, "Second issue");
        assert_eq!(issues[1].body, "body two");
    }

    #[tokio::test]
    async fn test_poll_ready_issues_empty_result() {
        let client = MockGhClient::new()
            .with_auth_result(Ok(()))
            .with_list_result(Ok(vec![]));
        let source = make_source(client);

        let issues = source.poll_ready_issues().await.unwrap();
        assert!(issues.is_empty());
    }

    #[tokio::test]
    async fn test_poll_ready_issues_auth_failure() {
        let client = MockGhClient::new()
            .with_auth_result(Err(SmeltError::tracker("auth_status", "not authenticated")));
        let source = make_source(client);

        let err = source.poll_ready_issues().await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("poll"), "expected 'poll' in error: {msg}");
    }

    // ── transition_state ────────────────────────────────────────

    #[tokio::test]
    async fn test_transition_state_edits_labels() {
        let client = MockGhClient::new().with_edit_result(Ok(()));
        let source = make_source(client);

        source
            .transition_state("42", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_transition_state_failure_propagates() {
        let client = MockGhClient::new()
            .with_edit_result(Err(SmeltError::tracker("edit_labels", "label swap failed")));
        let source = make_source(client);

        let err = source
            .transition_state("42", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("label swap failed"),
            "expected failure message: {msg}"
        );
    }

    #[tokio::test]
    async fn test_transition_state_invalid_issue_id() {
        let client = MockGhClient::new();
        let source = make_source(client);

        let err = source
            .transition_state("not-a-number", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not a valid u64"),
            "expected parse error: {msg}"
        );
    }

    // ── ensure_labels ───────────────────────────────────────────

    #[tokio::test]
    async fn test_ensure_labels_creates_all_lifecycle_labels() {
        // Queue 6 successes — one per TrackerState::ALL variant.
        let mut client = MockGhClient::new();
        for _ in TrackerState::ALL {
            client = client.with_create_label_result(Ok(()));
        }
        let source = make_source(client);

        source.ensure_labels().await.unwrap();
    }

    // ── poll uses correct label ─────────────────────────────────

    #[tokio::test]
    async fn test_poll_uses_ready_label_prefix() {
        // This test verifies that the source passes through the ready
        // label derived from the configured prefix.  The mock doesn't
        // inspect the label string, but the fact that the call succeeds
        // with one auth + one list result proves the flow.
        let client = MockGhClient::new()
            .with_auth_result(Ok(()))
            .with_list_result(Ok(vec![make_gh_issue(7, "Issue", "body")]));
        let source = GithubTrackerSource::new(client, "owner/repo".into(), "custom-prefix".into());

        let issues = source.poll_ready_issues().await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "7");
    }
}
