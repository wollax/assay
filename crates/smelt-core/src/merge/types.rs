//! Types for merge operations and reporting.

use serde::{Deserialize, Serialize};

/// Strategy for ordering sessions during merge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum MergeOrderStrategy {
    /// Order sessions by manifest position (default — preserves Phase 4 behavior).
    #[default]
    CompletionTime,
    /// Order sessions by file overlap — merge least-overlapping first.
    FileOverlap,
}

/// Options for a merge operation.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MergeOpts {
    /// Override the target branch name (default: `smelt/merge/<manifest-name>`).
    pub target_branch: Option<String>,
    /// Override the merge ordering strategy.
    pub strategy: Option<MergeOrderStrategy>,
}

impl MergeOpts {
    /// Create merge options with a custom target branch.
    pub fn with_target_branch(target: String) -> Self {
        Self {
            target_branch: Some(target),
            strategy: None,
        }
    }

    /// Create merge options with a specific ordering strategy.
    pub fn with_strategy(strategy: MergeOrderStrategy) -> Self {
        Self {
            target_branch: None,
            strategy: Some(strategy),
        }
    }
}

/// Per-file diff statistics.
#[derive(Debug, Clone, Serialize)]
pub struct DiffStat {
    pub file: String,
    pub insertions: usize,
    pub deletions: usize,
}

/// Result of merging a single session.
#[derive(Debug, Clone, Serialize)]
pub struct MergeSessionResult {
    pub session_name: String,
    pub commit_hash: String,
    pub diff_stats: Vec<DiffStat>,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Overall merge report.
#[derive(Debug, Clone, Serialize)]
pub struct MergeReport {
    pub target_branch: String,
    pub base_commit: String,
    pub sessions_merged: Vec<MergeSessionResult>,
    pub sessions_skipped: Vec<String>,
    pub total_files_changed: usize,
    pub total_insertions: usize,
    pub total_deletions: usize,
}

impl MergeReport {
    /// Returns `true` if any sessions were skipped during the merge.
    pub fn has_skipped(&self) -> bool {
        !self.sessions_skipped.is_empty()
    }
}
