//! Linear client abstraction for interacting with Linear Issues via GraphQL API.
//!
//! # Design
//!
//! - `LinearClient` is a trait with async methods using RPITIT — not object-safe.
//!   Use generics (`impl LinearClient` / `<L: LinearClient>`) at call sites.
//! - `ReqwestLinearClient` sends GraphQL requests via async `reqwest::Client`
//!   with `Authorization` header auth (async, not blocking, because the serve
//!   loop is tokio-based).
//!
//! # Testing
//!
//! `MockLinearClient` (available in `#[cfg(test)]` only) is a VecDeque-based
//! test double matching the `MockGhClient` pattern for deterministic unit testing.

/// Production `reqwest`-based Linear client implementation.
pub mod client;

/// `LinearTrackerSource` — bridges `LinearClient` to `TrackerSource`.
pub mod source;

#[cfg(test)]
pub(crate) mod mock;

// Re-export public API.
pub use client::ReqwestLinearClient;
pub use source::LinearTrackerSource;

use serde::Deserialize;
use smelt_core::error::SmeltError;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A Linear issue as returned by the GraphQL API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LinearIssue {
    /// Linear issue UUID.
    pub id: String,
    /// Human-readable identifier (e.g. "KAT-42").
    pub identifier: String,
    /// Issue title.
    pub title: String,
    /// Issue description / body (may be empty or null).
    #[serde(default)]
    pub description: String,
    /// Web URL to view the issue.
    pub url: String,
}

/// A Linear label.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LinearLabel {
    /// Linear label UUID.
    pub id: String,
    /// Label name.
    pub name: String,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Async Linear client abstraction.
///
/// # Object safety
///
/// This trait is intentionally **not** object-safe (it uses `async fn` with
/// RPITIT). Use `impl LinearClient` / `<L: LinearClient>` at call sites rather
/// than `dyn LinearClient`.
///
/// All async methods return `Send` futures so they can be spawned on the tokio
/// runtime.
pub trait LinearClient {
    /// List issues matching `label_name` in `team_id`.
    fn list_issues(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> impl std::future::Future<Output = Result<Vec<LinearIssue>, SmeltError>> + Send;

    /// Add a label to an issue by UUIDs.
    fn add_label(
        &self,
        issue_id: &str,
        label_id: &str,
    ) -> impl std::future::Future<Output = Result<(), SmeltError>> + Send;

    /// Remove a label from an issue by UUIDs.
    fn remove_label(
        &self,
        issue_id: &str,
        label_id: &str,
    ) -> impl std::future::Future<Output = Result<(), SmeltError>> + Send;

    /// Find a label by name in a team.
    fn find_label(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> impl std::future::Future<Output = Result<Option<LinearLabel>, SmeltError>> + Send;

    /// Create a label in a team.
    fn create_label(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> impl std::future::Future<Output = Result<LinearLabel, SmeltError>> + Send;
}

// ---------------------------------------------------------------------------
// Compile-test: verify a dummy type can implement LinearClient.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod compile_tests {
    use super::*;

    #[test]
    fn test_linear_client_trait_compiles() {
        struct DummyLinearClient;

        impl LinearClient for DummyLinearClient {
            async fn list_issues(
                &self,
                _team_id: &str,
                _label_name: &str,
            ) -> Result<Vec<LinearIssue>, SmeltError> {
                Ok(vec![])
            }

            async fn add_label(&self, _issue_id: &str, _label_id: &str) -> Result<(), SmeltError> {
                Ok(())
            }

            async fn remove_label(
                &self,
                _issue_id: &str,
                _label_id: &str,
            ) -> Result<(), SmeltError> {
                Ok(())
            }

            async fn find_label(
                &self,
                _team_id: &str,
                _label_name: &str,
            ) -> Result<Option<LinearLabel>, SmeltError> {
                Ok(None)
            }

            async fn create_label(
                &self,
                _team_id: &str,
                _label_name: &str,
            ) -> Result<LinearLabel, SmeltError> {
                Ok(LinearLabel {
                    id: "dummy".to_string(),
                    name: "dummy".to_string(),
                })
            }
        }

        let _client = DummyLinearClient;
    }
}
