//! Roundtrip validation tests: serialize known-good instances to JSON,
//! validate against the generated schema for each type.

use assay_types::feature_spec::*;
use assay_types::*;
use chrono::Utc;

/// Helper: generate schema for a type and compile a Draft 2020-12 validator.
fn validate<T: schemars::JsonSchema + serde::Serialize>(instance: &T) {
    let schema = schemars::schema_for!(T);
    let schema_value = schema.to_value();
    let validator = jsonschema::draft202012::new(&schema_value)
        .expect("schema should compile as Draft 2020-12");

    let instance_json = serde_json::to_value(instance).expect("instance should serialize to JSON");

    let errors: Vec<_> = validator.iter_errors(&instance_json).collect();
    if !errors.is_empty() {
        panic!(
            "Instance of {} failed schema validation:\n{}",
            std::any::type_name::<T>(),
            errors
                .iter()
                .map(|e| format!("  - {} at {}", e, e.instance_path()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn spec_validates() {
    validate(&Spec {
        name: "build-feature".to_string(),
        description: "Implement the login page".to_string(),
        criteria: vec![Criterion {
            name: "compiles".to_string(),
            description: "The project compiles".to_string(),
            cmd: None,
            path: None,
            timeout: None,
        }],
    });
}

#[test]
fn gate_validates() {
    validate(&Gate {
        name: "lint-check".to_string(),
        passed: true,
    });
}

#[test]
fn review_validates() {
    validate(&Review {
        spec_name: "build-feature".to_string(),
        approved: true,
        comments: vec!["Looks good!".to_string(), "Minor nit on naming".to_string()],
    });
}

#[test]
fn workflow_validates() {
    validate(&Workflow {
        name: "ci-pipeline".to_string(),
        specs: vec![Spec {
            name: "build-feature".to_string(),
            description: "Implement the login page".to_string(),
            criteria: vec![Criterion {
                name: "compiles".to_string(),
                description: "The project compiles".to_string(),
                cmd: None,
                timeout: None,
            }],
        }],
        gates: vec![Gate {
            name: "lint-check".to_string(),
            passed: true,
        }],
    });
}

#[test]
fn config_validates() {
    validate(&Config {
        project_name: "test-project".to_string(),
        specs_dir: "specs/".to_string(),
        gates: Some(assay_types::GatesConfig {
            default_timeout: 300,
            working_dir: Some(".".to_string()),
        }),
    });
}

#[test]
fn gates_config_validates() {
    validate(&assay_types::GatesConfig {
        default_timeout: 300,
        working_dir: None,
    });
}

#[test]
fn gate_kind_command_validates() {
    validate(&GateKind::Command {
        cmd: "cargo test".to_string(),
    });
}

#[test]
fn gate_kind_always_pass_validates() {
    validate(&GateKind::AlwaysPass);
}

#[test]
fn gate_result_full_validates() {
    validate(&GateResult {
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
    });
}

#[test]
fn gate_result_minimal_validates() {
    validate(&GateResult {
        passed: true,
        kind: GateKind::AlwaysPass,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        duration_ms: 0,
        timestamp: Utc::now(),
        truncated: false,
        original_bytes: None,
    });
}

#[test]
fn criterion_with_cmd_validates() {
    validate(&Criterion {
        name: "tests pass".to_string(),
        description: "All unit tests pass".to_string(),
        cmd: Some("cargo test".to_string()),
        path: None,
        timeout: None,
    });
}

#[test]
fn criterion_without_cmd_validates() {
    validate(&Criterion {
        name: "builds cleanly".to_string(),
        description: "The project compiles without warnings".to_string(),
        cmd: None,
        timeout: None,
    });
}

#[test]
fn criterion_with_timeout_validates() {
    validate(&Criterion {
        name: "slow test".to_string(),
        description: "Integration tests with timeout".to_string(),
        cmd: Some("cargo test -- --ignored".to_string()),
        path: None,
        timeout: Some(60),
    });
}

#[test]
fn gate_kind_file_exists_validates() {
    validate(&GateKind::FileExists {
        path: "README.md".to_string(),
    });
}

#[test]
fn gate_result_truncated_validates() {
    validate(&GateResult {
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
    });
}

#[test]
fn feature_spec_minimal_validates() {
    validate(&FeatureSpec {
        name: "auth-flow".to_string(),
        status: SpecStatus::Draft,
        version: "0.1".to_string(),
        overview: None,
        constraints: None,
        users: vec![],
        requirements: vec![Requirement {
            id: "REQ-FUNC-001".to_string(),
            title: "Login".to_string(),
            statement: "The system SHALL authenticate users".to_string(),
            rationale: String::new(),
            obligation: Obligation::Shall,
            priority: Priority::Must,
            verification: VerificationMethod::Test,
            status: SpecStatus::Draft,
            acceptance_criteria: vec![],
        }],
        quality: None,
        assumptions: vec![],
        dependencies: vec![],
        risks: vec![],
        verification: None,
    });
}

#[test]
fn gates_spec_validates() {
    validate(&GatesSpec {
        name: "auth-flow".to_string(),
        description: String::new(),
        criteria: vec![GateCriterion {
            name: "auth-compiles".to_string(),
            description: "Auth module compiles".to_string(),
            cmd: Some("cargo build -p auth".to_string()),
            path: None,
            timeout: None,
            requirements: vec!["REQ-FUNC-001".to_string()],
        }],
    });
}

#[test]
fn gate_criterion_without_cmd_validates() {
    validate(&GateCriterion {
        name: "password-policy".to_string(),
        description: "Password hashing meets requirements".to_string(),
        cmd: None,
        path: None,
        timeout: None,
        requirements: vec![],
    });
}

#[test]
fn gate_run_summary_full_validates() {
    validate(&GateRunSummary {
        spec_name: "test-spec".to_string(),
        results: vec![CriterionResult {
            criterion_name: "unit-tests".to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::Command {
                    cmd: "cargo test".to_string(),
                },
                stdout: "all tests passed".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                duration_ms: 1500,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
            }),
        }],
        passed: 1,
        failed: 0,
        skipped: 0,
        total_duration_ms: 1500,
    });
}

#[test]
fn gate_run_summary_with_skipped_criterion_validates() {
    validate(&GateRunSummary {
        spec_name: "test-spec".to_string(),
        results: vec![CriterionResult {
            criterion_name: "descriptive-only".to_string(),
            result: None,
        }],
        passed: 0,
        failed: 0,
        skipped: 1,
        total_duration_ms: 0,
    });
}

#[test]
fn criterion_result_with_result_validates() {
    validate(&CriterionResult {
        criterion_name: "compiles".to_string(),
        result: Some(GateResult {
            passed: true,
            kind: GateKind::AlwaysPass,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
        }),
    });
}

#[test]
fn criterion_result_skipped_validates() {
    validate(&CriterionResult {
        criterion_name: "manual-check".to_string(),
        result: None,
    });
}

#[test]
fn gate_run_summary_backward_compat_deserialize() {
    // Verify TYPE-03: a minimal JSON blob missing optional/defaultable fields
    // still deserializes successfully (serde(default) on results vec).
    let minimal_json = r#"{
        "spec_name": "legacy-spec",
        "passed": 0,
        "failed": 0,
        "skipped": 0,
        "total_duration_ms": 0
    }"#;

    let summary: GateRunSummary =
        serde_json::from_str(minimal_json).expect("minimal JSON should deserialize");
    assert!(
        summary.results.is_empty(),
        "results should default to empty vec"
    );
    assert_eq!(summary.spec_name, "legacy-spec");

    // Also validate the deserialized instance against the schema
    validate(&summary);
}
