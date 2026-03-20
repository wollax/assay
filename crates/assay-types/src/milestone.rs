//! Milestone and chunk types for tracking feature delivery progress.
//!
//! A `Milestone` is a named, version-controlled unit of work tracked in
//! `.assay/milestones/<slug>.toml`. Each milestone aggregates a set of
//! `ChunkRef` entries that reference `GatesSpec` chunks by slug and ordering.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

/// Lifecycle status of a milestone.
///
/// Transitions follow the canonical path:
/// `Draft` → `InProgress` → `Verify` → `Complete`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    /// Milestone is being defined but not yet in active development.
    #[default]
    Draft,
    /// Development is actively in progress.
    InProgress,
    /// All chunks pass gates; awaiting human or automated verification.
    Verify,
    /// Milestone is fully complete and merged.
    Complete,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "milestone-status",
        generate: || schemars::schema_for!(MilestoneStatus),
    }
}

/// A reference to a `GatesSpec` chunk within a milestone, identified by slug and ordering.
///
/// Each `ChunkRef` corresponds to a directory-based spec (a "chunk") that
/// contributes to the parent milestone. The `order` field controls the
/// canonical sequence for display and progress tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChunkRef {
    /// Slug of the referenced `GatesSpec` (matches the spec directory/file name).
    pub slug: String,
    /// Position of this chunk in the milestone's ordered sequence (0-based or 1-based is caller's choice).
    pub order: u32,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "chunk-ref",
        generate: || schemars::schema_for!(ChunkRef),
    }
}

/// A milestone, tracking a named feature delivery across one or more gate spec chunks.
///
/// Persisted to `.assay/milestones/<slug>.toml`. The `slug` field in the file
/// must match the filename (e.g. `my-feature.toml` → `slug = "my-feature"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    /// Unique identifier for this milestone; must match the TOML filename without extension.
    pub slug: String,

    /// Human-readable display name.
    pub name: String,

    /// Optional longer description of the milestone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Current lifecycle status. Defaults to `Draft`.
    #[serde(default)]
    pub status: MilestoneStatus,

    /// Ordered list of chunk references included in this milestone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chunks: Vec<ChunkRef>,

    /// Slugs of chunks that have been verified and advanced past in the development cycle.
    ///
    /// This is the central state for the cycle state machine: the "active chunk" is
    /// derived at runtime as the lowest-`order` `ChunkRef` whose slug is not in this list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_chunks: Vec<String>,

    /// Milestone slugs that must be `Complete` before this one can start.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,

    /// Feature branch name for this milestone's pull request (e.g. `"feat/my-feature"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_branch: Option<String>,

    /// Target base branch for the pull request (e.g. `"main"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_base: Option<String>,

    /// When this milestone record was first created.
    pub created_at: DateTime<Utc>,

    /// When this milestone record was last updated.
    pub updated_at: DateTime<Utc>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "milestone",
        generate: || schemars::schema_for!(Milestone),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_now() -> DateTime<Utc> {
        // Fixed timestamp for deterministic roundtrip tests
        "2026-03-19T00:00:00Z".parse().unwrap()
    }

    #[test]
    fn milestone_toml_roundtrip() {
        let now = make_now();
        let milestone = Milestone {
            slug: "my-feature".to_string(),
            name: "My Feature".to_string(),
            description: Some("Delivers the my-feature capability".to_string()),
            status: MilestoneStatus::InProgress,
            chunks: vec![
                ChunkRef {
                    slug: "auth-flow".to_string(),
                    order: 1,
                },
                ChunkRef {
                    slug: "payment-flow".to_string(),
                    order: 2,
                },
            ],
            completed_chunks: vec![],
            depends_on: vec!["auth-foundation".to_string()],
            pr_branch: Some("feat/my-feature".to_string()),
            pr_base: Some("main".to_string()),
            created_at: now,
            updated_at: now,
        };

        let toml_str = toml::to_string(&milestone).expect("serialize to TOML");
        let roundtripped: Milestone = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(milestone, roundtripped);

        // Verify required fields present
        assert_eq!(roundtripped.slug, "my-feature");
        assert_eq!(roundtripped.name, "My Feature");
        assert_eq!(roundtripped.status, MilestoneStatus::InProgress);
        assert_eq!(roundtripped.chunks.len(), 2);
        assert_eq!(roundtripped.chunks[0].slug, "auth-flow");
        assert_eq!(roundtripped.chunks[1].order, 2);
        assert_eq!(roundtripped.depends_on, vec!["auth-foundation"]);
        assert_eq!(roundtripped.pr_branch.as_deref(), Some("feat/my-feature"));
        assert_eq!(roundtripped.pr_base.as_deref(), Some("main"));
    }

    #[test]
    fn milestone_minimal_toml_roundtrip() {
        let now = make_now();
        let milestone = Milestone {
            slug: "simple".to_string(),
            name: "Simple Milestone".to_string(),
            description: None,
            status: MilestoneStatus::Draft,
            chunks: vec![],
            completed_chunks: vec![],
            depends_on: vec![],
            pr_branch: None,
            pr_base: None,
            created_at: now,
            updated_at: now,
        };

        let toml_str = toml::to_string(&milestone).expect("serialize to TOML");
        let roundtripped: Milestone = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(milestone, roundtripped);

        // Optional fields absent from output
        assert!(roundtripped.description.is_none());
        assert!(roundtripped.pr_branch.is_none());
        assert!(roundtripped.pr_base.is_none());
        assert!(roundtripped.chunks.is_empty());
        assert!(roundtripped.completed_chunks.is_empty());
        assert!(roundtripped.depends_on.is_empty());
        assert!(!toml_str.contains("description"));
        assert!(!toml_str.contains("pr_branch"));
        assert!(!toml_str.contains("depends_on"));
        // skip_serializing_if = "Vec::is_empty" — empty completed_chunks must not appear in TOML
        assert!(
            !toml_str.contains("completed_chunks"),
            "empty completed_chunks should be omitted from TOML, got: {toml_str}"
        );
    }

    #[test]
    fn milestone_status_default_is_draft() {
        assert_eq!(MilestoneStatus::default(), MilestoneStatus::Draft);
    }

    #[test]
    fn milestone_status_serde_roundtrip() {
        let statuses = [
            MilestoneStatus::Draft,
            MilestoneStatus::InProgress,
            MilestoneStatus::Verify,
            MilestoneStatus::Complete,
        ];
        for &status in &statuses {
            let json = serde_json::to_string(&status).expect("serialize status");
            let back: MilestoneStatus = serde_json::from_str(&json).expect("deserialize status");
            assert_eq!(back, status);
        }
        // Verify snake_case serialization
        assert_eq!(
            serde_json::to_string(&MilestoneStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&MilestoneStatus::Complete).unwrap(),
            "\"complete\""
        );
    }

    #[test]
    fn chunk_ref_deny_unknown_fields() {
        let json = r#"{"slug":"auth","order":1,"unknown":true}"#;
        assert!(
            serde_json::from_str::<ChunkRef>(json).is_err(),
            "ChunkRef should reject unknown fields"
        );
    }

    #[test]
    fn milestone_deny_unknown_fields() {
        let json = r#"{"slug":"x","name":"X","status":"draft","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","unknown":1}"#;
        assert!(
            serde_json::from_str::<Milestone>(json).is_err(),
            "Milestone should reject unknown fields"
        );
    }
}
