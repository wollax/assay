//! Merge check types for conflict detection between git refs.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Change type for files in a clean merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// File was added.
    Added,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

/// A file changed in a clean merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileChange {
    /// Path of the changed file relative to the repository root.
    pub path: String,
    /// Type of change (added, modified, or deleted).
    pub change_type: ChangeType,
}

/// Classification of a merge conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Both sides modified the same file differently.
    Content,
    /// One side renamed, other deleted.
    RenameDelete,
    /// Both sides renamed the same file differently.
    RenameRename,
    /// One side modified, other deleted.
    ModifyDelete,
    /// Both sides added a file at the same path.
    AddAdd,
    /// One side has a file, other has a directory at the same path.
    FileDirectory,
    /// Binary file modified on both sides.
    Binary,
    /// Submodule conflict.
    Submodule,
    /// Unknown or future conflict type.
    Other(String),
}

impl fmt::Display for ConflictType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content => write!(f, "content"),
            Self::RenameDelete => write!(f, "rename/delete"),
            Self::RenameRename => write!(f, "rename/rename"),
            Self::ModifyDelete => write!(f, "modify/delete"),
            Self::AddAdd => write!(f, "add/add"),
            Self::FileDirectory => write!(f, "file/directory"),
            Self::Binary => write!(f, "binary"),
            Self::Submodule => write!(f, "submodule"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// A single merge conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeConflict {
    /// Path of the conflicted file relative to the repository root.
    pub path: String,
    /// Classification of the conflict.
    pub conflict_type: ConflictType,
    /// Raw git conflict message.
    pub message: String,
}

/// Result of a merge check between two refs.
///
/// Contains conflict information, file changes, and divergence metadata.
/// Produced by `merge_check()` in `assay-core`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeCheck {
    /// Whether the merge is clean (no conflicts).
    pub clean: bool,
    /// Resolved SHA of the base ref.
    pub base_sha: String,
    /// Resolved SHA of the head ref.
    pub head_sha: String,
    /// SHA of the merge base (common ancestor). `None` for unrelated histories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_base_sha: Option<String>,
    /// Whether the head is a direct descendant of base (fast-forward possible).
    pub fast_forward: bool,
    /// Number of commits in head not in base.
    pub ahead: u32,
    /// Number of commits in base not in head.
    pub behind: u32,
    /// Files changed in a clean merge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileChange>,
    /// Conflicts detected in a conflicted merge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<MergeConflict>,
    /// Whether the conflict list was truncated by `max_conflicts`.
    pub truncated: bool,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "merge-check",
        generate: || schemars::schema_for!(MergeCheck),
    }
}

// ---------------------------------------------------------------------------
// Merge execution types
// ---------------------------------------------------------------------------

/// Type of a conflict marker found in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkerType {
    /// Start of "ours" side: `<<<<<<<`
    Ours,
    /// Separator between sides: `=======`
    Separator,
    /// Start of "theirs" side: `>>>>>>>`
    Theirs,
}

impl fmt::Display for MarkerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ours => write!(f, "ours"),
            Self::Separator => write!(f, "separator"),
            Self::Theirs => write!(f, "theirs"),
        }
    }
}

/// A single conflict marker found in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictMarker {
    /// File path relative to the repository root.
    pub file: String,
    /// 1-based line number where the marker was found.
    pub line: u32,
    /// Type of marker (`<<<<<<<`, `=======`, or `>>>>>>>`).
    pub marker_type: MarkerType,
}

/// Result of scanning files for conflict markers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictScan {
    /// Whether any conflict markers were found.
    pub has_markers: bool,
    /// Individual markers found (may be empty).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<ConflictMarker>,
    /// Whether the scan was truncated (too many files or markers).
    pub truncated: bool,
}

/// Result of executing `git merge --no-ff`.
///
/// On success, contains the merge commit SHA and changed files.
/// On conflict, contains conflict details with automatic abort cleanup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeExecuteResult {
    /// SHA of the merge commit (present only on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_sha: Option<String>,
    /// Files changed by the merge (present only on success).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_changed: Vec<FileChange>,
    /// Whether the merge had a conflict.
    pub was_conflict: bool,
    /// Conflict details when `was_conflict` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_details: Option<ConflictScan>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "merge-execute-result",
        generate: || schemars::schema_for!(MergeExecuteResult),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "conflict-scan",
        generate: || schemars::schema_for!(ConflictScan),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "conflict-marker",
        generate: || schemars::schema_for!(ConflictMarker),
    }
}

// ---------------------------------------------------------------------------
// Merge propose types
// ---------------------------------------------------------------------------

/// Result of a merge proposal (push branch + create PR).
///
/// Returned by `merge_propose()` in `assay-core`. When `dry_run` is true,
/// `pr_url` and `pr_number` are `None` and no side effects occur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeProposal {
    /// URL of the created pull request (absent in dry-run mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    /// Number of the created pull request (absent in dry-run mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    /// Formatted gate evidence used as the PR body.
    pub gate_summary: String,
    /// Whether this was a dry run (no side effects).
    pub dry_run: bool,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "merge-proposal",
        generate: || schemars::schema_for!(MergeProposal),
    }
}

/// Configuration for `merge_propose()`.
///
/// This is a function-parameter struct, not a persisted type.
/// It is not serialized or included in schema snapshots.
#[derive(Debug, Clone)]
pub struct MergeProposeConfig {
    /// Name of the spec to gather gate evidence for.
    pub spec_name: String,
    /// Specific run ID to use for gate evidence. If `None`, uses the latest.
    pub run_id: Option<String>,
    /// Branch to push and create PR from.
    pub branch: String,
    /// Target base branch for the PR (e.g. "main").
    pub base_branch: String,
    /// PR title.
    pub title: String,
    /// Working directory (git repository root).
    pub working_dir: std::path::PathBuf,
    /// Path to the `.assay` directory.
    pub assay_dir: std::path::PathBuf,
    /// If true, return the proposal without pushing or creating a PR.
    pub dry_run: bool,
}
