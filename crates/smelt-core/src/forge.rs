//! Forge integration — `ForgeClient` trait, public types, and the `GitHubForge`
//! implementation backed by octocrab.
//!
//! # Feature gating
//!
//! * All public **types** and the **`ForgeClient` trait** are compiled
//!   unconditionally so that other crates (e.g. `smelt-core`'s manifest module)
//!   can reference `ForgeConfig` without enabling the `forge` feature.
//! * **`GitHubForge`** and everything that imports from `octocrab` is gated
//!   behind `#[cfg(feature = "forge")]`.

use serde::{Deserialize, Serialize};

// ── Public types (unconditional) ─────────────────────────────────────────────

/// Configuration block for a forge provider, loaded from the manifest.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ForgeConfig {
    /// Provider identifier, e.g. `"github"`.
    pub provider: String,
    /// Repository slug in `owner/repo` format.
    pub repo: String,
    /// Name of the environment variable that holds the auth token.
    pub token_env: String,
}

/// Opaque handle to a pull request returned by [`ForgeClient::create_pr`].
#[derive(Debug, Clone, PartialEq)]
pub struct PrHandle {
    /// Browser URL of the pull request.
    pub url: String,
    /// Pull request number.
    pub number: u64,
}

/// High-level state of a pull request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    /// The pull request is open and not yet merged or closed.
    Open,
    /// The pull request was merged into the target branch.
    Merged,
    /// The pull request was closed without being merged.
    Closed,
}

/// Aggregate CI check status for a pull request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    /// CI checks are in progress and have not yet produced a result.
    Pending,
    /// All CI checks completed successfully.
    Passing,
    /// One or more CI checks failed.
    Failing,
    /// CI status could not be determined (e.g. no checks configured).
    Unknown,
}

/// Combined status snapshot for a pull request.
#[derive(Debug, Clone)]
pub struct PrStatus {
    /// High-level open/merged/closed state of the pull request.
    pub state: PrState,
    /// Aggregate CI check result for the pull request HEAD commit.
    pub ci_status: CiStatus,
    /// Number of review comments on the pull request.
    pub review_count: u32,
}

// ── ForgeClient trait (unconditional) ────────────────────────────────────────

/// Forge operations required by the Smelt PR lifecycle.
///
/// Implementations must be `Send + Sync`.  All methods return futures that
/// are `Send` so they can be driven inside a Tokio multi-thread executor.
pub trait ForgeClient: Send + Sync {
    /// Open a pull request and return an opaque [`PrHandle`].
    fn create_pr(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = crate::Result<PrHandle>> + Send;

    /// Poll the current state and CI status of an open pull request.
    fn poll_pr_status(
        &self,
        repo: &str,
        number: u64,
    ) -> impl std::future::Future<Output = crate::Result<PrStatus>> + Send;
}

// ── GitHubForge (feature-gated) ──────────────────────────────────────────────

/// GitHub implementation of [`ForgeClient`] backed by `octocrab`.
///
/// Only available with `--features forge`.
#[cfg(feature = "forge")]
pub struct GitHubForge {
    client: octocrab::Octocrab,
}

#[cfg(feature = "forge")]
impl GitHubForge {
    /// Construct a `GitHubForge` authenticated with a personal access token.
    pub fn new(token: String) -> crate::Result<Self> {
        let client = octocrab::OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .map_err(|e| crate::SmeltError::forge("init", e.to_string()))?;
        Ok(Self { client })
    }
}

/// Parse an `"owner/repo"` slug into its two components.
///
/// Returns `SmeltError::Forge { operation: "create_pr", .. }` if the slash
/// is absent.
#[cfg(feature = "forge")]
fn parse_repo(repo: &str) -> crate::Result<(&str, &str)> {
    repo.split_once('/')
        .ok_or_else(|| crate::SmeltError::forge("create_pr", "invalid repo: expected 'owner/repo' format"))
}

#[cfg(feature = "forge")]
impl ForgeClient for GitHubForge {
    async fn create_pr(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> crate::Result<PrHandle> {
        let (owner, repo_name) = parse_repo(repo)?;
        let pr = self
            .client
            .pulls(owner, repo_name)
            .create(title, head, base)
            .body(body)
            .send()
            .await
            .map_err(|e| crate::SmeltError::forge("create_pr", e.to_string()))?;
        Ok(PrHandle {
            url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
            number: pr.number,
        })
    }

    async fn poll_pr_status(
        &self,
        repo: &str,
        number: u64,
    ) -> crate::Result<PrStatus> {
        let (owner, repo_name) = parse_repo(repo)?;

        // 1. Fetch the PR model.
        let pr = self
            .client
            .pulls(owner, repo_name)
            .get(number)
            .await
            .map_err(|e| crate::SmeltError::forge("poll_pr_status", e.to_string()))?;

        // 2. Derive PrState from merged flag and state field.
        use octocrab::models::IssueState;
        let state = match (pr.merged, pr.state.as_ref()) {
            (Some(true), _) => PrState::Merged,
            (_, Some(IssueState::Closed)) => PrState::Closed,
            _ => PrState::Open,
        };

        // 3. Fetch combined commit status for the HEAD SHA.
        //    Failure is non-fatal: fall back to CiStatus::Unknown.
        #[derive(serde::Deserialize)]
        struct CombinedStatus {
            state: String,
        }

        let sha = pr.head.sha.as_str();
        let ci_status = if sha.is_empty() {
            CiStatus::Unknown
        } else {
            let url = format!("/repos/{owner}/{repo_name}/commits/{sha}/status");
            match self.client._get(url).await {
                Ok(resp) => match self.client.body_to_string(resp).await {
                    Ok(body) => match serde_json::from_str::<CombinedStatus>(&body) {
                        Ok(cs) => match cs.state.as_str() {
                            "success" => CiStatus::Passing,
                            "failure" | "error" => CiStatus::Failing,
                            "pending" => CiStatus::Pending,
                            _ => CiStatus::Unknown,
                        },
                        Err(_) => CiStatus::Unknown,
                    },
                    Err(_) => CiStatus::Unknown,
                },
                Err(_) => CiStatus::Unknown,
            }
        };

        // 4. Derive review_count from inline diff comments (no extra API call).
        let review_count = pr.review_comments.unwrap_or(0) as u32;

        Ok(PrStatus { state, ci_status, review_count })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "forge")]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Compile-time assertion: octocrab::Error must be Send + Sync + 'static
    // for it to be usable as a boxed source error.  If this does not compile,
    // keep forge_with_source as a stringify-only helper (D053).
    fn _assert_octocrab_error_send_sync()
    where
        octocrab::Error: Send + Sync + 'static,
    {
    }

    /// Build a `GitHubForge` redirected at the given mock server URI.
    ///
    /// Constructs `GitHubForge { client }` directly to bypass token-at-
    /// construction validation and point octocrab at the mock base URI.
    async fn forge_for_server(server: &MockServer) -> GitHubForge {
        let client = octocrab::OctocrabBuilder::new()
            .base_uri(server.uri())
            .expect("invalid mock server URI")
            .personal_token("test-token".to_string())
            .build()
            .expect("failed to build octocrab client");
        GitHubForge { client }
    }

    // ── create_pr tests ───────────────────────────────────────────────────────

    /// Happy path: server returns 201 → `PrHandle { url, number }` matches.
    #[tokio::test]
    async fn test_create_pr_happy_path() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/owner/repo/pulls/42",
                "id": 1,
                "node_id": "PR_1",
                "html_url": "https://github.com/owner/repo/pull/42",
                "number": 42,
                "state": "open",
                "locked": false,
                "maintainer_can_modify": false,
                "title": "Test PR",
                "head": { "ref": "feature", "sha": "abc123" },
                "base": { "ref": "main", "sha": "def456" }
            })))
            .mount(&server)
            .await;

        let result = forge
            .create_pr("owner/repo", "feature", "main", "Test PR", "Body")
            .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let handle = result.unwrap();
        assert_eq!(handle.number, 42);
        assert_eq!(handle.url, "https://github.com/owner/repo/pull/42");
    }

    /// Auth error: server returns 401 → `Err(SmeltError::Forge { .. })`.
    #[tokio::test]
    async fn test_create_pr_auth_error() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "message": "Bad credentials",
                "documentation_url": "https://docs.github.com/rest"
            })))
            .mount(&server)
            .await;

        let result = forge
            .create_pr("owner/repo", "feature", "main", "Test PR", "Body")
            .await;
        assert!(
            matches!(result, Err(crate::SmeltError::Forge { .. })),
            "expected Forge error, got {:?}",
            result
        );
    }

    /// Validation error: server returns 422 → `Err(SmeltError::Forge { .. })`.
    #[tokio::test]
    async fn test_create_pr_validation_error() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
                "message": "Validation Failed",
                "errors": [{ "message": "A pull request already exists for feature." }],
                "documentation_url": "https://docs.github.com/rest"
            })))
            .mount(&server)
            .await;

        let result = forge
            .create_pr("owner/repo", "feature", "main", "Test PR", "Body")
            .await;
        assert!(
            matches!(result, Err(crate::SmeltError::Forge { .. })),
            "expected Forge error, got {:?}",
            result
        );
    }

    // ── poll_pr_status tests ──────────────────────────────────────────────────

    /// Open PR + pending CI → `PrStatus { state: Open, ci_status: Pending, .. }`.
    #[tokio::test]
    async fn test_poll_pr_status_open_pending() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/owner/repo/pulls/1",
                "id": 1,
                "number": 1,
                "state": "open",
                "merged": false,
                "html_url": "https://github.com/owner/repo/pull/1",
                "head": { "ref": "feature", "sha": "sha-open" },
                "base": { "ref": "main", "sha": "base-sha" }
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/commits/sha-open/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "pending",
                "statuses": []
            })))
            .mount(&server)
            .await;

        let result = forge.poll_pr_status("owner/repo", 1).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let status = result.unwrap();
        assert_eq!(status.state, PrState::Open);
        assert_eq!(status.ci_status, CiStatus::Pending);
        assert_eq!(status.review_count, 0);
    }

    /// Merged PR + passing CI → `PrStatus { state: Merged, ci_status: Passing, .. }`.
    #[tokio::test]
    async fn test_poll_pr_status_merged_passing() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/owner/repo/pulls/2",
                "id": 2,
                "number": 2,
                "state": "closed",
                "merged": true,
                "html_url": "https://github.com/owner/repo/pull/2",
                "head": { "ref": "feature", "sha": "sha-merged" },
                "base": { "ref": "main", "sha": "base-sha" }
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/commits/sha-merged/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "success",
                "statuses": []
            })))
            .mount(&server)
            .await;

        let result = forge.poll_pr_status("owner/repo", 2).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let status = result.unwrap();
        assert_eq!(status.state, PrState::Merged);
        assert_eq!(status.ci_status, CiStatus::Passing);
    }

    /// Closed (not merged) PR + failing CI → `PrStatus { state: Closed, ci_status: Failing, .. }`.
    #[tokio::test]
    async fn test_poll_pr_status_closed_failing() {
        let server = MockServer::start().await;
        let forge = forge_for_server(&server).await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/owner/repo/pulls/3",
                "id": 3,
                "number": 3,
                "state": "closed",
                "merged": false,
                "html_url": "https://github.com/owner/repo/pull/3",
                "head": { "ref": "feature", "sha": "sha-closed" },
                "base": { "ref": "main", "sha": "base-sha" }
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/commits/sha-closed/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "failure",
                "statuses": []
            })))
            .mount(&server)
            .await;

        let result = forge.poll_pr_status("owner/repo", 3).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let status = result.unwrap();
        assert_eq!(status.state, PrState::Closed);
        assert_eq!(status.ci_status, CiStatus::Failing);
    }
}
