//! Evaluator output types for the Claude Code evaluator subprocess.
//!
//! These types define the JSON Schema contract between the Claude Code
//! evaluator subprocess and assay. The `EvaluatorOutput` struct is used
//! with `--json-schema` to validate structured output from the evaluator.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Outcome of evaluating a single criterion.
///
/// Four-state outcome: pass, fail, skip (could not assess), or warn (soft concern).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CriterionOutcome {
    /// Criterion satisfied.
    Pass,
    /// Criterion not satisfied.
    Fail,
    /// Evaluator could not assess this criterion.
    Skip,
    /// Soft concern — does not fail the gate.
    Warn,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion-outcome",
        generate: || schemars::schema_for!(CriterionOutcome),
    }
}

/// Per-criterion evaluation result from the evaluator subprocess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorCriterionResult {
    /// Criterion name (must match a criterion in the spec).
    pub name: String,
    /// Four-state outcome of the evaluation.
    pub outcome: CriterionOutcome,
    /// Free-text reasoning explaining the judgment.
    pub reasoning: String,
    /// Concrete evidence observed (optional but encouraged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "evaluator-criterion-result",
        generate: || schemars::schema_for!(EvaluatorCriterionResult),
    }
}

/// Aggregate summary from the evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorSummary {
    /// Whether the gate passed overall.
    pub passed: bool,
    /// Brief rationale for the overall judgment.
    pub rationale: String,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "evaluator-summary",
        generate: || schemars::schema_for!(EvaluatorSummary),
    }
}

/// Complete evaluator output: per-criterion results plus overall summary.
///
/// This struct defines the JSON Schema used with the `--json-schema` flag
/// when spawning the Claude Code evaluator subprocess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorOutput {
    /// Per-criterion evaluation results.
    pub criteria: Vec<EvaluatorCriterionResult>,
    /// Overall summary with aggregate judgment.
    pub summary: EvaluatorSummary,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "evaluator-output",
        generate: || schemars::schema_for!(EvaluatorOutput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_produces_valid_json() {
        let schema = schemars::schema_for!(EvaluatorOutput);
        let json = serde_json::to_string(&schema).expect("schema should serialize to JSON");
        assert!(!json.is_empty(), "schema JSON should be non-empty");

        // Verify it's valid JSON by parsing back
        let _: serde_json::Value =
            serde_json::from_str(&json).expect("schema JSON should parse back");
    }

    #[test]
    fn criterion_outcome_serde_roundtrip() {
        for outcome in [
            CriterionOutcome::Pass,
            CriterionOutcome::Fail,
            CriterionOutcome::Skip,
            CriterionOutcome::Warn,
        ] {
            let json = serde_json::to_string(&outcome).expect("serialize");
            let roundtripped: CriterionOutcome =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(outcome, roundtripped);
        }
    }

    #[test]
    fn criterion_outcome_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&CriterionOutcome::Pass).unwrap(),
            "\"pass\""
        );
        assert_eq!(
            serde_json::to_string(&CriterionOutcome::Fail).unwrap(),
            "\"fail\""
        );
        assert_eq!(
            serde_json::to_string(&CriterionOutcome::Skip).unwrap(),
            "\"skip\""
        );
        assert_eq!(
            serde_json::to_string(&CriterionOutcome::Warn).unwrap(),
            "\"warn\""
        );
    }

    #[test]
    fn evaluator_output_serde_roundtrip() {
        let output = EvaluatorOutput {
            criteria: vec![
                EvaluatorCriterionResult {
                    name: "tests-pass".to_string(),
                    outcome: CriterionOutcome::Pass,
                    reasoning: "All unit tests pass".to_string(),
                    evidence: Some("42 tests passed, 0 failed".to_string()),
                },
                EvaluatorCriterionResult {
                    name: "code-quality".to_string(),
                    outcome: CriterionOutcome::Warn,
                    reasoning: "Minor style issues found".to_string(),
                    evidence: None,
                },
            ],
            summary: EvaluatorSummary {
                passed: true,
                rationale: "All required criteria pass".to_string(),
            },
        };

        let json = serde_json::to_string_pretty(&output).expect("serialize");
        let roundtripped: EvaluatorOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(output, roundtripped);
    }

    #[test]
    fn evaluator_output_evidence_omitted_when_none() {
        let result = EvaluatorCriterionResult {
            name: "test".to_string(),
            outcome: CriterionOutcome::Skip,
            reasoning: "Could not assess".to_string(),
            evidence: None,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        assert!(
            !json.contains("evidence"),
            "JSON should omit None evidence, got: {json}"
        );
    }

    #[test]
    fn evaluator_output_evidence_included_when_some() {
        let result = EvaluatorCriterionResult {
            name: "test".to_string(),
            outcome: CriterionOutcome::Pass,
            reasoning: "Looks good".to_string(),
            evidence: Some("Found the implementation".to_string()),
        };

        let json = serde_json::to_string(&result).expect("serialize");
        assert!(
            json.contains("evidence"),
            "JSON should include evidence when Some, got: {json}"
        );
    }
}
