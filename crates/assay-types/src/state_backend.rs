//! Backend configuration for state persistence.

use crate::schema_registry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Backend configuration for state persistence.
///
/// Serializes with `snake_case` tag keys:
/// - `LocalFs` → `"local_fs"`
/// - `Linear` → `{"linear": {...}}`
/// - `GitHub` → `{"github": {...}}` — note: `rename_all = "snake_case"` would
///   produce `"git_hub"`, so the `GitHub` variant carries an explicit
///   `#[serde(rename = "github")]` attribute to override this
/// - `Ssh` → `{"ssh": {...}}`
/// - `Custom` → `{"custom": {...}}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Custom third-party backend identified by name.
    Custom {
        /// Identifier for the backend implementation.
        name: String,
        /// Backend-specific configuration payload.
        config: serde_json::Value,
    },
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "state-backend-config",
        generate: || schemars::schema_for!(StateBackendConfig),
    }
}
