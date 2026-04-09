//! Integration tests for MCP handler lifecycle flows.
//!
//! Each test runs in its own process (integration test binary), avoiding
//! CWD race conditions that would occur with `#[tokio::test]` in the
//! unit test module.

use std::io::Write as _;
use std::path::Path;

use assay_mcp::{
    AssayServer, GateFinalizeParams, GateHistoryParams, GateReportParams, GateRunParams,
    MergeCheckParams, Parameters, SpecValidateParams,
};
use assay_types::{Confidence, EvaluatorRole};
use rmcp::model::RawContent;
use serial_test::serial;

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract text content from a CallToolResult.
fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Create a tempdir with a valid `.assay/config.toml`.
/// Returns the tempdir handle (caller must keep alive).
/// Use `project_path()` to get the canonical path for `set_current_dir`.
fn create_project(config_toml: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let assay_dir = dir.path().join(".assay");
    std::fs::create_dir_all(&assay_dir).unwrap();
    let mut f = std::fs::File::create(assay_dir.join("config.toml")).unwrap();
    f.write_all(config_toml.as_bytes()).unwrap();
    dir
}

/// Create a spec file inside a project's specs directory.
fn create_spec(project_dir: &Path, filename: &str, content: &str) {
    let specs_path = project_dir.join(".assay").join("specs");
    std::fs::create_dir_all(&specs_path).unwrap();
    let mut f = std::fs::File::create(specs_path.join(filename)).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

// ── Lifecycle tests ──────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn gate_lifecycle_run_report_finalize() {
    let dir = create_project(r#"project_name = "lifecycle-test""#);
    create_spec(
        dir.path(),
        "mixed-spec.toml",
        r#"
name = "mixed-spec"
description = "Mixed cmd + agent spec"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"

[[criteria]]
name = "code-review"
description = "Agent reviews code quality"
kind = { type = "agent_report" }
prompt = "Review the code for quality issues"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    // Step 1: gate_run — should create a session for agent criteria
    let run_result = server
        .gate_run(Parameters(GateRunParams {
            name: "mixed-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    assert!(
        !run_result.is_error.unwrap_or(false),
        "gate_run should succeed, got: {}",
        extract_text(&run_result)
    );

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    assert_eq!(run_json["spec_name"], "mixed-spec");
    assert_eq!(run_json["passed"], 1, "echo-check should pass");

    let session_id = run_json["session_id"]
        .as_str()
        .expect("should have session_id for mixed spec");
    let pending = run_json["pending_criteria"]
        .as_array()
        .expect("should have pending_criteria");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], "code-review");

    // Step 2: gate_report — submit agent evaluation
    let report_result = server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "code-review".to_string(),
            passed: true,
            evidence: "All functions have doc comments, error handling is consistent".to_string(),
            reasoning: "Code quality meets project standards".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    assert!(
        !report_result.is_error.unwrap_or(false),
        "gate_report should succeed, got: {}",
        extract_text(&report_result)
    );

    let report_json: serde_json::Value =
        serde_json::from_str(&extract_text(&report_result)).unwrap();
    assert_eq!(report_json["accepted"], true);
    assert_eq!(report_json["evaluations_count"], 1);
    assert!(
        report_json["pending_criteria"]
            .as_array()
            .unwrap()
            .is_empty(),
        "no more pending criteria after reporting"
    );

    // Step 3: gate_finalize — persist results
    let finalize_result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: session_id.to_string(),
        }))
        .await
        .unwrap();

    assert!(
        !finalize_result.is_error.unwrap_or(false),
        "gate_finalize should succeed, got: {}",
        extract_text(&finalize_result)
    );

    let finalize_json: serde_json::Value =
        serde_json::from_str(&extract_text(&finalize_result)).unwrap();
    assert_eq!(finalize_json["persisted"], true);
    assert_eq!(finalize_json["spec_name"], "mixed-spec");

    insta::assert_json_snapshot!("gate_lifecycle_finalize", {
        let mut snap = finalize_json.clone();
        snap["run_id"] = serde_json::json!("[run_id]");
        snap
    });

    // Verify history file exists
    let results_dir = dir.path().join(".assay").join("results").join("mixed-spec");
    assert!(
        results_dir.exists(),
        "results directory should exist at {:?}",
        results_dir
    );
    let result_files: Vec<_> = std::fs::read_dir(&results_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert_eq!(
        result_files.len(),
        1,
        "should have exactly one history file"
    );
}

#[tokio::test]
#[serial]
async fn gate_run_with_timeout() {
    let dir = create_project(r#"project_name = "timeout-test""#);
    create_spec(
        dir.path(),
        "quick-spec.toml",
        r#"
name = "quick-spec"
description = "Quick spec"

[[criteria]]
name = "fast-check"
description = "Should complete quickly"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .gate_run(Parameters(GateRunParams {
            name: "quick-spec".to_string(),
            include_evidence: false,
            timeout: Some(10),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "gate_run with timeout should succeed, got: {}",
        extract_text(&result)
    );
    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["passed"], 1);
}

#[tokio::test]
#[serial]
async fn gate_run_command_only_persists_history() {
    let dir = create_project(r#"project_name = "cmd-history-test""#);
    create_spec(
        dir.path(),
        "cmd-only-spec.toml",
        r#"
name = "cmd-only-spec"
description = "Command-only spec for history persistence test"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .gate_run(Parameters(GateRunParams {
            name: "cmd-only-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "gate_run should succeed, got: {}",
        extract_text(&result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["passed"], 1);
    assert!(
        json["session_id"].is_null(),
        "command-only spec should not have a session_id"
    );

    // Verify history file exists on disk
    let results_dir = dir
        .path()
        .join(".assay")
        .join("results")
        .join("cmd-only-spec");
    assert!(
        results_dir.exists(),
        "results directory should exist at {:?}",
        results_dir
    );

    let result_files: Vec<_> = std::fs::read_dir(&results_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert_eq!(
        result_files.len(),
        1,
        "should have exactly one history file"
    );

    // Read and verify the history record contents
    let record_path = result_files[0].path();
    let record_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&record_path).unwrap()).unwrap();

    assert_eq!(record_json["summary"]["spec_name"], "cmd-only-spec");
    assert_eq!(record_json["summary"]["passed"], 1);
    assert!(
        record_json["run_id"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "run_id should be a non-empty string"
    );
    assert!(
        record_json["working_dir"].as_str().is_some(),
        "working_dir should be present"
    );
}

#[tokio::test]
#[serial]
async fn spec_list_with_parse_errors() {
    let dir = create_project(r#"project_name = "error-test""#);

    // Valid spec
    create_spec(
        dir.path(),
        "valid-spec.toml",
        r#"
name = "valid-spec"
description = "A valid spec"

[[criteria]]
name = "check"
description = "Check"
cmd = "echo ok"
"#,
    );

    // Malformed TOML file
    create_spec(
        dir.path(),
        "broken-spec.toml",
        "this is not valid toml {{{{",
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server.spec_list().await.unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "spec_list should succeed even with parse errors, got: {}",
        extract_text(&result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();

    // Should have the valid spec
    let specs = json["specs"].as_array().unwrap();
    assert!(
        specs.iter().any(|s| s["name"] == "valid-spec"),
        "should contain valid-spec in specs: {json}"
    );

    // Should have an error entry for the broken spec
    let errors = json["errors"].as_array();
    assert!(
        errors.is_some() && !errors.unwrap().is_empty(),
        "should have errors for broken spec: {json}"
    );
}

// ── Error path tests ────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn gate_finalize_invalid_session_returns_error() {
    let dir = create_project(r#"project_name = "finalize-test""#);
    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();
    let result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: "nonexistent-session-99".to_string(),
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "gate_finalize for invalid session should return error"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("nonexistent-session-99"),
        "error should mention the session_id, got: {text}"
    );
}

#[tokio::test]
async fn gate_report_not_found_returns_recovery_hint() {
    let server = AssayServer::new();
    let result = server
        .gate_report(Parameters(GateReportParams {
            session_id: "fabricated-session-abc".to_string(),
            criterion_name: "some-criterion".to_string(),
            passed: true,
            evidence: "test".to_string(),
            reasoning: "test".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "gate_report for missing session should return error"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("not found"),
        "error should mention 'not found', got: {text}"
    );
    assert!(
        text.contains("gate_run"),
        "error should suggest gate_run as recovery, got: {text}"
    );
    // Must NOT list active sessions.
    assert!(
        !text.contains("active sessions"),
        "error should NOT list active sessions, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn gate_finalize_not_found_returns_recovery_hint() {
    let dir = create_project(r#"project_name = "finalize-test""#);
    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();
    let result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: "fabricated-session-xyz".to_string(),
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "gate_finalize for missing session should return error"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("not found"),
        "error should mention 'not found', got: {text}"
    );
    assert!(
        text.contains("gate_run"),
        "error should suggest gate_run, got: {text}"
    );
    assert!(
        text.contains("gate_history"),
        "error should suggest gate_history, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn gate_report_and_finalize_not_found_errors_are_consistent() {
    let dir = create_project(r#"project_name = "finalize-test""#);
    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();
    let session_id = "same-fabricated-id-42";

    let report_result = server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "irrelevant".to_string(),
            passed: true,
            evidence: "test".to_string(),
            reasoning: "test".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    let finalize_result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: session_id.to_string(),
        }))
        .await
        .unwrap();

    let report_text = extract_text(&report_result);
    let finalize_text = extract_text(&finalize_result);

    // Both should be errors.
    assert!(report_result.is_error.unwrap_or(false));
    assert!(finalize_result.is_error.unwrap_or(false));

    // Both should mention the session ID.
    assert!(
        report_text.contains(session_id),
        "gate_report error should contain session ID, got: {report_text}"
    );
    assert!(
        finalize_text.contains(session_id),
        "gate_finalize error should contain session ID, got: {finalize_text}"
    );

    // Both should follow the same pattern (same structure, same recovery hints).
    assert!(
        report_text.contains("gate_run") && finalize_text.contains("gate_run"),
        "both errors should suggest gate_run"
    );
    assert!(
        report_text.contains("gate_history") && finalize_text.contains("gate_history"),
        "both errors should suggest gate_history"
    );
}

#[tokio::test]
#[serial]
async fn gate_report_nonexistent_criterion_returns_error() {
    let dir = create_project(r#"project_name = "report-error-test""#);
    create_spec(
        dir.path(),
        "agent-spec.toml",
        r#"
name = "agent-spec"
description = "Agent spec for error testing"

[[criteria]]
name = "real-criterion"
description = "Agent reviews code"
kind = { type = "agent_report" }
prompt = "Review the code"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    // gate_run to create a session
    let run_result = server
        .gate_run(Parameters(GateRunParams {
            name: "agent-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    let session_id = run_json["session_id"]
        .as_str()
        .expect("should have session_id");

    // Report to a criterion that doesn't exist in the session
    let report_result = server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "nonexistent-criterion".to_string(),
            passed: true,
            evidence: "test".to_string(),
            reasoning: "test".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    assert!(
        report_result.is_error.unwrap_or(false),
        "gate_report for nonexistent criterion should return error"
    );
    let text = extract_text(&report_result);
    assert!(
        text.contains("nonexistent-criterion"),
        "error should mention the criterion name, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn gate_report_duplicate_criterion_returns_error() {
    let dir = create_project(r#"project_name = "duplicate-report-test""#);
    create_spec(
        dir.path(),
        "agent-spec.toml",
        r#"
name = "agent-spec"
description = "Agent spec for duplicate test"

[[criteria]]
name = "review"
description = "Agent review"
kind = { type = "agent_report" }
prompt = "Review code"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    let run_result = server
        .gate_run(Parameters(GateRunParams {
            name: "agent-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    let session_id = run_json["session_id"]
        .as_str()
        .expect("should have session_id");

    // First report — should succeed
    let first_report = server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "review".to_string(),
            passed: true,
            evidence: "looks good".to_string(),
            reasoning: "code is clean".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    assert!(
        !first_report.is_error.unwrap_or(false),
        "first report should succeed, got: {}",
        extract_text(&first_report)
    );

    // Second report to same criterion — server accepts updates (overwrites)
    let second_report = server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "review".to_string(),
            passed: false,
            evidence: "actually bad".to_string(),
            reasoning: "changed my mind".to_string(),
            confidence: Some(Confidence::Low),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    assert!(
        !second_report.is_error.unwrap_or(false),
        "duplicate report should be accepted (update), got: {}",
        extract_text(&second_report)
    );
    let second_json: serde_json::Value =
        serde_json::from_str(&extract_text(&second_report)).unwrap();
    // Server uses highest-priority evaluator wins; duplicate reports are appended
    assert!(
        second_json["evaluations_count"].as_u64().unwrap() >= 1,
        "should have at least one evaluation after duplicate report"
    );
}

#[tokio::test]
#[serial]
async fn gate_history_nonexistent_spec_returns_error() {
    let dir = create_project(r#"project_name = "history-error-test""#);
    std::fs::create_dir_all(dir.path().join(".assay").join("specs")).unwrap();

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .gate_history(Parameters(GateHistoryParams {
            name: "nonexistent-spec".to_string(),
            run_id: None,
            limit: None,
            outcome: None,
        }))
        .await
        .unwrap();

    // gate_history for a spec that has no history should still succeed (empty list)
    // but for a spec that doesn't exist at all, behavior depends on implementation
    let text = extract_text(&result);
    // Either succeeds with empty runs or returns an error — both are valid
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    if !result.is_error.unwrap_or(false) {
        assert_eq!(json["total_runs"], 0);
    }
}

// ── Warnings field tests ─────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn gate_run_command_only_success_omits_warnings() {
    let dir = create_project(r#"project_name = "warnings-cmd-test""#);
    create_spec(
        dir.path(),
        "cmd-spec.toml",
        r#"
name = "cmd-spec"
description = "Command-only spec for warnings test"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .gate_run(Parameters(GateRunParams {
            name: "cmd-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "gate_run should succeed, got: {}",
        extract_text(&result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["passed"], 1);
    // warnings should be omitted entirely when empty (skip_serializing_if)
    assert!(
        json.get("warnings").is_none(),
        "warnings should be absent when empty, got: {json}"
    );
}

#[tokio::test]
#[serial]
async fn gate_finalize_success_omits_warnings_and_has_persisted() {
    let dir = create_project(r#"project_name = "warnings-finalize-test""#);
    create_spec(
        dir.path(),
        "agent-spec.toml",
        r#"
name = "agent-spec"
description = "Agent spec for finalize warnings test"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"

[[criteria]]
name = "code-review"
description = "Agent reviews code quality"
kind = { type = "agent_report" }
prompt = "Review the code for quality issues"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    // gate_run
    let run_result = server
        .gate_run(Parameters(GateRunParams {
            name: "agent-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    let session_id = run_json["session_id"]
        .as_str()
        .expect("should have session_id");

    // gate_report
    server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "code-review".to_string(),
            passed: true,
            evidence: "Code looks good".to_string(),
            reasoning: "Meets standards".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    // gate_finalize
    let finalize_result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: session_id.to_string(),
        }))
        .await
        .unwrap();

    assert!(
        !finalize_result.is_error.unwrap_or(false),
        "gate_finalize should succeed, got: {}",
        extract_text(&finalize_result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&finalize_result)).unwrap();

    // persisted should be true on success
    assert_eq!(json["persisted"], true);

    // warnings should be omitted when empty
    assert!(
        json.get("warnings").is_none(),
        "warnings should be absent when empty, got: {json}"
    );

    // Verify full response structure from GateFinalizeResponse struct.
    // The spec has 1 cmd criterion (passes) and 1 agent criterion (reported as passed).
    assert!(
        json["run_id"].as_str().is_some_and(|s| !s.is_empty()),
        "run_id should be present"
    );
    assert_eq!(json["spec_name"], "agent-spec");
    assert_eq!(json["passed"], 2, "both cmd and agent criteria should pass");
    assert_eq!(json["failed"], 0);
    assert_eq!(json["skipped"], 0);
    assert_eq!(json["required_failed"], 0);
    assert_eq!(json["advisory_failed"], 0);
    assert_eq!(
        json["blocked"], false,
        "no required failures means not blocked"
    );
}

#[tokio::test]
#[serial]
async fn gate_finalize_save_failure_surfaces_warning() {
    // Skip when running as root (e.g. Docker CI containers) — root can write to
    // read-only directories, so the permission-based save-failure can't be simulated.
    #[cfg(unix)]
    if std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
    {
        eprintln!("Skipping: running as root, read-only permission test is not reliable");
        return;
    }
    let dir = create_project(r#"project_name = "save-fail-test""#);
    create_spec(
        dir.path(),
        "cmd-spec.toml",
        r#"
name = "cmd-spec"
description = "Command-only spec for save failure test"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"

[[criteria]]
name = "code-review"
description = "Agent reviews code"
kind = { type = "agent_report" }
prompt = "Review the code"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    // gate_run to create a session
    let run_result = server
        .gate_run(Parameters(GateRunParams {
            name: "cmd-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    let session_id = run_json["session_id"]
        .as_str()
        .expect("should have session_id");

    // Submit agent report
    server
        .gate_report(Parameters(GateReportParams {
            session_id: session_id.to_string(),
            criterion_name: "code-review".to_string(),
            passed: true,
            evidence: "Looks good".to_string(),
            reasoning: "Clean code".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
        }))
        .await
        .unwrap();

    // Make the results directory read-only to force a save failure.
    // Create the results dir first, then make it non-writable.
    let results_dir = dir.path().join(".assay").join("results");
    std::fs::create_dir_all(&results_dir).unwrap();

    // On Unix, remove write permission from the results directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o444);
        std::fs::set_permissions(&results_dir, perms).unwrap();
    }

    let finalize_result = server
        .gate_finalize(Parameters(GateFinalizeParams {
            session_id: session_id.to_string(),
        }))
        .await
        .unwrap();

    // Restore permissions before assertions (so tempdir cleanup works)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&results_dir, perms).unwrap();
    }

    assert!(
        !finalize_result.is_error.unwrap_or(false),
        "gate_finalize should succeed (with warnings), got: {}",
        extract_text(&finalize_result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&finalize_result)).unwrap();

    // On Unix, save should fail due to read-only dir, surfacing a warning
    #[cfg(unix)]
    {
        assert_eq!(
            json["persisted"], false,
            "persisted should be false when save fails"
        );
        let warnings = json["warnings"]
            .as_array()
            .expect("warnings should be present");
        assert!(
            !warnings.is_empty(),
            "should have at least one warning about save failure"
        );
        assert!(
            warnings[0]
                .as_str()
                .unwrap()
                .contains("history save failed"),
            "warning should mention save failure, got: {}",
            warnings[0]
        );
    }
}

// ── Outcome filter and limit cap tests ───────────────────────────────

/// Helper: run a gate and return its JSON response. Panics on handler error.
async fn run_gate(server: &AssayServer, spec_name: &str) -> serde_json::Value {
    let result = server
        .gate_run(Parameters(GateRunParams {
            name: spec_name.to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();
    assert!(
        !result.is_error.unwrap_or(false),
        "gate_run should succeed, got: {}",
        extract_text(&result)
    );
    serde_json::from_str(&extract_text(&result)).unwrap()
}

/// Helper: query gate_history in list mode and return parsed JSON.
async fn query_history(
    server: &AssayServer,
    spec_name: &str,
    limit: Option<usize>,
    outcome: Option<&str>,
) -> serde_json::Value {
    let result = server
        .gate_history(Parameters(GateHistoryParams {
            name: spec_name.to_string(),
            run_id: None,
            limit,
            outcome: outcome.map(String::from),
        }))
        .await
        .unwrap();
    assert!(
        !result.is_error.unwrap_or(false),
        "gate_history should succeed, got: {}",
        extract_text(&result)
    );
    serde_json::from_str(&extract_text(&result)).unwrap()
}

#[tokio::test]
#[serial]
async fn gate_history_outcome_failed_filters_correctly() {
    let dir = create_project(r#"project_name = "outcome-failed-test""#);

    // Spec with a required criterion that always fails
    create_spec(
        dir.path(),
        "fail-spec.toml",
        r#"
name = "fail-spec"
description = "Spec with a failing required criterion"

[[criteria]]
name = "will-fail"
description = "Always fails"
cmd = "false"
"#,
    );

    // Spec with a criterion that always passes
    create_spec(
        dir.path(),
        "pass-spec.toml",
        r#"
name = "pass-spec"
description = "Spec with a passing criterion"

[[criteria]]
name = "will-pass"
description = "Always passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    // Create 2 passing runs and 2 failing runs (using separate specs that share history dir conceptually)
    // Actually, outcome filter is per-spec, so we need a spec that can both pass and fail.
    // Let's use two separate specs and query them independently.
    run_gate(&server, "fail-spec").await;
    run_gate(&server, "fail-spec").await;
    run_gate(&server, "pass-spec").await;
    run_gate(&server, "pass-spec").await;

    // Query failed runs for fail-spec
    let failed_history = query_history(&server, "fail-spec", None, Some("failed")).await;
    let runs = failed_history["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2, "should have 2 failed runs");
    // total_runs reflects ALL on-disk records, not just filtered
    assert_eq!(
        failed_history["total_runs"], 2,
        "total_runs should reflect all on-disk records for fail-spec"
    );
    for run in runs {
        assert!(
            run["required_failed"].as_u64().unwrap() > 0,
            "all returned runs should have required_failed > 0"
        );
    }

    // Query passed runs for fail-spec — should be empty but total_runs still shows all records
    let passed_history = query_history(&server, "fail-spec", None, Some("passed")).await;
    let runs = passed_history["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 0, "fail-spec should have 0 passed runs");
    assert_eq!(
        passed_history["total_runs"], 2,
        "total_runs should reflect all on-disk records regardless of outcome filter"
    );

    // Query failed runs for pass-spec — should be empty
    let failed_history = query_history(&server, "pass-spec", None, Some("failed")).await;
    let runs = failed_history["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 0, "pass-spec should have 0 failed runs");
    assert_eq!(
        failed_history["total_runs"], 2,
        "total_runs should reflect all on-disk records for pass-spec"
    );
}

#[tokio::test]
#[serial]
async fn gate_history_outcome_passed_filters_correctly() {
    let dir = create_project(r#"project_name = "outcome-passed-test""#);
    create_spec(
        dir.path(),
        "pass-spec.toml",
        r#"
name = "pass-spec"
description = "Spec with a passing criterion"

[[criteria]]
name = "will-pass"
description = "Always passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    run_gate(&server, "pass-spec").await;
    run_gate(&server, "pass-spec").await;
    run_gate(&server, "pass-spec").await;

    let passed_history = query_history(&server, "pass-spec", None, Some("passed")).await;
    let runs = passed_history["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 3, "should have 3 passed runs");
    for run in runs {
        assert_eq!(
            run["required_failed"].as_u64().unwrap(),
            0,
            "all returned runs should have required_failed == 0"
        );
    }
}

#[tokio::test]
#[serial]
async fn gate_history_outcome_any_returns_all() {
    let dir = create_project(r#"project_name = "outcome-any-test""#);

    // Create two separate specs: one that always passes, one that always fails.
    // This ensures we have genuinely mixed outcomes when querying a single spec.
    create_spec(
        dir.path(),
        "pass-spec.toml",
        r#"
name = "pass-spec"
description = "Spec that always passes"

[[criteria]]
name = "will-pass"
description = "Always passes"
cmd = "echo ok"
"#,
    );

    create_spec(
        dir.path(),
        "fail-spec.toml",
        r#"
name = "fail-spec"
description = "Spec that always fails"

[[criteria]]
name = "will-fail"
description = "Always fails"
cmd = "false"
"#,
    );

    // Use a single spec with an advisory failing criterion so the run itself is "passed"
    // (required_failed == 0), mixed with a spec that has required failures.
    create_spec(
        dir.path(),
        "mixed-spec.toml",
        r#"
name = "mixed-spec"
description = "Spec with advisory fail — overall passes"

[[criteria]]
name = "will-pass"
description = "Always passes"
cmd = "echo ok"

[[criteria]]
name = "advisory-fail"
description = "Fails but advisory"
cmd = "false"
enforcement = "advisory"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    // Run pass-spec (passed) and fail-spec (failed) to verify the filter works
    // across specs with different outcomes. Then query each with outcome=any.
    run_gate(&server, "pass-spec").await;
    run_gate(&server, "fail-spec").await;

    // Run mixed-spec twice — all runs are "passed" because advisory failures don't block
    run_gate(&server, "mixed-spec").await;

    // outcome=any should return all runs for pass-spec
    let any_history = query_history(&server, "pass-spec", None, Some("any")).await;
    let runs = any_history["runs"].as_array().unwrap();
    assert_eq!(
        runs.len(),
        1,
        "outcome=any should return the 1 pass-spec run"
    );

    // outcome=any should return all runs for fail-spec
    let any_history = query_history(&server, "fail-spec", None, Some("any")).await;
    let runs = any_history["runs"].as_array().unwrap();
    assert_eq!(
        runs.len(),
        1,
        "outcome=any should return the 1 fail-spec run"
    );
    assert!(
        runs[0]["blocked"].as_bool().unwrap(),
        "fail-spec run should be blocked"
    );

    // No outcome (default) should also return all runs
    let default_history = query_history(&server, "pass-spec", None, None).await;
    let default_runs = default_history["runs"].as_array().unwrap();
    assert_eq!(
        default_runs.len(),
        1,
        "default outcome should return all runs"
    );

    // mixed-spec: advisory failure means the run is "passed" (not blocked)
    let mixed_history = query_history(&server, "mixed-spec", None, Some("passed")).await;
    let mixed_runs = mixed_history["runs"].as_array().unwrap();
    assert_eq!(
        mixed_runs.len(),
        1,
        "mixed-spec run should be 'passed' (advisory fail doesn't block)"
    );
    assert!(
        !mixed_runs[0]["blocked"].as_bool().unwrap(),
        "mixed-spec run should not be blocked"
    );
    assert_eq!(
        mixed_runs[0]["advisory_failed"], 1,
        "should have 1 advisory failure"
    );
}

#[tokio::test]
#[serial]
async fn gate_history_unrecognized_outcome_returns_error() {
    let dir = create_project(r#"project_name = "bad-outcome-test""#);
    create_spec(
        dir.path(),
        "some-spec.toml",
        r#"
name = "some-spec"
description = "Spec for outcome validation test"

[[criteria]]
name = "check"
description = "Always passes"
cmd = "true"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    let result = server
        .gate_history(Parameters(GateHistoryParams {
            name: "some-spec".to_string(),
            run_id: None,
            limit: None,
            outcome: Some("invalid_value".to_string()),
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "unrecognized outcome value should return error, got: {}",
        extract_text(&result)
    );
    let text = extract_text(&result);
    assert!(
        text.contains("unrecognized outcome filter"),
        "error should mention unrecognized outcome, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn gate_history_limit_capped_at_50() {
    let dir = create_project(r#"project_name = "limit-cap-test""#);
    create_spec(
        dir.path(),
        "cap-spec.toml",
        r#"
name = "cap-spec"
description = "Spec for limit cap test"

[[criteria]]
name = "check"
description = "Always passes"
cmd = "true"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    // Create 51 runs to exceed the 50-run cap
    for _ in 0..51 {
        run_gate(&server, "cap-spec").await;
    }

    // Request limit=100, which should be capped at 50 internally.
    let history = query_history(&server, "cap-spec", Some(100), None).await;
    let runs = history["runs"].as_array().unwrap();
    assert_eq!(
        runs.len(),
        50,
        "should return exactly 50 runs (limit=100 capped to 50)"
    );
    assert_eq!(
        history["total_runs"], 51,
        "total_runs should reflect all 51 on-disk records"
    );
}

#[tokio::test]
#[serial]
async fn gate_history_default_limit_is_10() {
    let dir = create_project(r#"project_name = "default-limit-test""#);
    create_spec(
        dir.path(),
        "limit-spec.toml",
        r#"
name = "limit-spec"
description = "Spec for default limit test"

[[criteria]]
name = "check"
description = "Always passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    // Create 15 runs
    for _ in 0..15 {
        run_gate(&server, "limit-spec").await;
    }

    // Query with no limit — should return exactly 10 (default)
    let history = query_history(&server, "limit-spec", None, None).await;
    let runs = history["runs"].as_array().unwrap();
    assert_eq!(
        runs.len(),
        10,
        "default limit should return 10 runs, got {}",
        runs.len()
    );
    assert_eq!(
        history["total_runs"], 15,
        "total_runs should reflect all 15 on-disk records"
    );
}

// ── spec_validate tests ─────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn spec_validate_valid_spec() {
    let dir = create_project(r#"project_name = "validate-test""#);
    create_spec(
        dir.path(),
        "good-spec.toml",
        r#"
name = "good spec"
description = "A valid spec"

[[criteria]]
name = "builds"
description = "Project builds"
cmd = "echo ok"
"#,
    );
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .spec_validate(Parameters(SpecValidateParams {
            name: "good-spec".to_string(),
            check_commands: false,
        }))
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["spec"], "good-spec");
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["summary"]["warnings"], 0);
}

#[tokio::test]
#[serial]
async fn spec_validate_toml_parse_error() {
    let dir = create_project(r#"project_name = "validate-test""#);
    create_spec(dir.path(), "bad-spec.toml", "this is not valid toml [[[");
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .spec_validate(Parameters(SpecValidateParams {
            name: "bad-spec".to_string(),
            check_commands: false,
        }))
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["valid"], false);
    assert_eq!(json["spec"], "bad-spec");
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(json["diagnostics"][0]["severity"], "error");
    assert_eq!(json["diagnostics"][0]["location"], "toml");
}

#[tokio::test]
#[serial]
async fn spec_validate_not_found_returns_validation_result() {
    let dir = create_project(r#"project_name = "validate-test""#);
    // Create a valid spec so the specs dir exists
    create_spec(
        dir.path(),
        "real-spec.toml",
        r#"
name = "real"
description = "exists"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
    );
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .spec_validate(Parameters(SpecValidateParams {
            name: "nonexistent-spec".to_string(),
            check_commands: false,
        }))
        .await
        .unwrap();

    // Should return a structured ValidationResult, not a domain_error
    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["valid"], false);
    assert_eq!(json["spec"], "nonexistent-spec");
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(json["diagnostics"][0]["severity"], "error");
    assert_eq!(json["diagnostics"][0]["location"], "name");
    assert!(
        json["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("not found"),
        "should mention spec not found"
    );
}

#[tokio::test]
#[serial]
async fn spec_validate_agent_prompt_warning() {
    let dir = create_project(r#"project_name = "validate-test""#);
    create_spec(
        dir.path(),
        "agent-spec.toml",
        r#"
name = "agent spec"
description = "Has agent criteria without prompt"

[[criteria]]
name = "review"
description = "Agent review"
kind = { type = "agent_report" }
"#,
    );
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .spec_validate(Parameters(SpecValidateParams {
            name: "agent-spec".to_string(),
            check_commands: false,
        }))
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    // Valid because warnings don't block
    assert_eq!(json["valid"], true);
    assert_eq!(json["summary"]["warnings"], 1);
    assert_eq!(json["diagnostics"][0]["severity"], "warning");
    assert!(
        json["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("no prompt")
    );
}

// ── merge_check tests ────────────────────────────────────────────────

/// Returns the project root (the git repo root for assay itself).
fn project_root() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest)
        .parent() // crates/
        .unwrap()
        .parent() // repo root
        .unwrap()
        .to_path_buf()
}

#[tokio::test]
#[serial]
async fn merge_check_invalid_ref_returns_domain_error() {
    let root = project_root();
    std::env::set_current_dir(&root).unwrap();

    let server = AssayServer::new();
    let result = server
        .merge_check(Parameters(MergeCheckParams {
            base: "nonexistent-ref-xyz".to_string(),
            head: "HEAD".to_string(),
            max_conflicts: None,
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "merge_check with invalid ref should return domain error, got: {}",
        extract_text(&result)
    );
    let text = extract_text(&result);
    assert!(
        text.contains("nonexistent-ref-xyz"),
        "error should mention the bad ref, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn merge_check_self_merge_is_clean() {
    let root = project_root();
    std::env::set_current_dir(&root).unwrap();

    let server = AssayServer::new();
    let result = server
        .merge_check(Parameters(MergeCheckParams {
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            max_conflicts: None,
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "merge_check HEAD..HEAD should succeed, got: {}",
        extract_text(&result)
    );

    let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(json["clean"], true, "self-merge should be clean");
    assert_eq!(
        json["conflicts"].as_array().map(|a| a.len()).unwrap_or(0),
        0,
        "self-merge should have no conflicts"
    );
    assert_eq!(json["ahead"], 0, "self-merge should have 0 ahead");
    assert_eq!(json["behind"], 0, "self-merge should have 0 behind");
    assert_eq!(
        json["fast_forward"], true,
        "self-merge should be fast-forward (HEAD is ancestor of itself)"
    );
    assert_eq!(
        json["truncated"], false,
        "self-merge should not be truncated"
    );
}

#[tokio::test]
#[serial]
async fn merge_check_both_refs_invalid_reports_both_errors() {
    let root = project_root();
    std::env::set_current_dir(&root).unwrap();

    let server = AssayServer::new();
    let result = server
        .merge_check(Parameters(MergeCheckParams {
            base: "nonexistent-base-xyz".to_string(),
            head: "nonexistent-head-abc".to_string(),
            max_conflicts: None,
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "merge_check with both refs invalid should return domain error, got: {}",
        extract_text(&result)
    );
    let text = extract_text(&result);
    assert!(
        text.contains("nonexistent-base-xyz"),
        "error should mention the bad base ref, got: {text}"
    );
    assert!(
        text.contains("nonexistent-head-abc"),
        "error should mention the bad head ref, got: {text}"
    );
}

// ── orchestrate_status integration tests ─────────────────────────────

#[tokio::test]
#[serial]
async fn orchestrate_status_reads_persisted_state_with_sessions() {
    use assay_mcp::OrchestrateStatusParams;

    let dir = create_project(r#"project_name = "status-integration""#);
    std::env::set_current_dir(dir.path()).unwrap();

    // Write a realistic state.json with multiple session statuses
    let run_id = "01JINTEGRATION01";
    let state_dir = dir.path().join(".assay").join("orchestrator").join(run_id);
    std::fs::create_dir_all(&state_dir).unwrap();

    let status = assay_types::orchestrate::OrchestratorStatus {
        run_id: run_id.to_string(),
        phase: assay_types::orchestrate::OrchestratorPhase::PartialFailure,
        failure_policy: assay_types::orchestrate::FailurePolicy::SkipDependents,
        sessions: vec![
            assay_types::orchestrate::SessionStatus {
                name: "auth".to_string(),
                spec: "spec-auth".to_string(),
                state: assay_types::orchestrate::SessionRunState::Completed,
                started_at: Some(chrono::Utc::now()),
                completed_at: Some(chrono::Utc::now()),
                duration_secs: Some(12.5),
                error: None,
                skip_reason: None,
            },
            assay_types::orchestrate::SessionStatus {
                name: "db".to_string(),
                spec: "spec-db".to_string(),
                state: assay_types::orchestrate::SessionRunState::Failed,
                started_at: Some(chrono::Utc::now()),
                completed_at: Some(chrono::Utc::now()),
                duration_secs: Some(3.2),
                error: Some("agent crashed".to_string()),
                skip_reason: None,
            },
            assay_types::orchestrate::SessionStatus {
                name: "api".to_string(),
                spec: "spec-api".to_string(),
                state: assay_types::orchestrate::SessionRunState::Skipped,
                started_at: None,
                completed_at: None,
                duration_secs: None,
                error: None,
                skip_reason: Some("upstream 'db' failed".to_string()),
            },
        ],
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        mesh_status: None,
        gossip_status: None,
    };
    let json = serde_json::to_string_pretty(&status).unwrap();
    std::fs::write(state_dir.join("state.json"), &json).unwrap();

    let server = AssayServer::new();
    let result = server
        .orchestrate_status(Parameters(OrchestrateStatusParams {
            run_id: run_id.to_string(),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "should succeed for valid state, got: {}",
        extract_text(&result)
    );

    let response_json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();

    // Response is now wrapped: { "status": {...}, "merge_report": null_or_object }
    let status_json = &response_json["status"];
    assert_eq!(status_json["run_id"], run_id);
    assert_eq!(status_json["phase"], "partial_failure");
    assert_eq!(status_json["failure_policy"], "skip_dependents");

    // Verify session details
    let sessions = status_json["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 3);

    let auth = sessions.iter().find(|s| s["name"] == "auth").unwrap();
    assert_eq!(auth["state"], "completed");
    assert!(auth["duration_secs"].as_f64().is_some());

    let db = sessions.iter().find(|s| s["name"] == "db").unwrap();
    assert_eq!(db["state"], "failed");
    assert!(db["error"].as_str().unwrap().contains("crashed"));

    let api = sessions.iter().find(|s| s["name"] == "api").unwrap();
    assert_eq!(api["state"], "skipped");
    assert!(api["skip_reason"].as_str().unwrap().contains("db"));
}

#[tokio::test]
#[serial]
async fn orchestrate_status_missing_run_id_returns_domain_error() {
    use assay_mcp::OrchestrateStatusParams;

    let dir = create_project(r#"project_name = "status-missing""#);
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .orchestrate_status(Parameters(OrchestrateStatusParams {
            run_id: "01JDOESNOTEXIST".to_string(),
        }))
        .await
        .unwrap();

    assert!(
        result.is_error.unwrap_or(false),
        "should return error for missing run_id"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("No orchestrator state found"),
        "error should mention missing state, got: {text}"
    );
    assert!(
        text.contains("01JDOESNOTEXIST"),
        "error should include the run_id, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn orchestrate_status_returns_mesh_status() {
    use assay_mcp::OrchestrateStatusParams;
    use assay_types::orchestrate::{MeshMemberState, MeshMemberStatus, MeshStatus};

    let dir = create_project(r#"project_name = "status-mesh""#);
    std::env::set_current_dir(dir.path()).unwrap();

    let run_id = "01JMESHSTATUS01";
    let state_dir = dir.path().join(".assay").join("orchestrator").join(run_id);
    std::fs::create_dir_all(&state_dir).unwrap();

    let status = assay_types::orchestrate::OrchestratorStatus {
        run_id: run_id.to_string(),
        phase: assay_types::orchestrate::OrchestratorPhase::Completed,
        failure_policy: assay_types::orchestrate::FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        mesh_status: Some(MeshStatus {
            members: vec![MeshMemberStatus {
                name: "alpha".into(),
                state: MeshMemberState::Completed,
                last_heartbeat_at: None,
            }],
            messages_routed: 3,
        }),
        gossip_status: None,
    };
    let json = serde_json::to_string_pretty(&status).unwrap();
    std::fs::write(state_dir.join("state.json"), &json).unwrap();

    let server = AssayServer::new();
    let result = server
        .orchestrate_status(Parameters(OrchestrateStatusParams {
            run_id: run_id.to_string(),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "should succeed for valid state, got: {}",
        extract_text(&result)
    );

    let response_json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    let status_json = &response_json["status"];

    // mesh_status must be present with correct values
    assert!(
        !status_json["mesh_status"].is_null(),
        "mesh_status should be present in response"
    );
    assert_eq!(
        status_json["mesh_status"]["messages_routed"], 3,
        "messages_routed should be 3"
    );
    assert_eq!(
        status_json["mesh_status"]["members"][0]["name"], "alpha",
        "first member name should be alpha"
    );
    assert_eq!(
        status_json["mesh_status"]["members"][0]["state"], "completed",
        "first member state should be completed"
    );

    // gossip_status must be absent (skip_serializing_if = None)
    assert!(
        status_json["gossip_status"].is_null()
            || !status_json
                .as_object()
                .unwrap()
                .contains_key("gossip_status"),
        "gossip_status should be null or absent when None"
    );
}

#[tokio::test]
#[serial]
async fn orchestrate_status_returns_gossip_status() {
    use assay_mcp::OrchestrateStatusParams;
    use assay_types::orchestrate::GossipStatus;

    let dir = create_project(r#"project_name = "status-gossip""#);
    std::env::set_current_dir(dir.path()).unwrap();

    let run_id = "01JGOSSIPSTATUS1";
    let state_dir = dir.path().join(".assay").join("orchestrator").join(run_id);
    std::fs::create_dir_all(&state_dir).unwrap();

    let status = assay_types::orchestrate::OrchestratorStatus {
        run_id: run_id.to_string(),
        phase: assay_types::orchestrate::OrchestratorPhase::Completed,
        failure_policy: assay_types::orchestrate::FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        mesh_status: None,
        gossip_status: Some(GossipStatus {
            sessions_synthesized: 2,
            knowledge_manifest_path: std::path::PathBuf::from("/tmp/run/gossip/knowledge.json"),
            coordinator_rounds: 4,
        }),
    };
    let json = serde_json::to_string_pretty(&status).unwrap();
    std::fs::write(state_dir.join("state.json"), &json).unwrap();

    let server = AssayServer::new();
    let result = server
        .orchestrate_status(Parameters(OrchestrateStatusParams {
            run_id: run_id.to_string(),
        }))
        .await
        .unwrap();

    assert!(
        !result.is_error.unwrap_or(false),
        "should succeed for valid state, got: {}",
        extract_text(&result)
    );

    let response_json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    let status_json = &response_json["status"];

    // gossip_status must be present with correct values
    assert!(
        !status_json["gossip_status"].is_null(),
        "gossip_status should be present in response"
    );
    assert_eq!(
        status_json["gossip_status"]["sessions_synthesized"], 2,
        "sessions_synthesized should be 2"
    );
    assert_eq!(
        status_json["gossip_status"]["coordinator_rounds"], 4,
        "coordinator_rounds should be 4"
    );
    assert!(
        status_json["gossip_status"]["knowledge_manifest_path"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "knowledge_manifest_path should be a non-empty string"
    );

    // mesh_status must be absent (skip_serializing_if = None)
    assert!(
        status_json["mesh_status"].is_null()
            || !status_json.as_object().unwrap().contains_key("mesh_status"),
        "mesh_status should be null or absent when None"
    );
}
