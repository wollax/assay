//! Test double and unit tests for the GitHub client abstraction.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use smelt_core::error::SmeltError;

use super::{GhClient, GhIssue};

// -----------------------------------------------------------------------
// MockGhClient
// -----------------------------------------------------------------------

type GhResultQueue<T> = Arc<Mutex<VecDeque<Result<T, SmeltError>>>>;

/// Test double for `GhClient` with configurable pop-front results.
#[derive(Clone)]
pub(crate) struct MockGhClient {
    list_results: GhResultQueue<Vec<GhIssue>>,
    edit_results: GhResultQueue<()>,
    create_label_results: GhResultQueue<()>,
    auth_results: GhResultQueue<()>,
}

impl MockGhClient {
    /// Create a new `MockGhClient` with empty result queues.
    pub fn new() -> Self {
        Self {
            list_results: Arc::new(Mutex::new(VecDeque::new())),
            edit_results: Arc::new(Mutex::new(VecDeque::new())),
            create_label_results: Arc::new(Mutex::new(VecDeque::new())),
            auth_results: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Enqueue a list_issues result.
    pub fn with_list_result(self, result: Result<Vec<GhIssue>, SmeltError>) -> Self {
        self.list_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue an edit_labels result.
    pub fn with_edit_result(self, result: Result<(), SmeltError>) -> Self {
        self.edit_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue a create_label result.
    pub fn with_create_label_result(self, result: Result<(), SmeltError>) -> Self {
        self.create_label_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue an auth_status result.
    pub fn with_auth_result(self, result: Result<(), SmeltError>) -> Self {
        self.auth_results.lock().unwrap().push_back(result);
        self
    }
}

impl GhClient for MockGhClient {
    async fn list_issues(
        &self,
        _repo: &str,
        _label: &str,
        _limit: u32,
    ) -> Result<Vec<GhIssue>, SmeltError> {
        self.list_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "list_issues",
                    "MockGhClient: no list_issues results configured",
                ))
            })
    }

    async fn edit_labels(
        &self,
        _repo: &str,
        _number: u64,
        _add_labels: &[&str],
        _remove_labels: &[&str],
    ) -> Result<(), SmeltError> {
        self.edit_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "edit_labels",
                    "MockGhClient: no edit_labels results configured",
                ))
            })
    }

    async fn create_label(&self, _repo: &str, _name: &str) -> Result<(), SmeltError> {
        self.create_label_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "create_label",
                    "MockGhClient: no create_label results configured",
                ))
            })
    }

    async fn auth_status(&self) -> Result<(), SmeltError> {
        self.auth_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "auth_status",
                    "MockGhClient: no auth_status results configured",
                ))
            })
    }
}

// -----------------------------------------------------------------------
// Unit tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_list_issues_returns_queued_results() {
        let issues = vec![
            GhIssue {
                number: 1,
                title: "First".to_string(),
                body: "body1".to_string(),
                url: "https://github.com/owner/repo/issues/1".to_string(),
            },
            GhIssue {
                number: 2,
                title: "Second".to_string(),
                body: "body2".to_string(),
                url: "https://github.com/owner/repo/issues/2".to_string(),
            },
        ];
        let client = MockGhClient::new().with_list_result(Ok(issues.clone()));

        let result = client.list_issues("owner/repo", "smelt:ready", 10).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), issues);
    }

    #[tokio::test]
    async fn test_mock_edit_labels_returns_queued_results() {
        let client = MockGhClient::new().with_edit_result(Ok(()));

        let result = client
            .edit_labels("owner/repo", 42, &["smelt:queued"], &["smelt:ready"])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_create_label_returns_queued_results() {
        let client = MockGhClient::new().with_create_label_result(Ok(()));

        let result = client.create_label("owner/repo", "smelt:ready").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_auth_status_returns_queued_results() {
        let client = MockGhClient::new().with_auth_result(Ok(()));

        let result = client.auth_status().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_exhausted_queue_returns_error() {
        let client = MockGhClient::new();

        let list_result = client.list_issues("owner/repo", "label", 10).await;
        assert!(list_result.is_err());
        let err = list_result.unwrap_err().to_string();
        assert!(
            err.contains("no list_issues results configured"),
            "unexpected error: {err}"
        );

        let edit_result = client.edit_labels("owner/repo", 1, &["a"], &["b"]).await;
        assert!(edit_result.is_err());

        let label_result = client.create_label("owner/repo", "label").await;
        assert!(label_result.is_err());

        let auth_result = client.auth_status().await;
        assert!(auth_result.is_err());
    }

    #[test]
    fn test_gh_issue_deserialize_from_json() {
        let json = r#"[
            {
                "number": 42,
                "title": "Fix the bug",
                "body": "This is broken",
                "url": "https://github.com/owner/repo/issues/42"
            },
            {
                "number": 43,
                "title": "Add feature",
                "body": "",
                "url": "https://github.com/owner/repo/issues/43"
            }
        ]"#;

        let issues: Vec<GhIssue> = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 42);
        assert_eq!(issues[0].title, "Fix the bug");
        assert_eq!(issues[0].body, "This is broken");
        assert_eq!(issues[1].number, 43);
        assert_eq!(issues[1].body, "");
    }

    #[test]
    fn test_subprocess_gh_client_binary_discovery() {
        // This test verifies the which::which call path. It may fail in
        // environments where `gh` is not installed — that's expected.
        use super::super::client::SubprocessGhClient;

        // We can't call the private gh_binary() directly, so we verify
        // the public type exists and implements GhClient.
        let _client = SubprocessGhClient;

        // Verify which::which("gh") at least runs without panic.
        // The result depends on whether gh is installed.
        let _result = which::which("gh");
    }

    /// Compile-test: verify a dummy type can implement GhClient.
    #[test]
    fn test_gh_client_trait_compiles() {
        struct DummyGhClient;

        impl GhClient for DummyGhClient {
            async fn list_issues(
                &self,
                _repo: &str,
                _label: &str,
                _limit: u32,
            ) -> Result<Vec<GhIssue>, SmeltError> {
                Ok(vec![])
            }

            async fn edit_labels(
                &self,
                _repo: &str,
                _number: u64,
                _add_labels: &[&str],
                _remove_labels: &[&str],
            ) -> Result<(), SmeltError> {
                Ok(())
            }

            async fn create_label(&self, _repo: &str, _name: &str) -> Result<(), SmeltError> {
                Ok(())
            }

            async fn auth_status(&self) -> Result<(), SmeltError> {
                Ok(())
            }
        }

        let _client = DummyGhClient;
    }
}
