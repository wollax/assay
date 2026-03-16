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
