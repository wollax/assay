//! Integration tests for `smelt run --dry-run`.

use assert_cmd::Command;
use predicates::prelude::*;

/// Return the workspace root directory (where `Cargo.toml` and `examples/` live).
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .to_path_buf()
}

fn smelt() -> Command {
    let mut cmd = Command::cargo_bin("smelt").expect("binary should be built");
    cmd.current_dir(workspace_root());
    cmd
}

// ── Happy path ─────────────────────────────────────────────────

#[test]
fn dry_run_valid_manifest_prints_execution_plan() {
    smelt()
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("═══ Execution Plan ═══")
                .and(predicate::str::contains("add-user-auth"))
                .and(predicate::str::contains("alpine:3"))
                .and(predicate::str::contains("frontend"))
                .and(predicate::str::contains("backend"))
                .and(predicate::str::contains("integration"))
                .and(predicate::str::contains("sequential"))
                .and(predicate::str::contains("anthropic"))
                .and(predicate::str::contains("claude-sonnet-4-20250514"))
                .and(predicate::str::contains("═══ End Plan ═══")),
        );
}

#[test]
fn dry_run_shows_session_details() {
    smelt()
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Timeout:    300s")
                .and(predicate::str::contains("Timeout:    600s"))
                .and(predicate::str::contains("Timeout:    900s"))
                .and(predicate::str::contains("Depends on: frontend, backend")),
        );
}

#[test]
fn dry_run_shows_resources() {
    smelt()
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("cpu")
                .and(predicate::str::contains("memory"))
                .and(predicate::str::contains("4G")),
        );
}

#[test]
fn dry_run_shows_merge_config() {
    smelt()
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Strategy:      sequential")
                .and(predicate::str::contains("Target:        main"))
                .and(predicate::str::contains("AI resolution: enabled"))
                .and(predicate::str::contains("frontend → backend → integration")),
        );
}

#[test]
fn dry_run_shows_credential_status() {
    // Credential env var is unset in CI/test environments, expect MISSING.
    smelt()
        .env_remove("ANTHROPIC_API_KEY")
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env:ANTHROPIC_API_KEY → MISSING"));
}

#[test]
fn dry_run_credential_resolved_when_set() {
    smelt()
        .env("ANTHROPIC_API_KEY", "test-secret-value")
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("env:ANTHROPIC_API_KEY → resolved")
                // Credential value must NEVER appear in output
                .and(predicate::str::contains("test-secret-value").not()),
        );
}

// ── Validation errors ──────────────────────────────────────────

#[test]
fn dry_run_bad_manifest_exits_with_error() {
    smelt()
        .args(["run", "examples/bad-manifest.toml", "--dry-run"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Validation failed")
                .or(predicate::str::contains("manifest validation failed")),
        );
}

#[test]
fn dry_run_nonexistent_manifest_exits_with_error() {
    smelt()
        .args(["run", "nonexistent.toml", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read").or(predicate::str::contains("failed to load")));
}

// ── Without --dry-run ──────────────────────────────────────────

#[test]
fn run_without_dry_run_attempts_docker() {
    // Running without --dry-run attempts the full Docker lifecycle.
    //
    // This test is environment-sensitive:
    // - No Docker daemon  → provider connection error (exit 1)
    // - Docker available, no assay binary in container → assay not found (exit 1)
    // - Docker available, assay present → pipeline runs (exit 0 or 1 depending on result)
    //
    // Phase 5.5 wiring (S02/T01): Before assay is invoked, execute_run() now runs
    // three setup steps inside the container:
    //   1. Write /workspace/.assay/config.toml (via sh -c base64 -d)
    //   2. Create /workspace/.assay/specs/ directory (mkdir -p)
    //   3. Write per-session spec TOML files under /workspace/.assay/specs/
    // All three steps succeed in alpine:3 (sh, mkdir, and base64 are all available).
    // After Phase 5.5, assay is invoked and exits 127 (not found) — a non-zero exit
    // code — so this test still observes a failure. The test behavior is unchanged.
    //
    // The important invariant: the OLD "not implemented" stub message must NOT appear.
    let assert = smelt()
        .args(["run", "examples/job-manifest.toml"])
        .assert();

    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Must never see the old placeholder message from before S02
    assert!(
        !stderr.contains("not implemented"),
        "unexpected stub message — Docker lifecycle should be wired: {stderr}"
    );
}

// ── Forge section ─────────────────────────────────────────────

#[test]
fn test_dry_run_with_forge_shows_forge_section() {
    smelt()
        .args(["run", "examples/job-manifest-forge.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("── Forge ──")
                .and(predicate::str::contains("github"))
                .and(predicate::str::contains("owner/my-repo"))
                .and(predicate::str::contains("GITHUB_TOKEN"))
                .and(predicate::str::contains("--no-pr")),
        );
}

#[test]
fn test_dry_run_no_pr_flag_accepted() {
    // --no-pr only affects live runs; dry-run still shows the forge section.
    smelt()
        .args(["run", "examples/job-manifest-forge.toml", "--dry-run", "--no-pr"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("── Forge ──")
                .and(predicate::str::contains("owner/my-repo"))
                .and(predicate::str::contains("GITHUB_TOKEN")),
        );
}

// ── smelt init → dry-run smoke ─────────────────────────────────

/// End-to-end smoke: `smelt init` in a fresh tempdir, then `smelt run --dry-run`
/// against the generated manifest.  Proves the skeleton is always valid and that
/// the init→dry-run path works without Docker or credentials.
#[test]
fn test_init_then_dry_run_smoke() {
    let dir = tempfile::TempDir::new().expect("tempdir should be created");

    // Step 1: smelt init
    Command::cargo_bin("smelt")
        .expect("binary should be built")
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job-manifest.toml"));

    // Manifest must exist after init
    assert!(
        dir.path().join("job-manifest.toml").exists(),
        "job-manifest.toml must exist after smelt init"
    );

    // Step 2: smelt run --dry-run against the generated manifest
    Command::cargo_bin("smelt")
        .expect("binary should be built")
        .current_dir(dir.path())
        .args(["run", "job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("═══ Execution Plan ═══")
                .and(predicate::str::contains("my-job"))
                .and(predicate::str::contains("═══ End Plan ═══")
                    .or(predicate::str::contains("End Plan"))),
        );
}

// ── Compose services section ──────────────────────────────────

#[test]
fn dry_run_compose_manifest_shows_services_section() {
    smelt()
        .args(["run", "examples/job-manifest-compose.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("── Compose Services ──")
                .and(predicate::str::contains("postgres"))
                .and(predicate::str::contains("postgres:16-alpine")),
        );
}

#[test]
fn dry_run_docker_manifest_no_services_section() {
    smelt()
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("── Compose Services ──").not());
}

// ── Kubernetes section ────────────────────────────────────────

#[test]
fn dry_run_kubernetes_manifest_shows_kubernetes_section() {
    smelt()
        .args(["run", "examples/job-manifest-k8s.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("── Kubernetes ──")
                .and(predicate::str::contains("smelt"))
                .and(predicate::str::contains("ambient")),
        );
}

// ── Credential value never leaked ──────────────────────────────

#[test]
fn dry_run_never_prints_credential_values() {
    let secret = "super-secret-credential-value-12345";
    smelt()
        .env("ANTHROPIC_API_KEY", secret)
        .args(["run", "examples/job-manifest.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(secret).not());
}
