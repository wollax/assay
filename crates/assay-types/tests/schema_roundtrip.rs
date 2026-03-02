//! Roundtrip validation tests: serialize known-good instances to JSON,
//! validate against the generated schema for each type.

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
    });
}

#[test]
fn criterion_with_cmd_validates() {
    validate(&Criterion {
        name: "tests pass".to_string(),
        description: "All unit tests pass".to_string(),
        cmd: Some("cargo test".to_string()),
    });
}

#[test]
fn criterion_without_cmd_validates() {
    validate(&Criterion {
        name: "builds cleanly".to_string(),
        description: "The project compiles without warnings".to_string(),
        cmd: None,
    });
}
