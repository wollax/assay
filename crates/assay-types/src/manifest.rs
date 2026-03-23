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
#[cfg(feature = "orchestrate")]
use crate::orchestrate::{GossipConfig, MeshConfig, OrchestratorMode};

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    /// Ordered list of sessions to execute.
    ///
    /// Maps to `[[sessions]]` in TOML. At least one session is expected,
    /// but emptiness is a validation concern, not a deserialization concern.
    pub sessions: Vec<ManifestSession>,

    /// Coordination mode for this run. Defaults to `dag` (existing behavior).
    #[cfg(feature = "orchestrate")]
    #[serde(default)]
    pub mode: OrchestratorMode,

    /// Mesh mode configuration. Ignored unless `mode = "mesh"`.
    #[cfg(feature = "orchestrate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_config: Option<MeshConfig>,

    /// Gossip mode configuration. Ignored unless `mode = "gossip"`.
    #[cfg(feature = "orchestrate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gossip_config: Option<GossipConfig>,
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

    /// Glob patterns defining the file scope for this session.
    ///
    /// When set, the harness enforces that the agent only modifies files
    /// matching these patterns. Empty means no scope restriction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_scope: Vec<String>,

    /// Glob patterns for files shared with other sessions.
    ///
    /// Files matching these patterns are expected to be touched by multiple
    /// agents. The harness uses this to detect shared-file conflicts during
    /// scope enforcement. Empty means no shared files declared.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_files: Vec<String>,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_session_with_scope_fields_toml_round_trip() {
        let toml_str = r#"
            [[sessions]]
            spec = "auth"
            file_scope = ["src/auth/**", "tests/auth/**"]
            shared_files = ["src/shared/config.rs"]
        "#;
        let manifest: RunManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.sessions.len(), 1);
        let session = &manifest.sessions[0];
        assert_eq!(session.file_scope, vec!["src/auth/**", "tests/auth/**"]);
        assert_eq!(session.shared_files, vec!["src/shared/config.rs"]);
    }

    #[test]
    fn manifest_session_without_scope_fields_backward_compat() {
        let toml_str = r#"
            [[sessions]]
            spec = "checkout"
            name = "checkout-session"
        "#;
        let manifest: RunManifest = toml::from_str(toml_str).unwrap();
        let session = &manifest.sessions[0];
        assert_eq!(session.spec, "checkout");
        assert!(session.file_scope.is_empty());
        assert!(session.shared_files.is_empty());
    }

    #[test]
    fn manifest_session_scope_fields_omitted_when_empty_in_toml() {
        let session = ManifestSession {
            spec: "test".to_string(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
        };
        let manifest = RunManifest {
            sessions: vec![session],
            ..Default::default()
        };
        let toml_out = toml::to_string(&manifest).unwrap();
        assert!(!toml_out.contains("file_scope"));
        assert!(!toml_out.contains("shared_files"));
    }

    #[cfg(feature = "orchestrate")]
    #[test]
    fn manifest_without_mode_defaults_to_dag() {
        let toml_str = "[[sessions]]\nspec = \"auth\"\n";
        let manifest: RunManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.mode, OrchestratorMode::Dag);
        assert!(manifest.mesh_config.is_none());
        assert!(manifest.gossip_config.is_none());
    }

    #[cfg(feature = "orchestrate")]
    #[test]
    fn manifest_with_mode_mesh_parses() {
        let toml_str = "mode = \"mesh\"\n[[sessions]]\nspec = \"auth\"\n";
        let manifest: RunManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.mode, OrchestratorMode::Mesh);
    }

    #[cfg(feature = "orchestrate")]
    #[test]
    fn manifest_with_mode_gossip_parses() {
        let toml_str = "mode = \"gossip\"\n[[sessions]]\nspec = \"auth\"\n";
        let manifest: RunManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.mode, OrchestratorMode::Gossip);
    }

    #[cfg(feature = "orchestrate")]
    #[test]
    fn manifest_mode_round_trip() {
        let manifest = RunManifest {
            sessions: vec![ManifestSession {
                spec: "auth".to_string(),
                name: None,
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec![],
            }],
            mode: OrchestratorMode::Dag,
            mesh_config: None,
            gossip_config: None,
        };
        let toml_out = toml::to_string(&manifest).unwrap();
        let back: RunManifest = toml::from_str(&toml_out).unwrap();
        assert_eq!(back.mode, OrchestratorMode::Dag);
    }

    #[cfg(feature = "orchestrate")]
    #[test]
    fn manifest_mesh_config_omitted_when_none() {
        let manifest = RunManifest {
            sessions: vec![ManifestSession {
                spec: "auth".to_string(),
                name: None,
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec![],
            }],
            mode: OrchestratorMode::Mesh,
            mesh_config: None,
            gossip_config: None,
        };
        let toml_out = toml::to_string(&manifest).unwrap();
        assert!(!toml_out.contains("mesh_config"));
        assert!(!toml_out.contains("gossip_config"));
    }
}
