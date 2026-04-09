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

#[cfg(not(feature = "orchestrate"))]
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

// ── Scope enforcement types ──────────────────────────────────────────

#[test]
fn scope_violation_type_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ScopeViolationType);
    assert_json_snapshot!("scope-violation-type-schema", schema.to_value());
}

#[test]
fn scope_violation_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ScopeViolation);
    assert_json_snapshot!("scope-violation-schema", schema.to_value());
}

// ── Milestone types ──────────────────────────────────────────────────

#[test]
fn milestone_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Milestone);
    assert_json_snapshot!("milestone-schema", schema.to_value());
}

#[test]
fn chunk_ref_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ChunkRef);
    assert_json_snapshot!("chunk-ref-schema", schema.to_value());
}

#[test]
fn milestone_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MilestoneStatus);
    assert_json_snapshot!("milestone-status-schema", schema.to_value());
}

#[test]
fn gates_spec_schema_updated_snapshot() {
    let schema = schemars::schema_for!(assay_types::GatesSpec);
    assert_json_snapshot!("gates-spec-schema", schema.to_value());
}

// ── Merge execution types ────────────────────────────────────────────

#[test]
fn merge_execute_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeExecuteResult);
    assert_json_snapshot!("merge-execute-result-schema", schema.to_value());
}

#[test]
fn conflict_scan_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictScan);
    assert_json_snapshot!("conflict-scan-schema", schema.to_value());
}

#[test]
fn conflict_marker_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictMarker);
    assert_json_snapshot!("conflict-marker-schema", schema.to_value());
}

// ── Merge propose types ──────────────────────────────────────────────

#[test]
fn merge_proposal_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeProposal);
    assert_json_snapshot!("merge-proposal-schema", schema.to_value());
}

// ── Orchestrator types (behind orchestrate feature) ──────────────────

#[cfg(feature = "orchestrate")]
#[test]
fn session_run_state_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::SessionRunState);
    assert_json_snapshot!("session-run-state-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn failure_policy_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::FailurePolicy);
    assert_json_snapshot!("failure-policy-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn orchestrator_phase_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::OrchestratorPhase);
    assert_json_snapshot!("orchestrator-phase-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn session_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::SessionStatus);
    assert_json_snapshot!("session-status-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn orchestrator_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::OrchestratorStatus);
    assert_json_snapshot!("orchestrator-status-schema", schema.to_value());
}

// Separate snapshot for the orchestrate feature variant — RunManifest's schema
// changes shape (adds state_backend) when compiled with --features orchestrate.
// See DECISIONS.md D159 for rationale.
#[cfg(feature = "orchestrate")]
#[test]
fn run_manifest_orchestrate_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::RunManifest);
    assert_json_snapshot!("run-manifest-orchestrate-schema", schema.to_value());
}

// ── Merge ordering & runner types ────────────────────────────────────

#[cfg(feature = "orchestrate")]
#[test]
fn merge_strategy_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeStrategy);
    assert_json_snapshot!("merge-strategy-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn merge_plan_entry_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergePlanEntry);
    assert_json_snapshot!("merge-plan-entry-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn merge_plan_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergePlan);
    assert_json_snapshot!("merge-plan-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn merge_session_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeSessionStatus);
    assert_json_snapshot!("merge-session-status-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn merge_session_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeSessionResult);
    assert_json_snapshot!("merge-session-result-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn merge_report_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MergeReport);
    assert_json_snapshot!("merge-report-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn conflict_action_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictAction);
    assert_json_snapshot!("conflict-action-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn conflict_resolution_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictResolutionConfig);
    assert_json_snapshot!("conflict-resolution-config-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn conflict_file_content_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictFileContent);
    assert_json_snapshot!("conflict-file-content-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn conflict_resolution_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ConflictResolution);
    assert_json_snapshot!("conflict-resolution-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn orchestrator_mode_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::OrchestratorMode);
    assert_json_snapshot!("orchestrator-mode-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn mesh_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MeshConfig);
    assert_json_snapshot!("mesh-config-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn gossip_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GossipConfig);
    assert_json_snapshot!("gossip-config-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn mesh_member_state_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MeshMemberState);
    assert_json_snapshot!("mesh-member-state-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn mesh_member_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MeshMemberStatus);
    assert_json_snapshot!("mesh-member-status-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn mesh_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::MeshStatus);
    assert_json_snapshot!("mesh-status-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn knowledge_entry_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::KnowledgeEntry);
    assert_json_snapshot!("knowledge-entry-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn knowledge_manifest_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::KnowledgeManifest);
    assert_json_snapshot!("knowledge-manifest-schema", schema.to_value());
}

#[cfg(feature = "orchestrate")]
#[test]
fn gossip_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GossipStatus);
    assert_json_snapshot!("gossip-status-schema", schema.to_value());
}

#[test]
fn provider_kind_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ProviderKind);
    assert_json_snapshot!("provider-kind-schema", schema.to_value());
}

#[test]
fn provider_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ProviderConfig);
    assert_json_snapshot!("provider-config-schema", schema.to_value());
}

#[test]
fn state_backend_config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::StateBackendConfig);
    assert_json_snapshot!("state-backend-config-schema", schema.to_value());
}

// ── Signal types ────────────────────────────────────────────────────

#[test]
fn gate_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateSummary);
    assert_json_snapshot!("signal-gate-summary-schema", schema.to_value());
}

#[test]
fn peer_update_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PeerUpdate);
    assert_json_snapshot!("peer-update-schema", schema.to_value());
}

#[test]
fn signal_request_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::SignalRequest);
    assert_json_snapshot!("signal-request-schema", schema.to_value());
}

#[test]
fn run_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::RunSummary);
    assert_json_snapshot!("run-summary-schema", schema.to_value());
}

#[test]
fn poll_signals_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PollSignalsResult);
    assert_json_snapshot!("poll-signals-result-schema", schema.to_value());
}

#[test]
fn peer_info_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PeerInfo);
    assert_json_snapshot!("peer-info-schema", schema.to_value());
}

#[test]
fn assay_server_state_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::AssayServerState);
    assert_json_snapshot!("assay-server-state-schema", schema.to_value());
}

#[test]
fn coverage_report_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::CoverageReport);
    assert_json_snapshot!("coverage-report-schema", schema.to_value());
}

#[test]
fn review_check_kind_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ReviewCheckKind);
    assert_json_snapshot!("review-check-kind-schema", schema.to_value());
}

#[test]
fn review_check_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ReviewCheck);
    assert_json_snapshot!("review-check-schema", schema.to_value());
}

#[test]
fn review_report_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::ReviewReport);
    assert_json_snapshot!("review-report-schema", schema.to_value());
}

#[test]
fn failed_criterion_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::FailedCriterionSummary);
    assert_json_snapshot!("failed-criterion-summary-schema", schema.to_value());
}

#[test]
fn gate_diagnostic_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateDiagnostic);
    assert_json_snapshot!("gate-diagnostic-schema", schema.to_value());
}

#[test]
fn checkpoint_phase_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::review::CheckpointPhase);
    assert_json_snapshot!("checkpoint-phase-schema", schema.to_value());
}

#[test]
fn when_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::criterion::When);
    assert_json_snapshot!("when-schema", schema.to_value());
}

#[test]
fn agent_event_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::AgentEvent);
    assert_json_snapshot!("agent-event-schema", schema.to_value());
}

#[test]
fn tool_call_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::work_session::ToolCallSummary);
    assert_json_snapshot!("tool-call-summary-schema", schema.to_value());
}

#[test]
fn work_session_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::work_session::WorkSession);
    assert_json_snapshot!("work-session-schema", schema.to_value());
}
