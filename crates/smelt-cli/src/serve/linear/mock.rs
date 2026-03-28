//! Test double and unit tests for the Linear client abstraction.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use smelt_core::error::SmeltError;

use super::{LinearClient, LinearIssue, LinearLabel};

// -----------------------------------------------------------------------
// MockLinearClient
// -----------------------------------------------------------------------

type ResultQueue<T> = Arc<Mutex<VecDeque<Result<T, SmeltError>>>>;

/// Test double for `LinearClient` with configurable pop-front results.
#[derive(Clone)]
pub(crate) struct MockLinearClient {
    list_results: ResultQueue<Vec<LinearIssue>>,
    add_label_results: ResultQueue<()>,
    remove_label_results: ResultQueue<()>,
    find_label_results: ResultQueue<Option<LinearLabel>>,
    create_label_results: ResultQueue<LinearLabel>,
}

impl MockLinearClient {
    /// Create a new `MockLinearClient` with empty result queues.
    pub fn new() -> Self {
        Self {
            list_results: Arc::new(Mutex::new(VecDeque::new())),
            add_label_results: Arc::new(Mutex::new(VecDeque::new())),
            remove_label_results: Arc::new(Mutex::new(VecDeque::new())),
            find_label_results: Arc::new(Mutex::new(VecDeque::new())),
            create_label_results: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Enqueue a list_issues result.
    pub fn with_list_result(self, result: Result<Vec<LinearIssue>, SmeltError>) -> Self {
        self.list_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue a find_label result.
    pub fn with_find_label_result(self, result: Result<Option<LinearLabel>, SmeltError>) -> Self {
        self.find_label_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue a create_label result.
    pub fn with_create_label_result(self, result: Result<LinearLabel, SmeltError>) -> Self {
        self.create_label_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue an add_label result.
    pub fn with_add_label_result(self, result: Result<(), SmeltError>) -> Self {
        self.add_label_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue a remove_label result.
    pub fn with_remove_label_result(self, result: Result<(), SmeltError>) -> Self {
        self.remove_label_results.lock().unwrap().push_back(result);
        self
    }
}

impl LinearClient for MockLinearClient {
    async fn list_issues(
        &self,
        _team_id: &str,
        _label_name: &str,
    ) -> Result<Vec<LinearIssue>, SmeltError> {
        self.list_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "list_issues",
                    "MockLinearClient: no list_issues results configured",
                ))
            })
    }

    async fn add_label(&self, _issue_id: &str, _label_id: &str) -> Result<(), SmeltError> {
        self.add_label_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "add_label",
                    "MockLinearClient: no add_label results configured",
                ))
            })
    }

    async fn remove_label(&self, _issue_id: &str, _label_id: &str) -> Result<(), SmeltError> {
        self.remove_label_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "remove_label",
                    "MockLinearClient: no remove_label results configured",
                ))
            })
    }

    async fn find_label(
        &self,
        _team_id: &str,
        _label_name: &str,
    ) -> Result<Option<LinearLabel>, SmeltError> {
        self.find_label_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "find_label",
                    "MockLinearClient: no find_label results configured",
                ))
            })
    }

    async fn create_label(
        &self,
        _team_id: &str,
        _label_name: &str,
    ) -> Result<LinearLabel, SmeltError> {
        self.create_label_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(SmeltError::tracker(
                    "create_label",
                    "MockLinearClient: no create_label results configured",
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
            LinearIssue {
                id: "uuid-1".to_string(),
                identifier: "KAT-42".to_string(),
                title: "First".to_string(),
                description: "body1".to_string(),
                url: "https://linear.app/team/issue/KAT-42".to_string(),
            },
            LinearIssue {
                id: "uuid-2".to_string(),
                identifier: "KAT-43".to_string(),
                title: "Second".to_string(),
                description: "body2".to_string(),
                url: "https://linear.app/team/issue/KAT-43".to_string(),
            },
        ];
        let client = MockLinearClient::new().with_list_result(Ok(issues.clone()));

        let result = client.list_issues("team-id", "smelt:ready").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), issues);
    }

    #[tokio::test]
    async fn test_mock_add_label_returns_queued_results() {
        let client = MockLinearClient::new().with_add_label_result(Ok(()));

        let result = client.add_label("issue-uuid", "label-uuid").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_remove_label_returns_queued_results() {
        let client = MockLinearClient::new().with_remove_label_result(Ok(()));

        let result = client.remove_label("issue-uuid", "label-uuid").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_find_label_returns_queued_results() {
        let label = LinearLabel {
            id: "label-uuid".to_string(),
            name: "smelt:ready".to_string(),
        };
        let client = MockLinearClient::new().with_find_label_result(Ok(Some(label.clone())));

        let result = client.find_label("team-id", "smelt:ready").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(label));
    }

    #[tokio::test]
    async fn test_mock_create_label_returns_queued_results() {
        let label = LinearLabel {
            id: "new-label-uuid".to_string(),
            name: "smelt:ready".to_string(),
        };
        let client = MockLinearClient::new().with_create_label_result(Ok(label.clone()));

        let result = client.create_label("team-id", "smelt:ready").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), label);
    }

    #[tokio::test]
    async fn test_mock_exhausted_queue_returns_error() {
        let client = MockLinearClient::new();

        let list_result = client.list_issues("team-id", "label").await;
        assert!(list_result.is_err());
        let err = list_result.unwrap_err().to_string();
        assert!(
            err.contains("no list_issues results configured"),
            "unexpected error: {err}"
        );

        let add_result = client.add_label("issue", "label").await;
        assert!(add_result.is_err());

        let remove_result = client.remove_label("issue", "label").await;
        assert!(remove_result.is_err());

        let find_result = client.find_label("team", "label").await;
        assert!(find_result.is_err());

        let create_result = client.create_label("team", "label").await;
        assert!(create_result.is_err());
    }

    #[test]
    fn test_linear_issue_deserialize_from_json() {
        let json = r#"[
            {
                "id": "uuid-1",
                "identifier": "KAT-42",
                "title": "Fix the bug",
                "description": "This is broken",
                "url": "https://linear.app/team/issue/KAT-42"
            },
            {
                "id": "uuid-2",
                "identifier": "KAT-43",
                "title": "Add feature",
                "url": "https://linear.app/team/issue/KAT-43"
            }
        ]"#;

        let issues: Vec<LinearIssue> = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "uuid-1");
        assert_eq!(issues[0].identifier, "KAT-42");
        assert_eq!(issues[0].title, "Fix the bug");
        assert_eq!(issues[0].description, "This is broken");
        assert_eq!(issues[1].id, "uuid-2");
        // description defaults to empty string when missing
        assert_eq!(issues[1].description, "");
    }

    #[test]
    fn test_linear_label_deserialize_from_json() {
        let json = r#"{"id": "label-uuid", "name": "smelt:ready"}"#;
        let label: LinearLabel = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(label.id, "label-uuid");
        assert_eq!(label.name, "smelt:ready");
    }
}
