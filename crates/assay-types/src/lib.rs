pub mod context;
pub mod criterion;
pub mod enforcement;
pub mod feature_spec;
pub mod gate;
pub mod gate_run;
pub mod gates_spec;
pub mod schema_registry;
pub mod session;

pub use context::{
    BloatCategory, ContextHealth, DiagnosticsReport, SessionEntry, SessionInfo, TokenEstimate,
    UsageData,
};
pub use criterion::{Criterion, CriterionKind};
pub use enforcement::{Enforcement, EnforcementSummary, GateSection};
pub use feature_spec::FeatureSpec;
pub use gate::{GateKind, GateResult};
pub use gate_run::{CriterionResult, GateRunRecord, GateRunSummary};
pub use gates_spec::{GateCriterion, GatesSpec};
pub use session::{AgentEvaluation, AgentSession, Confidence, EvaluatorRole};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A specification that defines what should be built and its acceptance criteria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Display name for this spec (required, must be unique across all specs).
    pub name: String,

    /// Human-readable description. Defaults to empty string, omitted from
    /// serialized output when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Gate configuration section (enforcement defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSection>,

    /// Acceptance criteria that must be satisfied.
    pub criteria: Vec<Criterion>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "spec",
        generate: || schemars::schema_for!(Spec),
    }
}

/// A quality gate that must pass before work proceeds.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Gate {
    pub name: String,
    pub passed: bool,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "gate",
        generate: || schemars::schema_for!(Gate),
    }
}

/// A review of completed work against a spec.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Review {
    pub spec_name: String,
    pub approved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "review",
        generate: || schemars::schema_for!(Review),
    }
}

/// A workflow combining specs, gates, and reviews into a development pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specs: Vec<Spec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<Gate>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "workflow",
        generate: || schemars::schema_for!(Workflow),
    }
}

/// Top-level configuration for an Assay project.
///
/// Loaded from `.assay/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Project name (required, non-empty after trim).
    pub project_name: String,

    /// Directory containing spec files, relative to `.assay/`.
    /// Defaults to `"specs/"`.
    #[serde(default = "default_specs_dir")]
    pub specs_dir: String,

    /// Gate execution configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<GatesConfig>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "config",
        generate: || schemars::schema_for!(Config),
    }
}

/// Gate execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatesConfig {
    /// Default timeout for gate commands in seconds. Defaults to 300.
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,

    /// Working directory for gate execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Maximum number of run history files to retain per spec.
    /// When set, oldest files beyond this limit are pruned on each save.
    /// A value of `0` means unlimited (no pruning). Defaults to `None` (no pruning).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_history: Option<usize>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "gates-config",
        generate: || schemars::schema_for!(GatesConfig),
    }
}

fn default_specs_dir() -> String {
    "specs/".to_string()
}

fn default_timeout() -> u64 {
    300
}
