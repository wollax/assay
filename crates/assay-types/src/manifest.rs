//! Run manifest types for declaring agentic work sessions.
//!
//! A [`RunManifest`] is the user-authored input that declares what work to run.
//! It contains one or more [`ManifestSession`] entries, each referencing a spec
//! and optionally overriding harness settings, hooks, and prompt layers.
//!
//! The pipeline (S07) constructs a full [`HarnessProfile`](crate::HarnessProfile)
//! from each manifest session combined with spec defaults.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::harness::{HookContract, PromptLayer, SettingsOverride};

/// Top-level run manifest declaring one or more agent sessions.
///
/// Deserialized from TOML where sessions use the `[[sessions]]` array-of-tables
/// syntax. Each session references a spec and optionally overrides harness
/// configuration.
///
/// # Example TOML
///
/// ```toml
/// [[sessions]]
/// spec = "auth-flow"
///
/// [[sessions]]
/// spec = "checkout"
/// name = "checkout-with-hooks"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    /// Ordered list of sessions to execute.
    ///
    /// Maps to `[[sessions]]` in TOML. At least one session is expected,
    /// but emptiness is a validation concern, not a deserialization concern.
    pub sessions: Vec<ManifestSession>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "run-manifest",
        generate: || schemars::schema_for!(RunManifest),
    }
}

/// A single session entry within a run manifest.
///
/// References a spec by name and optionally overrides harness configuration.
/// The pipeline merges these overrides with spec defaults and project config
/// to produce a complete [`HarnessProfile`](crate::HarnessProfile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManifestSession {
    /// Name of the spec to run (required). Must match a spec file in the project.
    pub spec: String,

    /// Optional display name for this session. Defaults to the spec name if omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional settings overrides applied to this session's harness profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<SettingsOverride>,

    /// Lifecycle hooks for this session. Empty by default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<HookContract>,

    /// Prompt layers injected into this session's prompt assembly. Empty by default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_layers: Vec<PromptLayer>,

    /// Sessions that must complete before this one can start.
    ///
    /// Each entry is an effective session name (i.e. `name` if set, otherwise `spec`)
    /// of another session in the same manifest. Used by the orchestrator to build an
    /// execution DAG. Empty means no dependencies — the session can run immediately.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "manifest-session",
        generate: || schemars::schema_for!(ManifestSession),
    }
}
