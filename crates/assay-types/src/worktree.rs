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

/// Information about a single git worktree.
///
/// Returned by worktree create and list operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorktreeInfo {
    /// The spec slug this worktree is associated with.
    pub spec_slug: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// The git branch checked out in this worktree (e.g., `assay/auth-flow`).
    pub branch: String,
    /// The base branch this worktree was created from.
    /// Populated on create, `None` when listing (not available from `git worktree list --porcelain`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
}

/// Status of a worktree including runtime state.
///
/// Extends worktree information with dirty state, HEAD commit, and ahead/behind counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Number of commits ahead of upstream.
    pub ahead: usize,
    /// Number of commits behind upstream.
    pub behind: usize,
}
