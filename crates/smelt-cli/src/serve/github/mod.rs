//! GitHub client abstraction for interacting with GitHub Issues via the `gh` CLI.
//!
//! # Design
//!
//! - `GhClient` is a trait with async methods using RPITIT (per D019) — not
//!   object-safe. Use generics (`impl GhClient` / `<C: GhClient>`) at call sites.
//! - `SubprocessGhClient` shells out to the system `gh` binary via
//!   `tokio::process::Command`, discovered at runtime via `which::which`.
//! - `MockGhClient` is a VecDeque-based test double matching the `MockSshClient`
//!   pattern for deterministic unit testing.

/// Subprocess-based `gh` CLI client implementation.
pub mod client;

/// `GithubTrackerSource` — bridges `GhClient` to `TrackerSource`.
pub mod source;

#[cfg(test)]
pub(crate) mod mock;

// Re-export public API.
pub use client::SubprocessGhClient;
pub use source::GithubTrackerSource;

use serde::Deserialize;
use smelt_core::error::SmeltError;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// A GitHub issue as returned by `gh issue list --json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GhIssue {
    /// Issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body / description (may be empty).
    pub body: String,
    /// Web URL to view the issue.
    pub url: String,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Async GitHub client abstraction.
///
/// # Object safety
///
/// This trait is intentionally **not** object-safe (it uses `async fn` with
/// RPITIT). Use `impl GhClient` / `<C: GhClient>` at call sites rather than
/// `dyn GhClient`.
///
/// All async methods return `Send` futures so they can be spawned on the tokio
/// runtime.
pub trait GhClient {
    /// List issues from `repo` matching `label`, returning at most `limit` results.
    fn list_issues(
        &self,
        repo: &str,
        label: &str,
        limit: u32,
    ) -> impl std::future::Future<Output = Result<Vec<GhIssue>, SmeltError>> + Send;

    /// Edit labels on issue `number` in `repo`: add `add_labels` and remove
    /// `remove_labels`.
    fn edit_labels(
        &self,
        repo: &str,
        number: u64,
        add_labels: &[&str],
        remove_labels: &[&str],
    ) -> impl std::future::Future<Output = Result<(), SmeltError>> + Send;

    /// Create a label named `name` in `repo` (idempotent via `--force`).
    fn create_label(
        &self,
        repo: &str,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), SmeltError>> + Send;

    /// Verify that `gh` is authenticated and can reach GitHub.
    fn auth_status(&self) -> impl std::future::Future<Output = Result<(), SmeltError>> + Send;
}

// ---------------------------------------------------------------------------
// Integration tests — gated by SMELT_GH_TEST=1 and SMELT_GH_REPO
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Return `(gh_test, repo)` from env, or `None` if the gate vars are not set.
    fn gate() -> Option<String> {
        let enabled = std::env::var("SMELT_GH_TEST").unwrap_or_default();
        if enabled != "1" {
            eprintln!("SMELT_GH_TEST not set to 1 — skipping integration test");
            return None;
        }
        match std::env::var("SMELT_GH_REPO") {
            Ok(r) if !r.is_empty() => Some(r),
            _ => {
                eprintln!("SMELT_GH_REPO not set — skipping integration test");
                None
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_gh_auth_status_integration() {
        if gate().is_none() {
            return;
        }
        let client = SubprocessGhClient;
        client
            .auth_status()
            .await
            .expect("gh auth status should succeed when SMELT_GH_TEST=1");
    }

    #[tokio::test]
    #[ignore]
    async fn test_gh_list_issues_integration() {
        let repo = match gate() {
            Some(r) => r,
            None => return,
        };
        let client = SubprocessGhClient;
        let issues = client
            .list_issues(&repo, "nonexistent-label-xyz", 5)
            .await
            .expect("gh issue list should succeed even with no matching issues");
        assert!(
            issues.is_empty(),
            "expected no issues with label 'nonexistent-label-xyz', got {}",
            issues.len()
        );
    }
}
