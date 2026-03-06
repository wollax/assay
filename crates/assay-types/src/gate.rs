//! Gate types for defining quality checks and capturing their results.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::session::{Confidence, EvaluatorRole};

/// The kind of gate to evaluate.
///
/// Uses internal tagging (`#[serde(tag = "kind")]`) so TOML output includes
/// a `kind = "Command"` or `kind = "AlwaysPass"` discriminator field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum GateKind {
    /// A gate that runs a shell command and checks its exit code.
    Command {
        /// The shell command to execute.
        cmd: String,
    },

    /// A gate that always passes — useful for placeholder or manual gates.
    AlwaysPass,

    /// A gate that checks whether a file exists at the given path.
    FileExists {
        /// Path to check, relative to the working directory.
        path: String,
    },

    /// A gate evaluated by an agent via structured reasoning.
    AgentReport,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-kind",
        generate: || schemars::schema_for!(GateKind),
    }
}

/// The result of evaluating a gate.
///
/// Captures whether the gate passed, the command output (if any), timing,
/// and which kind of gate produced this result (self-describing via `kind`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    /// Whether the gate passed.
    pub passed: bool,

    /// Which gate kind produced this result.
    pub kind: GateKind,

    /// Standard output captured from the gate command.
    /// Omitted from serialized output when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub stdout: String,

    /// Standard error captured from the gate command.
    /// Omitted from serialized output when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub stderr: String,

    /// Exit code from the gate command, if applicable.
    /// Omitted from serialized output when `None` (e.g., for `AlwaysPass` gates).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exit_code: Option<i32>,

    /// How long the gate took to evaluate, in milliseconds.
    pub duration_ms: u64,

    /// When the gate evaluation completed.
    pub timestamp: DateTime<Utc>,

    /// Whether stdout or stderr was truncated due to size limits.
    /// Omitted from serialized output when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,

    /// Original byte count before truncation.
    /// Omitted from serialized output when `None`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_bytes: Option<u64>,

    /// What the agent observed (concrete facts).
    /// Only populated for `AgentReport` gates.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<String>,

    /// Why those facts lead to pass/fail.
    /// Only populated for `AgentReport` gates.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<String>,

    /// Agent's confidence level in the evaluation.
    /// Only populated for `AgentReport` gates.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confidence: Option<Confidence>,

    /// Role of the evaluator who produced this result.
    /// Only populated for `AgentReport` gates.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evaluator_role: Option<EvaluatorRole>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-result",
        generate: || schemars::schema_for!(GateResult),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_kind_command_toml_roundtrip() {
        let kind = GateKind::Command {
            cmd: "cargo test".to_string(),
        };

        let toml_str = toml::to_string(&kind).expect("serialize to TOML");
        assert!(
            toml_str.contains(r#"kind = "Command""#),
            "TOML should contain kind = \"Command\", got:\n{toml_str}"
        );
        assert!(
            toml_str.contains(r#"cmd = "cargo test""#),
            "TOML should contain cmd = \"cargo test\", got:\n{toml_str}"
        );

        let roundtripped: GateKind = toml::from_str(&toml_str).expect("deserialize from TOML");
        let re_serialized = toml::to_string(&roundtripped).expect("re-serialize to TOML");
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_kind_file_exists_toml_roundtrip() {
        let kind = GateKind::FileExists {
            path: "README.md".to_string(),
        };

        let toml_str = toml::to_string(&kind).expect("serialize to TOML");
        assert!(
            toml_str.contains(r#"kind = "FileExists""#),
            "TOML should contain kind = \"FileExists\", got:\n{toml_str}"
        );
        assert!(
            toml_str.contains(r#"path = "README.md""#),
            "TOML should contain path = \"README.md\", got:\n{toml_str}"
        );

        let roundtripped: GateKind = toml::from_str(&toml_str).expect("deserialize from TOML");
        let re_serialized = toml::to_string(&roundtripped).expect("re-serialize to TOML");
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_kind_always_pass_toml_roundtrip() {
        let kind = GateKind::AlwaysPass;

        let toml_str = toml::to_string(&kind).expect("serialize to TOML");
        assert!(
            toml_str.contains(r#"kind = "AlwaysPass""#),
            "TOML should contain kind = \"AlwaysPass\", got:\n{toml_str}"
        );

        let roundtripped: GateKind = toml::from_str(&toml_str).expect("deserialize from TOML");
        let re_serialized = toml::to_string(&roundtripped).expect("re-serialize to TOML");
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_result_json_skips_empty_fields() {
        let result = GateResult {
            passed: true,
            kind: GateKind::AlwaysPass,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        };

        let json = serde_json::to_string(&result).expect("serialize to JSON");
        assert!(
            !json.contains("stdout"),
            "JSON should omit empty stdout, got:\n{json}"
        );
        assert!(
            !json.contains("stderr"),
            "JSON should omit empty stderr, got:\n{json}"
        );
        assert!(
            !json.contains("exit_code"),
            "JSON should omit None exit_code, got:\n{json}"
        );
        assert!(
            !json.contains("truncated"),
            "JSON should omit false truncated, got:\n{json}"
        );
        assert!(
            !json.contains("original_bytes"),
            "JSON should omit None original_bytes, got:\n{json}"
        );
    }

    #[test]
    fn gate_result_json_includes_populated_fields() {
        let result = GateResult {
            passed: true,
            kind: GateKind::Command {
                cmd: "cargo test".to_string(),
            },
            stdout: "all tests passed".to_string(),
            stderr: "warning: unused variable".to_string(),
            exit_code: Some(0),
            duration_ms: 1500,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        };

        let json = serde_json::to_string(&result).expect("serialize to JSON");
        assert!(
            json.contains("stdout"),
            "JSON should include populated stdout, got:\n{json}"
        );
        assert!(
            json.contains("stderr"),
            "JSON should include populated stderr, got:\n{json}"
        );
        assert!(
            json.contains("exit_code"),
            "JSON should include Some exit_code, got:\n{json}"
        );
    }

    #[test]
    fn gate_result_json_includes_truncation_fields_when_populated() {
        let result = GateResult {
            passed: true,
            kind: GateKind::Command {
                cmd: "cargo test".to_string(),
            },
            stdout: "output".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 100,
            timestamp: Utc::now(),
            truncated: true,
            original_bytes: Some(131_072),
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        };

        let json = serde_json::to_string(&result).expect("serialize to JSON");
        assert!(
            json.contains("truncated"),
            "JSON should include truncated when true, got:\n{json}"
        );
        assert!(
            json.contains("original_bytes"),
            "JSON should include original_bytes when Some, got:\n{json}"
        );
    }

    #[test]
    fn gate_kind_agent_report_toml_roundtrip() {
        let kind = GateKind::AgentReport;

        let toml_str = toml::to_string(&kind).expect("serialize to TOML");
        assert!(
            toml_str.contains(r#"kind = "AgentReport""#),
            "TOML should contain kind = \"AgentReport\", got:\n{toml_str}"
        );

        let roundtripped: GateKind = toml::from_str(&toml_str).expect("deserialize from TOML");
        let re_serialized = toml::to_string(&roundtripped).expect("re-serialize to TOML");
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_result_agent_fields_json_skipped_when_none() {
        let result = GateResult {
            passed: true,
            kind: GateKind::AgentReport,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        };

        let json = serde_json::to_string(&result).expect("serialize to JSON");
        assert!(
            !json.contains("evidence"),
            "JSON should omit None evidence, got:\n{json}"
        );
        assert!(
            !json.contains("reasoning"),
            "JSON should omit None reasoning, got:\n{json}"
        );
        assert!(
            !json.contains("confidence"),
            "JSON should omit None confidence, got:\n{json}"
        );
        assert!(
            !json.contains("evaluator_role"),
            "JSON should omit None evaluator_role, got:\n{json}"
        );
    }

    #[test]
    fn gate_result_agent_fields_json_included_when_set() {
        let result = GateResult {
            passed: true,
            kind: GateKind::AgentReport,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 50,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: Some("Found auth module with JWT validation".to_string()),
            reasoning: Some("JWT validation present and tests pass".to_string()),
            confidence: Some(Confidence::High),
            evaluator_role: Some(EvaluatorRole::SelfEval),
        };

        let json = serde_json::to_string(&result).expect("serialize to JSON");
        assert!(json.contains("evidence"), "JSON should include evidence");
        assert!(json.contains("reasoning"), "JSON should include reasoning");
        assert!(
            json.contains("confidence"),
            "JSON should include confidence"
        );
        assert!(
            json.contains("evaluator_role"),
            "JSON should include evaluator_role"
        );
    }

    #[test]
    fn gate_result_agent_json_roundtrip() {
        let result = GateResult {
            passed: false,
            kind: GateKind::AgentReport,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 120,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: Some("No error handling found in auth module".to_string()),
            reasoning: Some("Missing try/catch blocks around DB calls".to_string()),
            confidence: Some(Confidence::Medium),
            evaluator_role: Some(EvaluatorRole::Independent),
        };

        let json = serde_json::to_string_pretty(&result).expect("serialize");
        let roundtripped: GateResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result, roundtripped);
    }

    #[test]
    fn gate_kind_unknown_variant_deser_fails() {
        let toml_str = r#"kind = "Unknown""#;
        let err = toml::from_str::<GateKind>(toml_str);
        assert!(
            err.is_err(),
            "unknown GateKind variant should fail deserialization"
        );
    }

    #[test]
    fn gate_kind_unknown_variant_json_deser_fails() {
        let json = r#"{"kind":"Bogus"}"#;
        let err = serde_json::from_str::<GateKind>(json);
        assert!(
            err.is_err(),
            "unknown GateKind variant in JSON should fail deserialization"
        );
    }

    #[test]
    fn gate_result_json_roundtrip_with_skip_fields() {
        // Test that skip_serializing_if + default pairing roundtrips correctly
        let result = GateResult {
            passed: true,
            kind: GateKind::Command {
                cmd: "echo ok".to_string(),
            },
            stdout: String::new(), // empty — skipped in serialization
            stderr: String::new(), // empty — skipped in serialization
            exit_code: None,       // None — skipped in serialization
            duration_ms: 50,
            timestamp: Utc::now(),
            truncated: false, // false — skipped in serialization
            original_bytes: None,
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        // Verify skip_serializing_if works
        assert!(
            !json.contains("stdout"),
            "empty stdout should be omitted: {json}"
        );
        assert!(
            !json.contains("exit_code"),
            "None exit_code should be omitted: {json}"
        );
        assert!(
            !json.contains("truncated"),
            "false truncated should be omitted: {json}"
        );

        // Verify roundtrip back to original
        let roundtripped: GateResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result, roundtripped);
    }
}
