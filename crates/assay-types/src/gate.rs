//! Gate types for defining quality checks and capturing their results.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
}

/// The result of evaluating a gate.
///
/// Captures whether the gate passed, the command output (if any), timing,
/// and which kind of gate produced this result (self-describing via `kind`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
}
