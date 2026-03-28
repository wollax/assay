//! `ReqwestLinearClient` — async HTTP client for the Linear GraphQL API.

use serde_json::{self, Value, json};
use smelt_core::error::SmeltError;
use tracing::{debug, warn};

use super::{LinearClient, LinearIssue, LinearLabel};

// ---------------------------------------------------------------------------
// Helper: execute a label mutation (shared by add_label / remove_label)
// ---------------------------------------------------------------------------

/// Execute a label mutation (`issueAddLabel` or `issueRemoveLabel`) and check
/// the `success` field in the response.
async fn execute_label_mutation(
    client: &ReqwestLinearClient,
    operation: &str,
    mutation_field: &str,
    issue_id: &str,
    label_id: &str,
) -> Result<(), SmeltError> {
    let query = format!(
        r#"mutation($issueId: String!, $labelId: String!) {{
            {mutation_field}(id: $issueId, labelId: $labelId) {{
                success
            }}
        }}"#
    );

    let body = json!({
        "query": query,
        "variables": {
            "issueId": issue_id,
            "labelId": label_id,
        }
    });

    let json = client.graphql(operation, &body).await?;

    let success = json
        .get("data")
        .and_then(|d| d.get(mutation_field))
        .and_then(|r| r.get("success"))
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    if !success {
        warn!(
            operation = %operation,
            mutation = %mutation_field,
            issue_id = %issue_id,
            label_id = %label_id,
            "label mutation returned success=false"
        );
        return Err(SmeltError::tracker(
            operation,
            format!(
                "{mutation_field} returned success=false for issue={issue_id} label={label_id}"
            ),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// ReqwestLinearClient
// ---------------------------------------------------------------------------

/// `LinearClient` implementation using async `reqwest::Client` for GraphQL.
///
/// Auth is via `Authorization: {api_key}` header (Linear personal API key).
/// The client sends POST requests to `{base_url}/graphql`.
#[derive(Clone)]
pub struct ReqwestLinearClient {
    client: reqwest::Client,
    graphql_url: String,
}

impl ReqwestLinearClient {
    /// Build a new client with the given API key and base URL.
    ///
    /// Pass `"https://api.linear.app"` for production; the `/graphql`
    /// suffix is appended automatically. Use a different base URL for
    /// testing against a mock server.
    pub fn new(api_key: String, base_url: String) -> Result<Self, SmeltError> {
        use reqwest::header;

        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(&api_key).map_err(|e| {
            SmeltError::tracker(
                "linear_client_new",
                format!("invalid Authorization header value: {e}"),
            )
        })?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| {
                SmeltError::tracker("linear_client_new", format!("failed to build client: {e}"))
            })?;

        let graphql_url = format!("{}/graphql", base_url.trim_end_matches('/'));

        Ok(Self {
            client,
            graphql_url,
        })
    }

    /// Execute a GraphQL request and return the parsed JSON body.
    ///
    /// Checks for:
    /// 1. HTTP non-200 status codes (checked first; body preview included)
    /// 2. GraphQL-level errors (`"errors"` array, typically on HTTP 200 — Assay pattern)
    async fn graphql(&self, operation: &str, body: &Value) -> Result<Value, SmeltError> {
        let query_preview = body
            .get("query")
            .and_then(|q| q.as_str())
            .unwrap_or("<none>")
            .chars()
            .take(80)
            .collect::<String>();

        debug!(
            operation = %operation,
            url = %self.graphql_url,
            query_preview = %query_preview,
            "sending GraphQL request"
        );

        let resp = self
            .client
            .post(&self.graphql_url)
            .json(body)
            .send()
            .await
            .map_err(|e| SmeltError::tracker(operation, format!("HTTP request failed: {e}")))?;

        let status = resp.status();

        // Read raw bytes so we can surface the body on non-JSON/non-200 responses.
        let body_bytes = resp.bytes().await.map_err(|e| {
            SmeltError::tracker(
                operation,
                format!("HTTP {status}: failed to read response body: {e}"),
            )
        })?;

        // Check HTTP status first — non-200 with HTML/text body would fail
        // JSON parse and produce a confusing error.
        if !status.is_success() {
            let preview: String = String::from_utf8_lossy(&body_bytes)
                .chars()
                .take(200)
                .collect();
            warn!(
                operation = %operation,
                status = %status,
                body_preview = %preview,
                "Linear API returned non-200"
            );
            return Err(SmeltError::tracker(
                operation,
                format!("Linear API returned HTTP {status}: {preview}"),
            ));
        }

        let json: Value = serde_json::from_slice(&body_bytes).map_err(|e| {
            SmeltError::tracker(
                operation,
                format!("HTTP {status}: failed to parse response body as JSON: {e}"),
            )
        })?;

        // GraphQL errors are in the `errors` array (typically on HTTP 200,
        // but checked on any status that made it past the guard above).
        if let Some(errors) = json.get("errors") {
            let msg = errors
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "unknown GraphQL error".to_string());
            warn!(
                operation = %operation,
                error = %msg,
                "GraphQL error response"
            );
            return Err(SmeltError::tracker(
                operation,
                format!("GraphQL error: {msg}"),
            ));
        }

        Ok(json)
    }
}

impl LinearClient for ReqwestLinearClient {
    async fn list_issues(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> Result<Vec<LinearIssue>, SmeltError> {
        let query = r#"
            query($teamId: ID!, $labelName: String!) {
                issues(filter: {
                    labels: { name: { eq: $labelName } },
                    team: { id: { eq: $teamId } }
                }) {
                    nodes { id identifier title description url }
                }
            }
        "#;

        let body = json!({
            "query": query,
            "variables": {
                "teamId": team_id,
                "labelName": label_name,
            }
        });

        let json = self.graphql("list_issues", &body).await?;

        let nodes = json
            .get("data")
            .and_then(|d| d.get("issues"))
            .and_then(|i| i.get("nodes"))
            .ok_or_else(|| {
                SmeltError::tracker(
                    "list_issues",
                    "unexpected response shape: missing data.issues.nodes",
                )
            })?;

        let issues: Vec<LinearIssue> = serde_json::from_value(nodes.clone()).map_err(|e| {
            SmeltError::tracker("list_issues", format!("failed to parse issues: {e}"))
        })?;

        Ok(issues)
    }

    async fn add_label(&self, issue_id: &str, label_id: &str) -> Result<(), SmeltError> {
        execute_label_mutation(self, "add_label", "issueAddLabel", issue_id, label_id).await
    }

    async fn remove_label(&self, issue_id: &str, label_id: &str) -> Result<(), SmeltError> {
        execute_label_mutation(self, "remove_label", "issueRemoveLabel", issue_id, label_id).await
    }

    async fn find_label(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> Result<Option<LinearLabel>, SmeltError> {
        let query = r#"
            query($teamId: ID!, $labelName: String!) {
                issueLabels(filter: {
                    name: { eq: $labelName },
                    team: { id: { eq: $teamId } }
                }) {
                    nodes { id name }
                }
            }
        "#;

        let body = json!({
            "query": query,
            "variables": {
                "teamId": team_id,
                "labelName": label_name,
            }
        });

        let json = self.graphql("find_label", &body).await?;

        let nodes = json
            .get("data")
            .and_then(|d| d.get("issueLabels"))
            .and_then(|l| l.get("nodes"))
            .ok_or_else(|| {
                SmeltError::tracker(
                    "find_label",
                    "unexpected response shape: missing data.issueLabels.nodes",
                )
            })?;

        let labels: Vec<LinearLabel> = serde_json::from_value(nodes.clone()).map_err(|e| {
            SmeltError::tracker("find_label", format!("failed to parse labels: {e}"))
        })?;

        Ok(labels.into_iter().next())
    }

    async fn create_label(
        &self,
        team_id: &str,
        label_name: &str,
    ) -> Result<LinearLabel, SmeltError> {
        let query = r#"
            mutation($teamId: String!, $labelName: String!) {
                issueLabelCreate(input: { name: $labelName, teamId: $teamId }) {
                    success
                    issueLabel { id name }
                }
            }
        "#;

        let body = json!({
            "query": query,
            "variables": {
                "teamId": team_id,
                "labelName": label_name,
            }
        });

        let json = self.graphql("create_label", &body).await?;

        let result = json
            .get("data")
            .and_then(|d| d.get("issueLabelCreate"))
            .ok_or_else(|| {
                SmeltError::tracker(
                    "create_label",
                    "unexpected response shape: missing data.issueLabelCreate",
                )
            })?;

        let success = result
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        if !success {
            return Err(SmeltError::tracker(
                "create_label",
                format!(
                    "issueLabelCreate returned success=false for team={team_id} label={label_name}"
                ),
            ));
        }

        let label_val = result.get("issueLabel").ok_or_else(|| {
            SmeltError::tracker(
                "create_label",
                "issueLabelCreate succeeded but missing issueLabel in response",
            )
        })?;

        let label: LinearLabel = serde_json::from_value(label_val.clone()).map_err(|e| {
            SmeltError::tracker(
                "create_label",
                format!("failed to parse created label: {e}"),
            )
        })?;

        Ok(label)
    }
}
