//! Integration tests for `assay_core::merge::merge_propose`.
//!
//! Uses the mock `gh` / `git` binary pattern (shell scripts on PATH) to test
//! the merge_propose flow without real GitHub API calls.

use chrono::Utc;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use assay_types::{
    CriterionResult, Enforcement, EnforcementSummary, GateKind, GateResult, GateRunRecord,
    GateRunSummary, MergeProposeConfig,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Write a fake executable shell script at `dir/name`.
fn write_fake_script(dir: &Path, name: &str, script: &str) {
    let script_path = dir.join(name);
    fs::write(&script_path, script).expect("write fake script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Write a fake `gh` script that prints `stdout` and exits with `exit_code`.
fn write_fake_gh(dir: &Path, exit_code: i32, stdout: &str) {
    let script = format!(
        "#!/bin/sh\necho '{}'\nexit {}\n",
        stdout.replace('\'', "'\\''"),
        exit_code
    );
    write_fake_script(dir, "gh", &script);
}

/// Write a fake `git` script that always succeeds.
fn write_fake_git(dir: &Path) {
    write_fake_script(dir, "git", "#!/bin/sh\nexit 0\n");
}

/// Prepend `dir` to `PATH`, run `f`, then restore original `PATH`.
///
/// # Safety
/// Modifies the process environment. Callers must use `#[serial]`.
fn with_mock_path<R, F: FnOnce() -> R>(dir: &Path, f: F) -> R {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), original_path);
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = f();
    unsafe { std::env::set_var("PATH", original_path) };
    result
}

/// Create a minimal `GateRunRecord` with one passing criterion.
fn make_record(spec_name: &str) -> GateRunRecord {
    GateRunRecord {
        run_id: "20260328T120000Z-abc123".to_string(),
        assay_version: "0.5.0".to_string(),
        timestamp: Utc::now(),
        working_dir: None,
        summary: GateRunSummary {
            spec_name: spec_name.to_string(),
            results: vec![CriterionResult {
                criterion_name: "build".to_string(),
                result: Some(GateResult {
                    passed: true,
                    kind: GateKind::Command {
                        cmd: "cargo build".to_string(),
                    },
                    stdout: "ok".to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
                source: None,
            }],
            passed: 1,
            failed: 0,
            skipped: 0,
            total_duration_ms: 100,
            enforcement: EnforcementSummary::default(),
        },
        diff_truncation: None,
    }
}

/// Set up a fake assay dir with a gate run record for the given spec.
fn setup_assay_dir(tmp: &TempDir, spec_name: &str) -> PathBuf {
    let assay_dir = tmp.path().join(".assay");
    let results_dir = assay_dir.join("results").join(spec_name);
    fs::create_dir_all(&results_dir).expect("create results dir");

    let record = make_record(spec_name);
    let json = serde_json::to_string_pretty(&record).expect("serialize record");
    fs::write(results_dir.join(format!("{}.json", record.run_id)), json)
        .expect("write gate run record");

    assay_dir
}

/// Create a default `MergeProposeConfig` pointing at the tmp dir.
fn make_config(assay_dir: &Path, dry_run: bool) -> MergeProposeConfig {
    MergeProposeConfig {
        spec_name: "test-spec".to_string(),
        run_id: None,
        branch: "feat/test-branch".to_string(),
        base_branch: "main".to_string(),
        title: "Test PR".to_string(),
        working_dir: assay_dir.parent().unwrap().to_path_buf(),
        assay_dir: assay_dir.to_path_buf(),
        dry_run,
    }
}

// ── Test: dry_run returns correct shape ──────────────────────────────────────

#[test]
#[serial]
fn test_merge_propose_dry_run_returns_correct_shape() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // gh must be on PATH for preflight, but should NOT be called for PR creation
    write_fake_gh(&bin_dir, 0, "gh version 2.50.0");

    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, true);

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    let proposal = result.expect("dry_run should succeed");

    assert!(proposal.dry_run, "dry_run should be true");
    assert!(
        proposal.pr_url.is_none(),
        "pr_url should be None in dry_run"
    );
    assert!(
        proposal.pr_number.is_none(),
        "pr_number should be None in dry_run"
    );
    assert!(
        !proposal.gate_summary.is_empty(),
        "gate_summary should be non-empty"
    );
}

// ── Test: live path invokes git push then gh pr create ───────────────────────

#[test]
#[serial]
fn test_merge_propose_live_path_creates_pr() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock git push (succeeds)
    write_fake_git(&bin_dir);

    // Mock gh: preflight succeeds, pr create returns JSON
    let gh_script = r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "gh version 2.50.0"
    exit 0
fi
# gh pr create path — output JSON
echo '{"number": 42, "url": "https://github.com/test/repo/pull/42"}'
exit 0
"#;
    write_fake_script(&bin_dir, "gh", gh_script);

    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, false);

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    let proposal = result.expect("live path should succeed");

    assert!(!proposal.dry_run, "dry_run should be false");
    assert_eq!(proposal.pr_number, Some(42));
    assert_eq!(
        proposal.pr_url.as_deref(),
        Some("https://github.com/test/repo/pull/42")
    );
    assert!(
        !proposal.gate_summary.is_empty(),
        "gate_summary should be non-empty"
    );
}

// ── Test: missing gh returns clear error ─────────────────────────────────────

#[test]
#[serial]
fn test_merge_propose_missing_gh_returns_clear_error() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // No gh script — empty bin dir on PATH
    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, false);

    // Use a PATH that has only the empty bin dir (no system gh)
    let result = {
        let original_path = std::env::var("PATH").unwrap_or_default();
        unsafe { std::env::set_var("PATH", bin_dir.display().to_string()) };
        let r = assay_core::merge::merge_propose(&config);
        unsafe { std::env::set_var("PATH", original_path) };
        r
    };

    let err = result.expect_err("should fail when gh is missing");
    let msg = err.to_string();
    assert!(
        msg.contains("gh CLI not found"),
        "error should mention gh CLI not found, got: {msg}"
    );
    assert!(
        msg.contains("cli.github.com"),
        "error should link to cli.github.com, got: {msg}"
    );
}

// ── Test: env vars set on gh subprocess ──────────────────────────────────────

#[test]
#[serial]
fn test_merge_propose_env_vars_set_on_gh_subprocess() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock git push (succeeds)
    write_fake_git(&bin_dir);

    // Mock gh: on pr create, dump env vars to a file, then output JSON
    let env_capture_file = tmp.path().join("env_capture.txt");
    let gh_script = format!(
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "gh version 2.50.0"
    exit 0
fi
# Capture env vars to file
echo "ASSAY_BRANCH=$ASSAY_BRANCH" >> '{capture}'
echo "ASSAY_SPEC=$ASSAY_SPEC" >> '{capture}'
echo "ASSAY_GATE_REPORT_PATH=$ASSAY_GATE_REPORT_PATH" >> '{capture}'
echo '{{"number": 99, "url": "https://github.com/test/repo/pull/99"}}'
exit 0
"#,
        capture = env_capture_file.display()
    );
    write_fake_script(&bin_dir, "gh", &gh_script);

    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, false);

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    result.expect("should succeed");

    // Verify env vars were set
    let captured = fs::read_to_string(&env_capture_file).expect("read env capture file");
    assert!(
        captured.contains("ASSAY_BRANCH=feat/test-branch"),
        "ASSAY_BRANCH should be set, got:\n{captured}"
    );
    assert!(
        captured.contains("ASSAY_SPEC=test-spec"),
        "ASSAY_SPEC should be set, got:\n{captured}"
    );
    assert!(
        captured.contains("ASSAY_GATE_REPORT_PATH="),
        "ASSAY_GATE_REPORT_PATH should be set, got:\n{captured}"
    );
    // Verify the report path points to the correct location
    assert!(
        captured.contains(".assay/results/test-spec/"),
        "ASSAY_GATE_REPORT_PATH should point to results dir, got:\n{captured}"
    );
}

// ── Test: empty history returns error ────────────────────────────────────────

#[test]
#[serial]
fn test_merge_propose_empty_history_returns_error() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    write_fake_gh(&bin_dir, 0, "gh version 2.50.0");

    // Create assay dir with no gate run results
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).expect("create .assay dir");

    let config = MergeProposeConfig {
        spec_name: "nonexistent-spec".to_string(),
        run_id: None,
        branch: "feat/test".to_string(),
        base_branch: "main".to_string(),
        title: "Test".to_string(),
        working_dir: tmp.path().to_path_buf(),
        assay_dir: assay_dir.clone(),
        dry_run: false,
    };

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    let err = result.expect_err("should fail with empty history");
    let msg = err.to_string();
    assert!(
        msg.contains("no gate runs"),
        "error should mention no gate runs, got: {msg}"
    );
}

// ── Test: gh pr create failure returns error with stderr ─────────────────────

#[test]
#[serial]
fn test_merge_propose_gh_failure_returns_error() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock git push (succeeds)
    write_fake_git(&bin_dir);

    // Mock gh: preflight succeeds, pr create fails
    let gh_script = r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "gh version 2.50.0"
    exit 0
fi
echo "pull request create failed: no upstream" >&2
exit 1
"#;
    write_fake_script(&bin_dir, "gh", gh_script);

    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, false);

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    let err = result.expect_err("should fail when gh pr create fails");
    let msg = err.to_string();
    assert!(
        msg.contains("gh pr create"),
        "error should mention gh pr create, got: {msg}"
    );
}

// ── Test: stdin pipe sends gate evidence body ────────────────────────────────

#[test]
#[serial]
fn test_merge_propose_stdin_pipe_sends_evidence() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock git push (succeeds)
    write_fake_git(&bin_dir);

    // Mock gh: capture stdin to a file
    let stdin_capture = tmp.path().join("stdin_capture.txt");
    let gh_script = format!(
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "gh version 2.50.0"
    exit 0
fi
# Capture stdin
cat > '{capture}'
echo '{{"number": 1, "url": "https://github.com/test/repo/pull/1"}}'
exit 0
"#,
        capture = stdin_capture.display()
    );
    write_fake_script(&bin_dir, "gh", &gh_script);

    let assay_dir = setup_assay_dir(&tmp, "test-spec");
    let config = make_config(&assay_dir, false);

    let result = with_mock_path(&bin_dir, || assay_core::merge::merge_propose(&config));
    result.expect("should succeed");

    // Verify stdin was piped with gate evidence
    let stdin_content = fs::read_to_string(&stdin_capture).expect("read stdin capture");
    assert!(
        !stdin_content.is_empty(),
        "stdin should contain gate evidence body"
    );
    // The evidence body should contain the spec name
    assert!(
        stdin_content.contains("test-spec"),
        "stdin body should reference the spec, got:\n{stdin_content}"
    );
}
