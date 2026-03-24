//! Integration tests for `assay_core::pr` — gate-gated PR creation workflow.
//!
//! Covers: gate-check pass/fail, missing spec, idempotency guard, gh-not-found,
//! mock-gh success, and Verify→Complete auto-transition.

use chrono::Utc;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

use assay_core::milestone::{milestone_load, milestone_save};
use assay_core::pr::{ChunkGateFailure, pr_check_milestone_gates, pr_create_if_gates_pass};
use assay_types::{ChunkRef, Milestone, MilestoneStatus};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_assay_dir(tmp: &TempDir) -> std::path::PathBuf {
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).expect("create .assay dir");
    assay_dir
}

fn make_milestone_with_status(
    slug: &str,
    status: MilestoneStatus,
    chunks: Vec<ChunkRef>,
) -> Milestone {
    let now = Utc::now();
    Milestone {
        slug: slug.to_string(),
        name: format!("Milestone {slug}"),
        description: None,
        status,
        chunks,
        completed_chunks: vec![],
        depends_on: vec![],
        pr_branch: Some("feat/my-feature".to_string()),
        pr_base: Some("main".to_string()),
        pr_number: None,
        pr_url: None,
        pr_labels: None,
        pr_reviewers: None,
        pr_body_template: None,
        created_at: now,
        updated_at: now,
    }
}

/// Create a minimal passing gates.toml spec for the given chunk slug inside the assay_dir.
fn create_passing_spec(assay_dir: &Path, chunk_slug: &str) {
    let spec_dir = assay_dir.join("specs").join(chunk_slug);
    fs::create_dir_all(&spec_dir).expect("create spec dir");
    let gates_toml = format!(
        "name = \"{chunk_slug}\"\n\n[[criteria]]\nname = \"pass\"\ndescription = \"always passes\"\ncmd = \"true\"\n"
    );
    fs::write(spec_dir.join("gates.toml"), gates_toml).expect("write gates.toml");
}

/// Create a minimal failing gates.toml spec for the given chunk slug inside the assay_dir.
fn create_failing_spec(assay_dir: &Path, chunk_slug: &str) {
    let spec_dir = assay_dir.join("specs").join(chunk_slug);
    fs::create_dir_all(&spec_dir).expect("create spec dir");
    let gates_toml = format!(
        "name = \"{chunk_slug}\"\n\n[[criteria]]\nname = \"fail\"\ndescription = \"always fails\"\ncmd = \"false\"\n"
    );
    fs::write(spec_dir.join("gates.toml"), gates_toml).expect("write gates.toml");
}

/// Write a fake `gh` script to `dir/gh` that exits with `exit_code` and prints `stdout`.
fn write_fake_gh(dir: &Path, exit_code: i32, stdout: &str) {
    let script_path = dir.join("gh");
    let script = format!(
        "#!/bin/sh\necho '{}'\nexit {}\n",
        stdout.replace('\'', "'\\''"),
        exit_code
    );
    fs::write(&script_path, script).expect("write fake gh script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Prepend `dir` to `PATH`, run `f`, then restore original `PATH`.
///
/// # Safety
/// This modifies the process environment variable `PATH`. Tests using this
/// helper must be annotated with `#[serial]` to prevent concurrent access.
fn with_mock_gh_path<R, F: FnOnce(&Path) -> R>(dir: &Path, f: F) -> R {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), original_path);
    // SAFETY: guarded by #[serial] on all callers; no other threads modify PATH concurrently.
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = f(dir);
    unsafe { std::env::set_var("PATH", original_path) };
    result
}

// ── Test 1: All specs pass → pr_check_milestone_gates returns Ok(vec![]) ─────

#[test]
fn test_pr_check_all_pass() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_passing_spec(&assay_dir, "chunk-a");
    create_passing_spec(&assay_dir, "chunk-b");

    let milestone = make_milestone_with_status(
        "all-pass",
        MilestoneStatus::Verify,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let failures = pr_check_milestone_gates(&assay_dir, &specs_dir, &working_dir, "all-pass")
        .expect("pr_check_milestone_gates should succeed");

    assert!(
        failures.is_empty(),
        "expected no failures when all gates pass, got: {failures:?}"
    );
}

// ── Test 2: One spec fails → returns ChunkGateFailure with required_failed > 0

#[test]
fn test_pr_check_one_fails() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_passing_spec(&assay_dir, "pass-chunk");
    create_failing_spec(&assay_dir, "fail-chunk");

    let milestone = make_milestone_with_status(
        "one-fail",
        MilestoneStatus::Verify,
        vec![
            ChunkRef {
                slug: "pass-chunk".to_string(),
                order: 1,
            },
            ChunkRef {
                slug: "fail-chunk".to_string(),
                order: 2,
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let failures = pr_check_milestone_gates(&assay_dir, &specs_dir, &working_dir, "one-fail")
        .expect("pr_check_milestone_gates should return Ok with failures listed");

    assert_eq!(
        failures.len(),
        1,
        "expected exactly 1 failure, got: {failures:?}"
    );
    let failure: &ChunkGateFailure = &failures[0];
    assert_eq!(
        failure.chunk_slug, "fail-chunk",
        "failure should reference fail-chunk, got: {}",
        failure.chunk_slug
    );
    assert!(
        failure.required_failed > 0,
        "required_failed must be > 0, got: {}",
        failure.required_failed
    );
}

// ── Test 3: Missing spec → Err(AssayError::Io) ───────────────────────────────

#[test]
fn test_pr_check_missing_spec() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    // Milestone references "ghost-chunk" which has no spec on disk
    let milestone = make_milestone_with_status(
        "missing-spec",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "ghost-chunk".to_string(),
            order: 1,
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = pr_check_milestone_gates(&assay_dir, &specs_dir, &working_dir, "missing-spec");
    assert!(result.is_err(), "expected Err when spec is missing, got Ok");
    // Should be an Io error — cannot open the non-existent gates.toml
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ghost-chunk") || msg.contains("gates.toml") || msg.contains("No such file"),
        "error should reference the missing spec, got: {msg}"
    );
}

// ── Test 4: PR already created → Err with "PR already created" ───────────────

#[test]
fn test_pr_create_already_created() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_passing_spec(&assay_dir, "chunk-a");

    let mut milestone = make_milestone_with_status(
        "already-created",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    // PR has already been created
    milestone.pr_number = Some(42);
    milestone.pr_url = Some("https://github.com/owner/repo/pull/42".to_string());
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = pr_create_if_gates_pass(
        &assay_dir,
        &specs_dir,
        &working_dir,
        "already-created",
        "feat: already-created",
        None,
        &[],
        &[],
    );
    assert!(
        result.is_err(),
        "expected Err when PR is already created, got Ok"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("PR already created") || msg.contains("already created"),
        "error message should mention 'PR already created', got: {msg}"
    );
}

// ── Test 5: Gates fail → Err, gh not invoked, milestone TOML unchanged ────────

#[test]
fn test_pr_create_gates_fail() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_failing_spec(&assay_dir, "fail-chunk");

    let milestone = make_milestone_with_status(
        "gates-fail",
        MilestoneStatus::InProgress,
        vec![ChunkRef {
            slug: "fail-chunk".to_string(),
            order: 1,
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = pr_create_if_gates_pass(
        &assay_dir,
        &specs_dir,
        &working_dir,
        "gates-fail",
        "feat: gates-fail",
        None,
        &[],
        &[],
    );
    assert!(result.is_err(), "expected Err when gates fail, got Ok");

    // Milestone TOML must not have been modified — pr_number still None
    let reloaded =
        milestone_load(&assay_dir, "gates-fail").expect("load milestone after failed attempt");
    assert!(
        reloaded.pr_number.is_none(),
        "pr_number must remain None after gate failure, got: {:?}",
        reloaded.pr_number
    );
}

// ── Test 6: gh not found → Err with "gh CLI not found" ───────────────────────

#[test]
#[serial]
fn test_pr_create_gh_not_found() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let empty_bin_dir = tmp.path().join("empty-bin");
    fs::create_dir_all(&empty_bin_dir).expect("create empty bin dir");

    create_passing_spec(&assay_dir, "chunk-a");

    let milestone = make_milestone_with_status(
        "gh-not-found",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    // PATH must be replaced (not prepended) to ensure `gh` is truly absent —
    // `with_mock_gh_path` prepends and would leave the real `gh` reachable.
    let original_path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: guarded by #[serial]; no concurrent thread modifies PATH.
    unsafe { std::env::set_var("PATH", empty_bin_dir.display().to_string()) };

    let result = pr_create_if_gates_pass(
        &assay_dir,
        &specs_dir,
        &working_dir,
        "gh-not-found",
        "feat: gh-not-found",
        None,
        &[],
        &[],
    );

    unsafe { std::env::set_var("PATH", original_path) };

    assert!(
        result.is_err(),
        "expected Err when gh is not in PATH, got Ok"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("gh CLI not found")
            || msg.contains("gh not found")
            || msg.contains("No such file"),
        "error should mention gh CLI not found, got: {msg}"
    );
}

// ── Test 7: Mock gh exits 0 → Ok(PrCreateResult), milestone TOML updated ─────

#[test]
#[serial]
fn test_pr_create_success_mock_gh() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");

    create_passing_spec(&assay_dir, "chunk-a");
    create_passing_spec(&assay_dir, "chunk-b");

    let milestone = make_milestone_with_status(
        "pr-success",
        MilestoneStatus::Verify,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    write_fake_gh(
        &bin_dir,
        0,
        r#"{"number":42,"url":"https://github.com/owner/repo/pull/42"}"#,
    );

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "pr-success",
            "feat: pr-success",
            None,
            &[],
            &[],
        )
    });

    let outcome = result.expect("pr_create_if_gates_pass should succeed with mock gh");
    assert_eq!(outcome.pr_number, 42, "pr_number should be 42");
    assert_eq!(
        outcome.pr_url, "https://github.com/owner/repo/pull/42",
        "pr_url should match mock gh output"
    );

    // Milestone TOML must be updated with pr_number and pr_url
    let reloaded =
        milestone_load(&assay_dir, "pr-success").expect("reload milestone after PR creation");
    assert_eq!(
        reloaded.pr_number,
        Some(42),
        "milestone TOML must contain pr_number = 42 after PR creation"
    );
    assert_eq!(
        reloaded.pr_url.as_deref(),
        Some("https://github.com/owner/repo/pull/42"),
        "milestone TOML must contain pr_url after PR creation"
    );
}

// ── Test 8: Missing JSON fields → Err (no silent defaults) ───────────────────

#[test]
#[serial]
fn test_pr_create_parse_gh_missing_fields() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin-missing-fields");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");

    create_passing_spec(&assay_dir, "chunk-a");

    let milestone = make_milestone_with_status(
        "missing-fields",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    // Fake gh returns valid JSON but without the required `number` field.
    write_fake_gh(&bin_dir, 0, r#"{"other":"data"}"#);

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "missing-fields",
            "feat: missing-fields",
            None,
            &[],
            &[],
        )
    });

    assert!(
        result.is_err(),
        "expected Err when gh JSON response is missing 'number' field, got Ok"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("number") || msg.contains("missing") || msg.contains("invalid"),
        "error message should describe the missing field, got: {msg}"
    );

    // Milestone TOML must not be modified — pr_number still None
    let reloaded =
        milestone_load(&assay_dir, "missing-fields").expect("load milestone after failed parse");
    assert!(
        reloaded.pr_number.is_none(),
        "pr_number must remain None when JSON parse fails, got: {:?}",
        reloaded.pr_number
    );
}

// ── Test 9: Verify status + all gates pass + mock gh → milestone becomes Complete

#[test]
#[serial]
fn test_pr_create_verify_transitions_to_complete() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin-complete");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");

    create_passing_spec(&assay_dir, "chunk-a");

    let milestone = make_milestone_with_status(
        "verify-to-complete",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    write_fake_gh(
        &bin_dir,
        0,
        r#"{"number":99,"url":"https://github.com/owner/repo/pull/99"}"#,
    );

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "verify-to-complete",
            "feat: verify-to-complete",
            None,
            &[],
            &[],
        )
        .expect("pr_create_if_gates_pass should succeed");
    });

    // Milestone status must have transitioned to Complete
    let reloaded = milestone_load(&assay_dir, "verify-to-complete")
        .expect("reload milestone after PR creation");
    assert_eq!(
        reloaded.status,
        MilestoneStatus::Complete,
        "milestone status should be Complete after successful PR creation from Verify, got: {:?}",
        reloaded.status
    );
    assert_eq!(
        reloaded.pr_number,
        Some(99),
        "pr_number should be 99 after successful PR creation"
    );
}

// ── Test 10: Labels and reviewers passed to gh ───────────────────────────────

/// Write a fake `gh` script that captures all args to a file using NUL separators
/// and returns success JSON. Args are separated by NUL bytes so multiline values
/// (like PR body templates with newlines) don't break parsing.
fn write_arg_capturing_gh(dir: &Path, args_file: &Path) {
    let script_path = dir.join("gh");
    let script = format!(
        r#"#!/bin/sh
# Write all arguments to the capture file, NUL-separated
for arg in "$@"; do
    printf '%s\0' "$arg" >> "{args_file}"
done
echo '{{"number":77,"url":"https://github.com/owner/repo/pull/77"}}'
exit 0
"#,
        args_file = args_file.display()
    );
    fs::write(&script_path, script).expect("write arg-capturing gh script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Read captured args from a NUL-separated file.
fn read_captured_args(args_file: &Path) -> Vec<String> {
    let content = fs::read(args_file).expect("read captured args file");
    content
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect()
}

#[test]
#[serial]
fn test_pr_create_passes_labels_and_reviewers() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin-labels");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");
    let args_file = tmp.path().join("captured_args.txt");

    create_passing_spec(&assay_dir, "chunk-a");

    let mut milestone = make_milestone_with_status(
        "labels-test",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    milestone.pr_labels = Some(vec!["ready-for-review".to_string()]);
    milestone.pr_reviewers = Some(vec!["teammate".to_string()]);
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    write_arg_capturing_gh(&bin_dir, &args_file);

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "labels-test",
            "feat: labels-test",
            None,
            &["extra-label".to_string()],
            &["extra-reviewer".to_string()],
        )
    });

    let outcome = result.expect("pr_create_if_gates_pass should succeed");
    assert_eq!(outcome.pr_number, 77);

    // Verify the captured args contain --label and --reviewer flags
    let args = read_captured_args(&args_file);

    // Check TOML label
    assert!(
        args.windows(2).any(|w| w[0] == "--label" && w[1] == "ready-for-review"),
        "expected --label ready-for-review in args, got: {args:?}"
    );
    // Check extra label
    assert!(
        args.windows(2).any(|w| w[0] == "--label" && w[1] == "extra-label"),
        "expected --label extra-label in args, got: {args:?}"
    );
    // Check TOML reviewer
    assert!(
        args.windows(2).any(|w| w[0] == "--reviewer" && w[1] == "teammate"),
        "expected --reviewer teammate in args, got: {args:?}"
    );
    // Check extra reviewer
    assert!(
        args.windows(2).any(|w| w[0] == "--reviewer" && w[1] == "extra-reviewer"),
        "expected --reviewer extra-reviewer in args, got: {args:?}"
    );
}

// ── Test 11: Body template rendered and passed to gh ─────────────────────────

#[test]
#[serial]
fn test_pr_create_renders_body_template() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin-template");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");
    let args_file = tmp.path().join("captured_args_template.txt");

    create_passing_spec(&assay_dir, "chunk-a");
    create_passing_spec(&assay_dir, "chunk-b");

    let mut milestone = make_milestone_with_status(
        "template-test",
        MilestoneStatus::Verify,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
            },
        ],
    );
    milestone.pr_body_template =
        Some("PR for {milestone_name} ({milestone_slug})\n\nChunks:\n{chunk_list}\n\nGates:\n{gate_summary}".to_string());
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    write_arg_capturing_gh(&bin_dir, &args_file);

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "template-test",
            "feat: template-test",
            None,
            &[],
            &[],
        )
    });

    result.expect("pr_create_if_gates_pass should succeed");

    // Verify the --body arg contains the rendered template
    let args = read_captured_args(&args_file);

    // Find the --body arg and the value after it
    let body_idx = args.iter().position(|a| a == "--body").expect("--body flag should be present");
    let body_value = &args[body_idx + 1];

    assert!(
        body_value.contains("Milestone template-test"),
        "body should contain milestone name, got: {body_value}"
    );
    assert!(
        body_value.contains("template-test"),
        "body should contain milestone slug, got: {body_value}"
    );
    assert!(
        body_value.contains("chunk-a") && body_value.contains("chunk-b"),
        "body should contain chunk slugs, got: {body_value}"
    );
    assert!(
        body_value.contains("passed"),
        "body should contain gate summary with 'passed', got: {body_value}"
    );
}

// ── Test 12: Caller body takes precedence over template ──────────────────────

#[test]
#[serial]
fn test_pr_create_caller_body_overrides_template() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    let bin_dir = tmp.path().join("mock-bin-override");
    fs::create_dir_all(&bin_dir).expect("create mock bin dir");
    let args_file = tmp.path().join("captured_args_override.txt");

    create_passing_spec(&assay_dir, "chunk-a");

    let mut milestone = make_milestone_with_status(
        "override-test",
        MilestoneStatus::Verify,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
        }],
    );
    milestone.pr_body_template = Some("TEMPLATE BODY".to_string());
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    write_arg_capturing_gh(&bin_dir, &args_file);

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = with_mock_gh_path(&bin_dir, |_| {
        pr_create_if_gates_pass(
            &assay_dir,
            &specs_dir,
            &working_dir,
            "override-test",
            "feat: override-test",
            Some("CALLER BODY"),
            &[],
            &[],
        )
    });

    result.expect("pr_create_if_gates_pass should succeed");

    let args = read_captured_args(&args_file);

    let body_idx = args.iter().position(|a| a == "--body").expect("--body flag should be present");
    let body_value = &args[body_idx + 1];

    assert_eq!(
        body_value, "CALLER BODY",
        "caller-provided body should take precedence over template, got: {body_value}"
    );
}
