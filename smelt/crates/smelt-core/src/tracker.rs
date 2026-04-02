//! Tracker types — platform-agnostic issue representation, lifecycle state,
//! and state-backend configuration.
//!
//! These types form the foundation for all tracker functionality in Smelt.
//! [`TrackerIssue`] is the normalized view of an issue from any tracker source
//! (Linear, GitHub, etc.). [`TrackerState`] models the label-based lifecycle
//! used to drive issue progression. [`StateBackendConfig`] is re-exported from
//! `assay-types` — the canonical definition.

use serde::{Deserialize, Serialize};

// Re-export the canonical StateBackendConfig from assay-types.
pub use assay_types::StateBackendConfig;

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
}
