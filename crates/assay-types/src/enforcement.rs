//! Enforcement level types for gate criteria.
//!
//! Determines whether a criterion failure blocks the gate (required)
//! or is informational only (advisory).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Enforcement level for a gate criterion.
///
/// Determines whether a criterion failure blocks the gate (required)
/// or is informational only (advisory).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    /// Failure blocks the gate. This is the default.
    #[default]
    Required,
    /// Failure is informational; does not block the gate.
    Advisory,
}

impl std::fmt::Display for Enforcement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required => write!(f, "required"),
            Self::Advisory => write!(f, "advisory"),
        }
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement",
        generate: || schemars::schema_for!(Enforcement),
    }
}

/// Gate-level configuration section.
///
/// Parsed from `[gate]` in spec TOML files. Provides spec-wide defaults
/// that individual criteria can override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateSection {
    /// Default enforcement level for all criteria in this spec.
    #[serde(default)]
    pub enforcement: Enforcement,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-section",
        generate: || schemars::schema_for!(GateSection),
    }
}

/// Enforcement breakdown in a gate run summary.
///
/// Always present on `GateRunSummary`, with counts defaulting to 0.
/// Only counts executable criteria (skipped criteria are excluded).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EnforcementSummary {
    pub required_passed: usize,
    pub required_failed: usize,
    pub advisory_passed: usize,
    pub advisory_failed: usize,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement-summary",
        generate: || schemars::schema_for!(EnforcementSummary),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcement_invalid_value_deser_fails() {
        let result = toml::from_str::<Enforcement>(r#""strict""#);
        assert!(
            result.is_err(),
            "invalid enforcement value 'strict' should fail deserialization"
        );
    }

    #[test]
    fn enforcement_toml_roundtrip_via_gate_section() {
        // TOML cannot serialize a bare enum at top level — use GateSection as wrapper.
        let section = GateSection {
            enforcement: Enforcement::Advisory,
        };
        let toml_str = toml::to_string(&section).expect("serialize");
        assert!(
            toml_str.contains("advisory"),
            "TOML should contain advisory, got:\n{toml_str}"
        );
        let roundtripped: GateSection = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(section, roundtripped);

        // Also test Required
        let section_req = GateSection {
            enforcement: Enforcement::Required,
        };
        let toml_str_req = toml::to_string(&section_req).expect("serialize");
        let roundtripped_req: GateSection = toml::from_str(&toml_str_req).expect("deserialize");
        assert_eq!(section_req, roundtripped_req);
    }

    #[test]
    fn enforcement_json_roundtrip() {
        for e in [Enforcement::Required, Enforcement::Advisory] {
            let json = serde_json::to_string(&e).expect("serialize");
            let roundtripped: Enforcement = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(e, roundtripped);
        }
    }

    #[test]
    fn enforcement_display_matches_serde() {
        assert_eq!(Enforcement::Required.to_string(), "required");
        assert_eq!(Enforcement::Advisory.to_string(), "advisory");
    }
}
