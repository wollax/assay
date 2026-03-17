//! Snapshot tests for schema determinism.
//!
//! Each test generates a schema and asserts it matches the stored snapshot.
//! If schemas change, run `cargo insta review` to accept or reject updates.

use insta::assert_json_snapshot;

#[test]
fn spec_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Spec);
    assert_json_snapshot!("spec-schema", schema.to_value());
}

#[test]
fn gate_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Gate);
    assert_json_snapshot!("gate-schema", schema.to_value());
}

#[test]
fn review_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Review);
    assert_json_snapshot!("review-schema", schema.to_value());
}

#[test]
fn workflow_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Workflow);
    assert_json_snapshot!("workflow-schema", schema.to_value());
}

#[test]
fn config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Config);
    assert_json_snapshot!("config-schema", schema.to_value());
}

#[test]
fn gates_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GatesConfig);
    assert_json_snapshot!("gates-config-schema", schema.to_value());
}

#[test]
fn gate_kind_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateKind);
    assert_json_snapshot!("gate-kind-schema", schema.to_value());
}

#[test]
fn gate_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateResult);
    assert_json_snapshot!("gate-result-schema", schema.to_value());
}

#[test]
fn criterion_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Criterion);
    assert_json_snapshot!("criterion-schema", schema.to_value());
}

#[test]
fn feature_spec_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::FeatureSpec);
    assert_json_snapshot!("feature-spec-schema", schema.to_value());
}

#[test]
fn gates_spec_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GatesSpec);
    assert_json_snapshot!("gates-spec-schema", schema.to_value());
}

#[test]
fn gate_criterion_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateCriterion);
    assert_json_snapshot!("gate-criterion-schema", schema.to_value());
}

#[test]
fn gate_run_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateRunSummary);
    assert_json_snapshot!("gate-run-summary-schema", schema.to_value());
}

#[test]
fn criterion_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::CriterionResult);
    assert_json_snapshot!("criterion-result-schema", schema.to_value());
}

#[test]
fn enforcement_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Enforcement);
    assert_json_snapshot!("enforcement-schema", schema.to_value());
}

#[test]
fn gate_section_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateSection);
    assert_json_snapshot!("gate-section-schema", schema.to_value());
}

#[test]
fn enforcement_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::EnforcementSummary);
    assert_json_snapshot!("enforcement-summary-schema", schema.to_value());
}

#[test]
fn criterion_kind_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::CriterionKind);
    assert_json_snapshot!("criterion-kind-schema", schema.to_value());
}

#[test]
fn evaluator_role_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::EvaluatorRole);
    assert_json_snapshot!("evaluator-role-schema", schema.to_value());
}

#[test]
fn confidence_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Confidence);
    assert_json_snapshot!("confidence-schema", schema.to_value());
}

#[test]
fn agent_evaluation_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::AgentEvaluation);
    assert_json_snapshot!("agent-evaluation-schema", schema.to_value());
}

#[test]
fn gate_eval_context_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateEvalContext);
    assert_json_snapshot!("gate-eval-context-schema", schema.to_value());
}

#[test]
fn diff_truncation_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::DiffTruncation);
    assert_json_snapshot!("diff-truncation-schema", schema.to_value());
}

#[test]
fn gate_run_record_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateRunRecord);
    assert_json_snapshot!("gate-run-record-schema", schema.to_value());
}

#[test]
fn worktree_info_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::WorktreeInfo);
    assert_json_snapshot!("worktree-info-schema", schema.to_value());
}

#[test]
fn worktree_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::WorktreeStatus);
    assert_json_snapshot!("worktree-status-schema", schema.to_value());
}

#[test]
fn worktree_metadata_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::WorktreeMetadata);
    assert_json_snapshot!("worktree-metadata-schema", schema.to_value());
}

#[test]
fn harness_profile_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::HarnessProfile);
    assert_json_snapshot!("harness-profile-schema", schema.to_value());
}

#[test]
fn prompt_layer_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PromptLayer);
    assert_json_snapshot!("prompt-layer-schema", schema.to_value());
}

#[test]
fn prompt_layer_kind_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PromptLayerKind);
    assert_json_snapshot!("prompt-layer-kind-schema", schema.to_value());
}

#[test]
fn settings_override_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::SettingsOverride);
    assert_json_snapshot!("settings-override-schema", schema.to_value());
}

#[test]
fn hook_contract_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::HookContract);
    assert_json_snapshot!("hook-contract-schema", schema.to_value());
}

#[test]
fn hook_event_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::HookEvent);
    assert_json_snapshot!("hook-event-schema", schema.to_value());
}

#[test]
fn run_manifest_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::RunManifest);
    assert_json_snapshot!("run-manifest-schema", schema.to_value());
}

#[test]
fn manifest_session_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ManifestSession);
    assert_json_snapshot!("manifest-session-schema", schema.to_value());
}
