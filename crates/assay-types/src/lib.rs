pub mod criterion;
pub mod gate;
pub mod schema_registry;

pub use criterion::Criterion;
pub use gate::{GateKind, GateResult};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A specification that defines what should be built and its acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Spec {
    pub name: String,
    pub description: String,
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
    pub specs: Vec<Spec>,
    pub gates: Vec<Gate>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "workflow",
        generate: || schemars::schema_for!(Workflow),
    }
}

/// Top-level configuration for Assay.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub project_name: String,
    pub workflows: Vec<Workflow>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "config",
        generate: || schemars::schema_for!(Config),
    }
}
