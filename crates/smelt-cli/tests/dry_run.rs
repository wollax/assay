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
                .and(predicate::str::contains("node:20-slim"))
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
fn run_without_dry_run_exits_unimplemented() {
    smelt()
        .args(["run", "examples/job-manifest.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Docker execution not yet implemented"));
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
