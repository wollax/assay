//! Tracker types — platform-agnostic issue representation, lifecycle state,
//! and state-backend configuration.
//!
//! These types form the foundation for all tracker functionality in Smelt.
//! [`TrackerIssue`] is the normalized view of an issue from any tracker source
//! (Linear, GitHub, etc.). [`TrackerState`] models the label-based lifecycle
//! used to drive issue progression. [`StateBackendConfig`] mirrors the Assay
//! schema (per D154) so Smelt can pass it through without depending on the
//! Assay crate (per D002).

use serde::{Deserialize, Serialize};

// ── TrackerIssue ────────────────────────────────────────────────

/// A platform-agnostic representation of a tracker issue.
///
/// Every tracker source (Linear, GitHub, etc.) normalizes its native issue
/// type into this struct before Smelt operates on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerIssue {
    /// Unique identifier for the issue within its source (e.g. `"KAT-42"`).
    pub id: String,
    /// Human-readable issue title.
    pub title: String,
    /// Full issue body / description (may be empty).
    pub body: String,
    /// URL to view the issue in the source tracker's web UI.
    pub source_url: String,
}

// ── TrackerState ────────────────────────────────────────────────

/// Label-based lifecycle state for a tracked issue.
///
/// Each variant maps to a label of the form `"{prefix}:{state}"` — e.g.
/// `"smelt:ready"`, `"smelt:running"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerState {
    /// Issue is ready to be picked up.
    Ready,
    /// Issue is queued for execution.
    Queued,
    /// Issue is currently being executed.
    Running,
    /// A pull request has been created for this issue.
    PrCreated,
    /// Issue has been completed successfully.
    Done,
    /// Issue execution failed.
    Failed,
}

impl TrackerState {
    /// Return the lowercase label key for this variant (e.g. `"ready"`,
    /// `"pr_created"`).
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::PrCreated => "pr_created",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    /// Return the label string for this state, e.g. `"smelt:ready"`.
    ///
    /// The format is `"{prefix}:{variant}"` where `variant` is the lowercase
    /// label key for this state (e.g. `"ready"`, `"pr_created"`).
    pub fn label_name(&self, prefix: &str) -> String {
        format!("{prefix}:{}", self.as_str())
    }

    /// All tracker state variants in lifecycle order.
    pub const ALL: &'static [TrackerState] = &[
        Self::Ready,
        Self::Queued,
        Self::Running,
        Self::PrCreated,
        Self::Done,
        Self::Failed,
    ];
}

impl std::fmt::Display for TrackerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── StateBackendConfig ──────────────────────────────────────────

/// Backend configuration for state persistence — mirrors the Assay schema.
///
/// This enum is a structural mirror of Assay's `StateBackendConfig` (per D154)
/// so that Smelt can deserialize `[state_backend]` from a job manifest and pass
/// it through to Assay without taking a crate dependency on `assay-types` (per
/// D002).
///
/// Serializes with `snake_case` tag keys. The `GitHub` variant carries an
/// explicit `#[serde(rename = "github")]` because `rename_all = "snake_case"`
/// would produce `"git_hub"`.
///
/// Uses `toml::Value` for the `Custom` config payload since job manifests are
/// TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateBackendConfig {
    /// Local filesystem backend (default). No additional config needed.
    LocalFs,
    /// Linear backend for syncing state to Linear projects.
    Linear {
        /// Linear team identifier.
        team_id: String,
        /// Optional Linear project to scope state within.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project_id: Option<String>,
    },
    /// GitHub backend for syncing state to GitHub issues/projects.
    #[serde(rename = "github")]
    GitHub {
        /// GitHub repository in `owner/repo` format.
        repo: String,
        /// Optional label to scope state issues.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// SSH backend for syncing state to a remote host.
    Ssh {
        /// Remote host to connect to.
        host: String,
        /// Path to the assay directory on the remote host.
        remote_assay_dir: String,
        /// SSH user (defaults to current user if omitted).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        /// SSH port (defaults to 22 if omitted).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<u16>,
    },
    /// Smelt event channel backend — Assay POSTs events to Smelt's HTTP endpoint.
    Smelt {
        /// URL of the Smelt event ingestion endpoint.
        endpoint_url: String,
        /// Job identifier for associating events.
        job_id: String,
        /// Environment variable name holding the write token (optional).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_env: Option<String>,
    },
    /// Custom third-party backend identified by name.
    Custom {
        /// Identifier for the backend implementation.
        name: String,
        /// Backend-specific configuration payload (TOML value).
        config: toml::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TrackerState label_name tests ───────────────────────────

    #[test]
    fn tracker_state_label_names() {
        let cases = [
            (TrackerState::Ready, "smelt:ready"),
            (TrackerState::Queued, "smelt:queued"),
            (TrackerState::Running, "smelt:running"),
            (TrackerState::PrCreated, "smelt:pr_created"),
            (TrackerState::Done, "smelt:done"),
            (TrackerState::Failed, "smelt:failed"),
        ];
        for (state, expected) in &cases {
            assert_eq!(state.label_name("smelt"), *expected, "state: {state:?}");
        }
    }

    #[test]
    fn tracker_state_label_custom_prefix() {
        assert_eq!(TrackerState::Ready.label_name("ci"), "ci:ready");
    }

    #[test]
    fn tracker_state_all_covers_six_variants() {
        assert_eq!(TrackerState::ALL.len(), 6);
    }

    // ── TrackerState serde round-trip ───────────────────────────

    #[test]
    fn tracker_state_serde_round_trip() {
        for state in TrackerState::ALL {
            let json = serde_json::to_string(state).unwrap();
            let back: TrackerState = serde_json::from_str(&json).unwrap();
            assert_eq!(*state, back);
        }
    }

    // ── TrackerIssue construction ───────────────────────────────

    #[test]
    fn tracker_issue_construction() {
        let issue = TrackerIssue {
            id: "KAT-42".into(),
            title: "Fix the widget".into(),
            body: "The widget is broken".into(),
            source_url: "https://linear.app/team/KAT-42".into(),
        };
        assert_eq!(issue.id, "KAT-42");
        assert_eq!(issue.title, "Fix the widget");
        assert_eq!(issue.body, "The widget is broken");
        assert_eq!(issue.source_url, "https://linear.app/team/KAT-42");
    }

    // ── StateBackendConfig TOML serde round-trips ───────────────

    #[test]
    fn state_backend_local_fs_toml_round_trip() {
        let toml_str = r#"state_backend = "local_fs""#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.state_backend, StateBackendConfig::LocalFs);
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_linear_toml_round_trip() {
        let toml_str = r#"
[state_backend]
linear = { team_id = "TEAM", project_id = "PROJ" }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.state_backend,
            StateBackendConfig::Linear {
                team_id: "TEAM".into(),
                project_id: Some("PROJ".into()),
            }
        );
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_github_toml_round_trip() {
        let toml_str = r#"
[state_backend]
github = { repo = "owner/repo", label = "smelt" }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.state_backend,
            StateBackendConfig::GitHub {
                repo: "owner/repo".into(),
                label: Some("smelt".into()),
            }
        );
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_ssh_toml_round_trip() {
        let toml_str = r#"
[state_backend]
ssh = { host = "ci.example.com", remote_assay_dir = "/opt/assay", user = "deploy", port = 2222 }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.state_backend,
            StateBackendConfig::Ssh {
                host: "ci.example.com".into(),
                remote_assay_dir: "/opt/assay".into(),
                user: Some("deploy".into()),
                port: Some(2222),
            }
        );
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_smelt_toml_round_trip() {
        let toml_str = r#"
[state_backend]
smelt = { endpoint_url = "http://host.docker.internal:8765/api/v1/events", job_id = "job-1", token_env = "SMELT_WRITE_TOKEN" }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.state_backend,
            StateBackendConfig::Smelt {
                endpoint_url: "http://host.docker.internal:8765/api/v1/events".into(),
                job_id: "job-1".into(),
                token_env: Some("SMELT_WRITE_TOKEN".into()),
            }
        );
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_smelt_without_token_env() {
        let toml_str = r#"
[state_backend]
smelt = { endpoint_url = "http://localhost:8765/api/v1/events", job_id = "job-2" }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.state_backend,
            StateBackendConfig::Smelt {
                endpoint_url: "http://localhost:8765/api/v1/events".into(),
                job_id: "job-2".into(),
                token_env: None,
            }
        );
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }

    #[test]
    fn state_backend_custom_toml_round_trip() {
        let toml_str = r#"
[state_backend]
custom = { name = "redis", config = { url = "redis://localhost:6379" } }
"#;
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Wrapper {
            state_backend: StateBackendConfig,
        }
        let parsed: Wrapper = toml::from_str(toml_str).unwrap();
        match &parsed.state_backend {
            StateBackendConfig::Custom { name, config } => {
                assert_eq!(name, "redis");
                assert!(config.get("url").is_some());
            }
            other => panic!("expected Custom, got {other:?}"),
        }
        let re_serialized = toml::to_string(&parsed).unwrap();
        let re_parsed: Wrapper = toml::from_str(&re_serialized).unwrap();
        assert_eq!(parsed, re_parsed);
    }
}
