use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A specification that defines what should be built and its acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Spec {
    pub name: String,
    pub description: String,
}

/// A quality gate that must pass before work proceeds.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Gate {
    pub name: String,
    pub passed: bool,
}

/// A review of completed work against a spec.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Review {
    pub spec_name: String,
    pub approved: bool,
    pub comments: Vec<String>,
}

/// A workflow combining specs, gates, and reviews into a development pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    pub name: String,
    pub specs: Vec<Spec>,
    pub gates: Vec<Gate>,
}

/// Top-level configuration for Assay.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub project_name: String,
    pub workflows: Vec<Workflow>,
}
