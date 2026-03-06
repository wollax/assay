//! Integration tests for MCP handler lifecycle flows.
//!
//! Each test runs in its own process (integration test binary), avoiding
//! CWD race conditions that would occur with `#[tokio::test]` in the
//! unit test module.

use std::io::Write as _;
use std::path::Path;

use assay_mcp::{
    AssayServer, GateFinalizeParams, GateReportParams, GateRunParams, Parameters,
};
use assay_types::{Confidence, EvaluatorRole};
use rmcp::model::RawContent;

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

/// Get the canonical path for a tempdir (avoids macOS /var -> /private/var symlink issues).
fn canonical(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().canonicalize().unwrap()
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
kind = "AgentReport"
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
