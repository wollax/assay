//! Roundtrip validation tests: serialize known-good instances to JSON,
//! validate against the generated schema for each type.

use assay_types::criterion::When;
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
        gate: None,
        depends: vec![],
        criteria: vec![Criterion {
            name: "compiles".to_string(),
            description: "The project compiles".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
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
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "compiles".to_string(),
                description: "The project compiles".to_string(),
                cmd: None,
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::default(),
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
            max_history: None,
            evaluator_model: "sonnet".to_string(),
            evaluator_retries: 1,
            evaluator_timeout: 120,
        }),
        guard: None,
        worktree: None,
        sessions: None,
        provider: None,
    });
}

#[test]
fn gates_config_validates() {
    validate(&assay_types::GatesConfig {
        default_timeout: 300,
        working_dir: None,
        max_history: None,
        evaluator_model: "sonnet".to_string(),
        evaluator_retries: 1,
        evaluator_timeout: 120,
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
        evidence: None,
        reasoning: None,
        confidence: None,
        evaluator_role: None,
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
        evidence: None,
        reasoning: None,
        confidence: None,
        evaluator_role: None,
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
        enforcement: None,
        kind: None,
        prompt: None,
        requirements: vec![],
        when: When::default(),
    });
}

#[test]
fn criterion_without_cmd_validates() {
    validate(&Criterion {
        name: "builds cleanly".to_string(),
        description: "The project compiles without warnings".to_string(),
        cmd: None,
        path: None,
        timeout: None,
        enforcement: None,
        kind: None,
        prompt: None,
        requirements: vec![],
        when: When::default(),
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
        enforcement: None,
        kind: None,
        prompt: None,
        requirements: vec![],
        when: When::default(),
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
        evidence: None,
        reasoning: None,
        confidence: None,
        evaluator_role: None,
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
        auto_promote: false,
    });
}

#[test]
fn gates_spec_validates() {
    validate(&GatesSpec {
        name: "auth-flow".to_string(),
        description: String::new(),
        gate: None,
        depends: vec![],
        milestone: None,
        order: None,
        criteria: vec![GateCriterion {
            name: "auth-compiles".to_string(),
            description: "Auth module compiles".to_string(),
            cmd: Some("cargo build -p auth".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec!["REQ-FUNC-001".to_string()],
            when: When::default(),
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
        enforcement: None,
        kind: None,
        prompt: None,
        requirements: vec![],
        when: When::default(),
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
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
        }],
        passed: 1,
        failed: 0,
        skipped: 0,
        total_duration_ms: 1500,
        enforcement: EnforcementSummary::default(),
    });
}

#[test]
fn gate_run_summary_with_skipped_criterion_validates() {
    validate(&GateRunSummary {
        spec_name: "test-spec".to_string(),
        results: vec![CriterionResult {
            criterion_name: "descriptive-only".to_string(),
            result: None,
            enforcement: Enforcement::Required,
        }],
        passed: 0,
        failed: 0,
        skipped: 1,
        total_duration_ms: 0,
        enforcement: EnforcementSummary::default(),
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
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        }),
        enforcement: Enforcement::Required,
    });
}

#[test]
fn criterion_result_skipped_validates() {
    validate(&CriterionResult {
        criterion_name: "manual-check".to_string(),
        result: None,
        enforcement: Enforcement::Required,
    });
}

#[test]
fn enforcement_roundtrip() {
    // Required (default)
    let req = Enforcement::Required;
    let json = serde_json::to_string(&req).unwrap();
    assert_eq!(json, r#""required""#);
    let back: Enforcement = serde_json::from_str(&json).unwrap();
    assert_eq!(back, req);

    // Advisory
    let adv = Enforcement::Advisory;
    let json = serde_json::to_string(&adv).unwrap();
    assert_eq!(json, r#""advisory""#);
    let back: Enforcement = serde_json::from_str(&json).unwrap();
    assert_eq!(back, adv);

    validate(&req);
    validate(&adv);
}

#[test]
fn gate_section_roundtrip() {
    let section = GateSection {
        enforcement: Enforcement::Advisory,
    };
    let json = serde_json::to_string(&section).unwrap();
    let back: GateSection = serde_json::from_str(&json).unwrap();
    assert_eq!(back.enforcement, Enforcement::Advisory);
    validate(&section);
}

#[test]
fn enforcement_summary_roundtrip() {
    let summary = EnforcementSummary {
        required_passed: 3,
        required_failed: 1,
        advisory_passed: 2,
        advisory_failed: 0,
    };
    let json = serde_json::to_string(&summary).unwrap();
    let back: EnforcementSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back, summary);
    validate(&summary);
}

#[test]
fn spec_with_gate_section_toml_roundtrip() {
    let toml_str = r#"
name = "test"
description = "A test spec"

[gate]
enforcement = "advisory"

[[criteria]]
name = "lint"
description = "Run linter"
cmd = "echo lint"
"#;
    let spec: Spec = toml::from_str(toml_str).expect("should parse TOML with [gate] section");
    assert_eq!(
        spec.gate.as_ref().unwrap().enforcement,
        Enforcement::Advisory
    );
    validate(&spec);
}

#[test]
fn spec_without_enforcement_fields_backward_compat() {
    // Old-style spec without enforcement or gate section still parses
    let toml_str = r#"
name = "legacy"
description = "No enforcement fields"

[[criteria]]
name = "build"
description = "Build it"
cmd = "make"
"#;
    let spec: Spec = toml::from_str(toml_str).expect("old TOML without enforcement should parse");
    assert!(spec.gate.is_none());
    assert!(spec.criteria[0].enforcement.is_none());
    validate(&spec);
}

#[test]
fn gate_run_summary_backward_compat_no_enforcement() {
    // Old JSON without enforcement field still deserializes (serde default)
    let json = r#"{
        "spec_name": "old-spec",
        "passed": 1,
        "failed": 0,
        "skipped": 0,
        "total_duration_ms": 100
    }"#;
    let summary: GateRunSummary =
        serde_json::from_str(json).expect("old JSON without enforcement should parse");
    assert_eq!(summary.enforcement.required_passed, 0);
    assert_eq!(summary.enforcement.required_failed, 0);
    validate(&summary);
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

#[test]
fn worktree_config_validates() {
    validate(&WorktreeConfig {
        base_dir: "../my-worktrees".to_string(),
    });
}

#[test]
fn worktree_config_empty_base_dir_validates() {
    validate(&WorktreeConfig {
        base_dir: String::new(),
    });
}

#[test]
fn worktree_info_validates() {
    validate(&WorktreeInfo {
        spec_slug: "auth-flow".to_string(),
        path: std::path::PathBuf::from("/tmp/worktrees/auth-flow"),
        branch: "assay/auth-flow".to_string(),
        base_branch: Some("main".to_string()),
        is_orphan: false,
    });
}

#[test]
fn worktree_info_without_base_branch_validates() {
    validate(&WorktreeInfo {
        spec_slug: "auth-flow".to_string(),
        path: std::path::PathBuf::from("/tmp/worktrees/auth-flow"),
        branch: "assay/auth-flow".to_string(),
        base_branch: None,
        is_orphan: false,
    });
}

#[test]
fn worktree_status_validates() {
    validate(&WorktreeStatus {
        spec_slug: "auth-flow".to_string(),
        path: std::path::PathBuf::from("/tmp/worktrees/auth-flow"),
        branch: "assay/auth-flow".to_string(),
        head: "abc1234".to_string(),
        dirty: true,
        ahead: Some(3),
        behind: Some(1),
        base_branch: Some("main".to_string()),
        warnings: vec![],
    });
}

#[test]
fn worktree_status_none_ahead_behind_validates() {
    validate(&WorktreeStatus {
        spec_slug: "auth-flow".to_string(),
        path: std::path::PathBuf::from("/tmp/worktrees/auth-flow"),
        branch: "assay/auth-flow".to_string(),
        head: "abc1234".to_string(),
        dirty: false,
        ahead: None,
        behind: None,
        base_branch: None,
        warnings: vec!["no worktree metadata found — ahead/behind unavailable".to_string()],
    });
}

#[test]
fn worktree_metadata_validates() {
    validate(&WorktreeMetadata {
        base_branch: "main".to_string(),
        spec_slug: "auth-flow".to_string(),
        session_id: None,
    });
}

// ── Scope enforcement types ──────────────────────────────────────────

#[test]
fn scope_violation_out_of_scope_validates() {
    validate(&ScopeViolation {
        file: "src/main.rs".to_string(),
        violation_type: ScopeViolationType::OutOfScope,
        pattern: "crates/auth/**".to_string(),
    });
}

#[test]
fn scope_violation_shared_file_conflict_validates() {
    validate(&ScopeViolation {
        file: "Cargo.lock".to_string(),
        violation_type: ScopeViolationType::SharedFileConflict,
        pattern: "Cargo.lock".to_string(),
    });
}

#[test]
fn scope_violation_json_roundtrip() {
    let original = ScopeViolation {
        file: "src/lib.rs".to_string(),
        violation_type: ScopeViolationType::OutOfScope,
        pattern: "tests/**".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: ScopeViolation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, restored);
}

// ── Manifest session types ───────────────────────────────────────────

#[test]
fn manifest_session_with_scope_validates() {
    validate(&ManifestSession {
        spec: "auth-flow".to_string(),
        name: Some("auth-scoped".to_string()),
        settings: None,
        hooks: vec![],
        prompt_layers: vec![],
        file_scope: vec!["crates/auth/**".to_string(), "tests/auth_*".to_string()],
        shared_files: vec!["Cargo.lock".to_string(), "Cargo.toml".to_string()],
        depends_on: vec![],
        user_prompt: None,
        prompt_file: None,
    });
}

#[test]
fn manifest_session_without_scope_validates() {
    validate(&ManifestSession {
        spec: "checkout".to_string(),
        name: None,
        settings: None,
        hooks: vec![],
        prompt_layers: vec![],
        file_scope: vec![],
        shared_files: vec![],
        depends_on: vec![],
        user_prompt: None,
        prompt_file: None,
    });
}

#[test]
fn manifest_session_toml_backward_compat() {
    // A TOML manifest without file_scope/shared_files should still parse
    let toml_str = r#"
spec = "auth-flow"
"#;
    let session: ManifestSession = toml::from_str(toml_str).expect("parse without scope fields");
    assert_eq!(session.spec, "auth-flow");
    assert!(session.file_scope.is_empty());
    assert!(session.shared_files.is_empty());
}

#[test]
fn manifest_session_toml_with_scope_roundtrip() {
    let toml_str = r#"
spec = "auth-flow"
file_scope = ["crates/auth/**", "tests/auth_*"]
shared_files = ["Cargo.lock"]
"#;
    let session: ManifestSession = toml::from_str(toml_str).expect("parse with scope fields");
    assert_eq!(session.file_scope, vec!["crates/auth/**", "tests/auth_*"]);
    assert_eq!(session.shared_files, vec!["Cargo.lock"]);
}

#[cfg(feature = "orchestrate")]
#[test]
fn run_manifest_with_scoped_sessions_validates() {
    validate(&RunManifest {
        sessions: vec![
            ManifestSession {
                spec: "auth".to_string(),
                name: None,
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec!["crates/auth/**".to_string()],
                shared_files: vec!["Cargo.lock".to_string()],
                depends_on: vec![],
                user_prompt: None,
                prompt_file: None,
            },
            ManifestSession {
                spec: "checkout".to_string(),
                name: None,
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec!["auth".to_string()],
                user_prompt: None,
                prompt_file: None,
            },
        ],
        mode: assay_types::orchestrate::OrchestratorMode::Dag,
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    });
}
