//! Worktree management types for git worktree lifecycle operations.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for worktree management.
///
/// Loaded from the `[worktree]` section of `.assay/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeConfig {
    /// Base directory for worktrees.
    /// Relative paths are resolved from the project root.
    /// Default: `"../<project-name>-worktrees/"` (computed at runtime when empty).
    #[serde(default)]
    pub base_dir: String,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "worktree-config",
        generate: || schemars::schema_for!(WorktreeConfig),
    }
}

/// Metadata persisted alongside a worktree at `<worktree>/.assay/worktree.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorktreeMetadata {
    /// The base branch this worktree was created from.
    pub base_branch: String,
    /// The spec slug this worktree is associated with.
    pub spec_slug: String,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "worktree-metadata",
        generate: || schemars::schema_for!(WorktreeMetadata),
    }
}

/// Information about a single git worktree.
///
/// Returned by worktree create and list operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeInfo {
    /// The spec slug this worktree is associated with.
    pub spec_slug: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// The git branch checked out in this worktree (e.g., `assay/auth-flow`).
    pub branch: String,
    /// The base branch this worktree was created from.
    /// Populated on create and list (from metadata), `None` when metadata is missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
}

/// Status of a worktree including runtime state.
///
/// Extends worktree information with dirty state, HEAD commit, and ahead/behind counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeStatus {
    /// The spec slug this worktree is associated with.
    pub spec_slug: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// The git branch checked out in this worktree.
    pub branch: String,
    /// The abbreviated HEAD commit SHA.
    pub head: String,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
    /// Number of commits ahead of base branch. `None` when the base ref is missing.
    pub ahead: Option<usize>,
    /// Number of commits behind base branch. `None` when the base ref is missing.
    pub behind: Option<usize>,
    /// The base branch this worktree was created from (from metadata).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    /// Warnings about degraded status (e.g., missing base ref).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}
