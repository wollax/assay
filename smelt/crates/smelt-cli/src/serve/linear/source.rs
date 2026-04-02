//! `LinearTrackerSource` — bridges `LinearClient` to `TrackerSource`.
//!
//! Generic over `L: LinearClient` for testability with `MockLinearClient`.
//! Label mutations in Linear require UUIDs, so `ensure_labels()` resolves
//! label name → UUID mappings into a cached `HashMap` at startup.

use std::collections::HashMap;

use smelt_core::error::SmeltError;
use smelt_core::tracker::{TrackerIssue, TrackerState};

use super::LinearClient;
use crate::serve::tracker::TrackerSource;

/// A tracker source that polls Linear Issues via a `LinearClient` implementation
/// and manages label-based lifecycle transitions.
///
/// Generic over `L: LinearClient` so unit tests can inject `MockLinearClient`.
///
/// Unlike `GithubTrackerSource`, Linear label mutations require UUIDs (not
/// names). Call [`ensure_labels`](Self::ensure_labels) once at startup to
/// populate the label UUID cache before `transition_state`.
pub struct LinearTrackerSource<L: LinearClient> {
    client: L,
    team_id: String,
    label_prefix: String,
    /// Label name → UUID cache, populated by `ensure_labels()`.
    label_cache: HashMap<String, String>,
}

impl<L: LinearClient> LinearTrackerSource<L> {
    /// Create a new `LinearTrackerSource` with an empty label cache.
    ///
    /// Call [`ensure_labels`](Self::ensure_labels) before `transition_state`
    /// to populate the label UUID cache.
    pub fn new(client: L, team_id: String, label_prefix: String) -> Self {
        Self {
            client,
            team_id,
            label_prefix,
            label_cache: HashMap::new(),
        }
    }

    /// Ensure all lifecycle labels exist in the Linear team and cache their UUIDs.
    ///
    /// For each variant in `TrackerState::ALL`, queries for the label by name.
    /// If found, caches the UUID. If not found, creates it and caches the
    /// returned UUID. Must be called before `transition_state`.
    ///
    /// On partial failure, the existing cache is left untouched — the new
    /// cache is only swapped in on complete success.
    pub async fn ensure_labels(&mut self) -> anyhow::Result<()> {
        let mut new_cache = HashMap::new();
        for state in TrackerState::ALL {
            let label_name = state.label_name(&self.label_prefix);

            let label = self
                .client
                .find_label(&self.team_id, &label_name)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("ensure_labels failed for label '{label_name}': {e}")
                })?;

            let (uuid, action) = match label {
                Some(existing) => (existing.id, "found"),
                None => {
                    let created = self
                        .client
                        .create_label(&self.team_id, &label_name)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "ensure_labels failed creating label '{label_name}': {e}"
                            )
                        })?;
                    (created.id, "created")
                }
            };

            tracing::info!(
                team_id = %self.team_id,
                label = %label_name,
                action = action,
                "ensured lifecycle label"
            );

            new_cache.insert(label_name, uuid);
        }
        // Only replace cache on complete success.
        self.label_cache = new_cache;
        Ok(())
    }

    /// Look up a label UUID from the cache, returning a descriptive error if missing.
    fn cached_label_uuid(&self, label_name: &str) -> anyhow::Result<&str> {
        self.label_cache
            .get(label_name)
            .map(|s| s.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!(SmeltError::tracker(
                    "transition",
                    format!("label '{label_name}' not in cache — was ensure_labels() called?"),
                ))
            })
    }
}

impl<L: LinearClient + Send + Sync> TrackerSource for LinearTrackerSource<L> {
    async fn poll_ready_issues(&self) -> anyhow::Result<Vec<TrackerIssue>> {
        let ready_label = TrackerState::Ready.label_name(&self.label_prefix);
        let linear_issues = self
            .client
            .list_issues(&self.team_id, &ready_label)
            .await
            .map_err(anyhow::Error::from)?;

        let issues = linear_issues
            .into_iter()
            .map(|li| TrackerIssue {
                // Use Linear UUID as id — mutations need UUIDs, and consumers
                // treat id as opaque.
                id: li.id,
                title: li.title,
                body: li.description,
                source_url: li.url,
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
        let from_label_name = from.label_name(&self.label_prefix);
        let to_label_name = to.label_name(&self.label_prefix);

        let from_label_uuid = self.cached_label_uuid(&from_label_name)?;
        let to_label_uuid = self.cached_label_uuid(&to_label_name)?;

        // Remove old label first, then add new. Not atomic — if add_label
        // fails after remove_label succeeds, the issue will have neither
        // label. See S04 forward intelligence for recovery considerations.
        self.client
            .remove_label(issue_id, from_label_uuid)
            .await
            .map_err(anyhow::Error::from)?;

        if let Err(e) = self.client.add_label(issue_id, to_label_uuid).await {
            tracing::error!(
                issue_id = %issue_id,
                from_label = %from_label_name,
                to_label = %to_label_name,
                error = %e,
                "CRITICAL: remove_label succeeded but add_label failed — issue is now label-less in Linear"
            );
            return Err(anyhow::Error::from(e));
        }

        tracing::info!(
            issue_id = %issue_id,
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
    use crate::serve::linear::mock::MockLinearClient;
    use crate::serve::linear::{LinearIssue, LinearLabel};

    fn make_source(client: MockLinearClient) -> LinearTrackerSource<MockLinearClient> {
        LinearTrackerSource::new(client, "team-uuid".to_string(), "smelt".to_string())
    }

    fn make_linear_issue(id: &str, identifier: &str, title: &str, desc: &str) -> LinearIssue {
        LinearIssue {
            id: id.to_string(),
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: desc.to_string(),
            url: format!("https://linear.app/team/issue/{identifier}"),
        }
    }

    fn make_label(id: &str, name: &str) -> LinearLabel {
        LinearLabel {
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    /// Build a source with all 6 lifecycle labels pre-cached.
    fn make_source_with_cache(client: MockLinearClient) -> LinearTrackerSource<MockLinearClient> {
        let mut source = make_source(client);
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            source
                .label_cache
                .insert(label_name, format!("{}-label-uuid", state));
        }
        source
    }

    // ── poll_ready_issues ───────────────────────────────────────

    #[tokio::test]
    async fn test_poll_ready_issues_returns_mapped_issues() {
        let client = MockLinearClient::new().with_list_result(Ok(vec![
            make_linear_issue("uuid-1", "KAT-42", "First issue", "body one"),
            make_linear_issue("uuid-2", "KAT-43", "Second issue", "body two"),
        ]));
        let source = make_source(client);

        let issues = source.poll_ready_issues().await.unwrap();
        assert_eq!(issues.len(), 2);

        // id should be the Linear UUID, not the identifier
        assert_eq!(issues[0].id, "uuid-1");
        assert_eq!(issues[0].title, "First issue");
        assert_eq!(issues[0].body, "body one");
        assert_eq!(issues[0].source_url, "https://linear.app/team/issue/KAT-42");

        assert_eq!(issues[1].id, "uuid-2");
        assert_eq!(issues[1].title, "Second issue");
        assert_eq!(issues[1].body, "body two");
    }

    #[tokio::test]
    async fn test_poll_ready_issues_empty_result() {
        let client = MockLinearClient::new().with_list_result(Ok(vec![]));
        let source = make_source(client);

        let issues = source.poll_ready_issues().await.unwrap();
        assert!(issues.is_empty());
    }

    #[tokio::test]
    async fn test_poll_ready_issues_list_failure() {
        let client = MockLinearClient::new().with_list_result(Err(SmeltError::tracker(
            "list_issues",
            "GraphQL error: unauthorized",
        )));
        let source = make_source(client);

        let err = source.poll_ready_issues().await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("list_issues"),
            "expected 'list_issues' in error: {msg}"
        );
        assert!(
            msg.contains("unauthorized"),
            "expected 'unauthorized' in error: {msg}"
        );
    }

    // ── transition_state ────────────────────────────────────────

    #[tokio::test]
    async fn test_transition_state_removes_old_adds_new() {
        let client = MockLinearClient::new()
            .with_remove_label_result(Ok(()))
            .with_add_label_result(Ok(()));
        let source = make_source_with_cache(client);

        source
            .transition_state("issue-uuid", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_transition_state_missing_cache_entry() {
        // Source with NO label cache — ensure_labels() not called
        let client = MockLinearClient::new();
        let source = make_source(client);

        let err = source
            .transition_state("issue-uuid", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not in cache"),
            "expected cache miss error: {msg}"
        );
        assert!(
            msg.contains("ensure_labels()"),
            "expected hint about ensure_labels: {msg}"
        );
    }

    #[tokio::test]
    async fn test_transition_state_remove_failure_propagates() {
        let client = MockLinearClient::new().with_remove_label_result(Err(SmeltError::tracker(
            "remove_label",
            "API error: not found",
        )));
        let source = make_source_with_cache(client);

        let err = source
            .transition_state("issue-uuid", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("remove_label"),
            "expected 'remove_label' in error: {msg}"
        );
    }

    #[tokio::test]
    async fn test_transition_state_add_failure_propagates() {
        let client = MockLinearClient::new()
            .with_remove_label_result(Ok(())) // remove succeeds
            .with_add_label_result(Err(SmeltError::tracker(
                "add_label",
                "API error: rate limited",
            )));
        let source = make_source_with_cache(client);

        let err = source
            .transition_state("issue-uuid", TrackerState::Ready, TrackerState::Queued)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("add_label"),
            "expected 'add_label' in error: {msg}"
        );
    }

    // ── ensure_labels ───────────────────────────────────────────

    #[tokio::test]
    async fn test_ensure_labels_creates_missing_labels() {
        // All 6 labels not found → create each
        let mut client = MockLinearClient::new();
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            client = client
                .with_find_label_result(Ok(None))
                .with_create_label_result(Ok(make_label(&format!("{}-uuid", state), &label_name)));
        }
        let mut source = make_source(client);

        source.ensure_labels().await.unwrap();

        // Verify cache has all 6
        assert_eq!(source.label_cache.len(), 6);
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            assert!(
                source.label_cache.contains_key(&label_name),
                "missing cache entry for {label_name}"
            );
            assert_eq!(source.label_cache[&label_name], format!("{}-uuid", state));
        }
    }

    #[tokio::test]
    async fn test_ensure_labels_finds_existing_labels() {
        // All 6 labels already exist → no create calls needed
        let mut client = MockLinearClient::new();
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            client = client.with_find_label_result(Ok(Some(make_label(
                &format!("existing-{}-uuid", state),
                &label_name,
            ))));
        }
        let mut source = make_source(client);

        source.ensure_labels().await.unwrap();

        assert_eq!(source.label_cache.len(), 6);
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            assert_eq!(
                source.label_cache[&label_name],
                format!("existing-{}-uuid", state)
            );
        }
    }

    #[tokio::test]
    async fn test_ensure_labels_populates_cache() {
        // Mix: first 3 found, last 3 created
        let states: Vec<_> = TrackerState::ALL.to_vec();
        let mut client = MockLinearClient::new();
        for (i, state) in states.iter().enumerate() {
            let label_name = state.label_name("smelt");
            if i < 3 {
                client = client.with_find_label_result(Ok(Some(make_label(
                    &format!("found-{i}"),
                    &label_name,
                ))));
            } else {
                client = client
                    .with_find_label_result(Ok(None))
                    .with_create_label_result(Ok(make_label(&format!("created-{i}"), &label_name)));
            }
        }
        let mut source = make_source(client);

        source.ensure_labels().await.unwrap();

        // All 6 lifecycle labels must be in cache
        assert_eq!(source.label_cache.len(), 6);
        for state in TrackerState::ALL {
            let label_name = state.label_name("smelt");
            assert!(
                source.label_cache.contains_key(&label_name),
                "missing: {label_name}"
            );
        }
        // Spot-check the UUIDs
        assert_eq!(
            source.label_cache[&TrackerState::Ready.label_name("smelt")],
            "found-0"
        );
        assert_eq!(
            source.label_cache[&TrackerState::Failed.label_name("smelt")],
            "created-5"
        );
    }
}
