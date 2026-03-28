//! LinearBackend — persists orchestrator state as Linear issues and comments.
//!
//! `LinearClient` wraps `reqwest::blocking::Client` for synchronous GraphQL
//! calls against the Linear API. `LinearBackend` implements [`StateBackend`]
//! by creating an issue per `run_dir` on the first `push_session_event`,
//! appending comments on subsequent calls, and reading the latest comment
//! body for `read_run_state`.
//!
//! The issue ID is cached in a `.linear-issue-id` file inside `run_dir`.

use std::path::Path;

use assay_core::{AssayError, CapabilitySet, StateBackend};
use assay_types::{OrchestratorStatus, TeamCheckpoint};
use serde_json::Value;

// ---------------------------------------------------------------------------
// LinearClient — low-level GraphQL HTTP wrapper
// ---------------------------------------------------------------------------

/// Low-level HTTP wrapper around the Linear GraphQL API.
///
/// All methods use `reqwest::blocking::Client`, which internally manages
/// its own tokio runtime — no nested-runtime risk (see D161).
struct LinearClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl LinearClient {
    /// Build a new client with the given API key and endpoint.
    ///
    /// `base_url` is the root URL *without* the `/graphql` path,
    /// e.g. `"https://api.linear.app"`. The `/graphql` suffix is
    /// appended automatically.
    fn new(api_key: &str, base_url: String) -> assay_core::Result<Self> {
        use reqwest::header;

        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(api_key).map_err(|e| {
            AssayError::io(
                "constructing Authorization header",
                "LINEAR_API_KEY",
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()),
            )
        })?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| {
                AssayError::io(
                    "building reqwest client",
                    "LINEAR_API_KEY",
                    std::io::Error::other(e.to_string()),
                )
            })?;

        let graphql_url = format!("{}/graphql", base_url.trim_end_matches('/'));
        Ok(Self {
            client,
            base_url: graphql_url,
        })
    }

    /// Execute a GraphQL request and return the parsed JSON body.
    ///
    /// Checks for GraphQL-level errors (200 with `"errors"` array) and
    /// surfaces them as [`AssayError::Io`].
    fn graphql(&self, body: &Value) -> assay_core::Result<Value> {
        tracing::debug!(url = %self.base_url, "sending GraphQL request");

        let resp = self
            .client
            .post(&self.base_url)
            .json(body)
            .send()
            .map_err(|e| {
                AssayError::io(
                    "sending GraphQL request",
                    &self.base_url,
                    std::io::Error::other(e.to_string()),
                )
            })?;

        let status = resp.status();
        let json: Value = resp.json().map_err(|e| {
            AssayError::io(
                "parsing GraphQL response",
                &self.base_url,
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
            )
        })?;

        // GraphQL errors arrive as 200 with an `errors` array.
        if let Some(errors) = json.get("errors") {
            let msg = errors
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown GraphQL error");
            tracing::warn!(error = %msg, "GraphQL error response");
            return Err(AssayError::io(
                format!("GraphQL error: {msg}"),
                &self.base_url,
                std::io::Error::other(msg.to_string()),
            ));
        }

        if !status.is_success() {
            return Err(AssayError::io(
                format!("Linear API returned HTTP {status}"),
                &self.base_url,
                std::io::Error::other(format!("HTTP {status}")),
            ));
        }

        Ok(json)
    }

    /// Create an issue and return its ID.
    fn create_issue(
        &self,
        team_id: &str,
        title: &str,
        description: &str,
    ) -> assay_core::Result<String> {
        let body = serde_json::json!({
            "query": "mutation CreateIssue($input: IssueCreateInput!) { issueCreate(input: $input) { success issue { id } } }",
            "variables": {
                "input": {
                    "teamId": team_id,
                    "title": title,
                    "description": description,
                }
            }
        });

        let json = self.graphql(&body)?;

        let issue_id = json
            .pointer("/data/issueCreate/issue/id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AssayError::io(
                    "extracting issue ID from issueCreate response",
                    &self.base_url,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unexpected response shape: {json}"),
                    ),
                )
            })?;

        tracing::info!(issue_id, "created Linear issue");
        Ok(issue_id.to_string())
    }

    /// Create a comment on an issue.
    fn create_comment(&self, issue_id: &str, body_text: &str) -> assay_core::Result<()> {
        let body = serde_json::json!({
            "query": "mutation CreateComment($input: CommentCreateInput!) { commentCreate(input: $input) { success comment { id body } } }",
            "variables": {
                "input": {
                    "issueId": issue_id,
                    "body": body_text,
                }
            }
        });

        self.graphql(&body)?;
        tracing::info!(issue_id, "created Linear comment");
        Ok(())
    }

    /// Fetch the latest comment body for an issue, or `None` if no comments.
    fn get_latest_comment(&self, issue_id: &str) -> assay_core::Result<Option<String>> {
        let body = serde_json::json!({
            "query": "query GetIssueComments($id: String!) { issue(id: $id) { comments(last: 1) { nodes { body } } } }",
            "variables": { "id": issue_id }
        });

        let json = self.graphql(&body)?;

        let nodes = json.pointer("/data/issue/comments/nodes");
        let comment_body = nodes
            .and_then(|n| n.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("body"))
            .and_then(|b| b.as_str());

        Ok(comment_body.map(|s| s.to_string()))
    }
}

// ---------------------------------------------------------------------------
// LinearBackend
// ---------------------------------------------------------------------------

/// Remote backend that persists orchestrator state as Linear issues/comments.
///
/// Each `run_dir` maps to one Linear issue. The issue ID is cached locally in
/// `run_dir/.linear-issue-id`. On the first `push_session_event` for a
/// `run_dir`, an issue is created. Subsequent calls append a comment with the
/// serialized `OrchestratorStatus` JSON.
///
/// `read_run_state` fetches the latest comment and deserializes it.
pub struct LinearBackend {
    client: LinearClient,
    team_id: String,
    #[allow(dead_code)]
    project_id: Option<String>,
}

impl std::fmt::Debug for LinearBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinearBackend")
            .field("team_id", &self.team_id)
            .field("project_id", &self.project_id)
            .field("base_url", &self.client.base_url)
            .finish()
    }
}

impl LinearBackend {
    /// Construct a `LinearBackend` with explicit credentials.
    ///
    /// `base_url` should be the full GraphQL endpoint,
    /// e.g. `"https://api.linear.app/graphql"` or a mockito server URL.
    pub fn new(
        api_key: String,
        base_url: String,
        team_id: String,
        project_id: Option<String>,
    ) -> Self {
        // Unwrap is acceptable here: the only failure mode is an invalid
        // header value, which means the key contains non-ASCII bytes —
        // Linear API keys are always ASCII.
        let client = LinearClient::new(&api_key, base_url)
            .expect("LinearClient construction should not fail with a valid API key");
        Self {
            client,
            team_id,
            project_id,
        }
    }

    /// Construct a `LinearBackend` by reading `LINEAR_API_KEY` from the
    /// environment.
    ///
    /// Returns an error if `LINEAR_API_KEY` is not set.
    pub fn from_env(
        base_url: String,
        team_id: String,
        project_id: Option<String>,
    ) -> assay_core::Result<Self> {
        let api_key = std::env::var("LINEAR_API_KEY").map_err(|_| {
            AssayError::io(
                "LINEAR_API_KEY not set",
                "LINEAR_API_KEY",
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "LINEAR_API_KEY environment variable is not set",
                ),
            )
        })?;
        Ok(Self::new(api_key, base_url, team_id, project_id))
    }

    /// Read the cached issue ID from `.linear-issue-id` in `run_dir`.
    fn read_issue_id(run_dir: &Path) -> assay_core::Result<Option<String>> {
        let path = run_dir.join(".linear-issue-id");
        match std::fs::read_to_string(&path) {
            Ok(id) => {
                let trimmed = id.trim().to_string();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AssayError::io("reading .linear-issue-id", &path, e)),
        }
    }

    /// Write the issue ID to `.linear-issue-id` in `run_dir`.
    fn write_issue_id(run_dir: &Path, issue_id: &str) -> assay_core::Result<()> {
        let path = run_dir.join(".linear-issue-id");
        std::fs::write(&path, issue_id)
            .map_err(|e| AssayError::io("writing .linear-issue-id", &path, e))
    }
}

impl StateBackend for LinearBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: true,
            supports_checkpoints: false,
            supports_signals: false,
        }
    }

    fn push_session_event(
        &self,
        run_dir: &Path,
        status: &OrchestratorStatus,
    ) -> assay_core::Result<()> {
        let status_json = serde_json::to_string_pretty(status)
            .map_err(|e| AssayError::json("serializing OrchestratorStatus", run_dir, e))?;

        match Self::read_issue_id(run_dir)? {
            Some(issue_id) => {
                // Subsequent call — append as comment.
                self.client.create_comment(&issue_id, &status_json)?;
            }
            None => {
                // First call — create issue, cache ID.
                let title = run_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "assay-run".to_string());
                let description = format!("Assay orchestrator run: {title}");
                let issue_id = self
                    .client
                    .create_issue(&self.team_id, &title, &description)?;
                Self::write_issue_id(run_dir, &issue_id)?;
            }
        }

        Ok(())
    }

    fn read_run_state(&self, run_dir: &Path) -> assay_core::Result<Option<OrchestratorStatus>> {
        let issue_id = match Self::read_issue_id(run_dir)? {
            Some(id) => id,
            None => return Ok(None),
        };

        let body = match self.client.get_latest_comment(&issue_id)? {
            Some(b) => b,
            None => return Ok(None),
        };

        let status: OrchestratorStatus = serde_json::from_str(&body).map_err(|e| {
            AssayError::json(
                "deserializing OrchestratorStatus from Linear comment",
                run_dir,
                e,
            )
        })?;

        Ok(Some(status))
    }

    fn send_message(
        &self,
        _inbox_path: &Path,
        _name: &str,
        _contents: &[u8],
    ) -> assay_core::Result<()> {
        Err(AssayError::io(
            "send_message not supported by LinearBackend",
            "LinearBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "LinearBackend does not support messaging",
            ),
        ))
    }

    fn poll_inbox(&self, _inbox_path: &Path) -> assay_core::Result<Vec<(String, Vec<u8>)>> {
        Err(AssayError::io(
            "poll_inbox not supported by LinearBackend",
            "LinearBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "LinearBackend does not support messaging",
            ),
        ))
    }

    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> assay_core::Result<()> {
        let issue_id = Self::read_issue_id(run_dir)?.ok_or_else(|| {
            AssayError::io(
                "annotate_run requires a prior push_session_event",
                run_dir,
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    ".linear-issue-id not found — call push_session_event first",
                ),
            )
        })?;

        let body = format!("[assay:manifest] {manifest_path}");
        self.client.create_comment(&issue_id, &body)?;
        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        _assay_dir: &Path,
        _checkpoint: &TeamCheckpoint,
    ) -> assay_core::Result<()> {
        Err(AssayError::io(
            "save_checkpoint_summary not supported by LinearBackend",
            "LinearBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "LinearBackend does not support checkpoints",
            ),
        ))
    }
}
