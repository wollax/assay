//! Integration tests for the Docker Compose container lifecycle.
//!
//! These tests require a running Docker daemon with `docker compose` available.
//! They exercise the full `ComposeProvider` lifecycle: provision → exec → teardown.
//!
//! Tests use `alpine:3` as a lightweight agent image. When Docker or `docker compose`
//! is unavailable, tests skip gracefully instead of failing.

use std::collections::HashMap;

use indexmap::IndexMap;
use smelt_core::compose::ComposeProvider;
use smelt_core::manifest::{
    ComposeService, CredentialConfig, Environment, JobManifest, JobMeta, MergeConfig, SessionDef,
};
use smelt_core::provider::RuntimeProvider;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Try to connect to Docker and verify `docker compose` is available.
///
/// Returns `None` (skipping) if:
/// - `docker compose version` fails (compose plugin not installed), or
/// - `ComposeProvider::new()` fails (Docker daemon not available).
///
/// Returns `Some(provider)` when both checks pass.
fn compose_provider_or_skip() -> Option<ComposeProvider> {
    let compose_ok = std::process::Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !compose_ok {
        eprintln!("Skipping: docker compose not available");
        return None;
    }

    match ComposeProvider::new() {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Skipping: Docker daemon not available: {e}");
            None
        }
    }
}

/// Build a minimal [`JobManifest`] for Compose lifecycle tests.
///
/// Sets `runtime = "compose"`, `image = "alpine:3"`, `job.repo` to the
/// current crate's manifest directory (a stable local path), and injects
/// the given services.
fn compose_manifest(name: &str, services: Vec<ComposeService>) -> JobManifest {
    JobManifest {
        job: JobMeta {
            name: name.to_string(),
            repo: env!("CARGO_MANIFEST_DIR").to_string(),
            base_ref: "main".to_string(),
        },
        environment: Environment {
            runtime: "compose".to_string(),
            image: "alpine:3".to_string(),
            resources: HashMap::new(),
        },
        credentials: CredentialConfig {
            provider: "none".to_string(),
            model: "none".to_string(),
            env: HashMap::new(),
        },
        session: vec![SessionDef {
            name: "test".to_string(),
            spec: "test session".to_string(),
            harness: "echo ok".to_string(),
            timeout: 60,
            depends_on: vec![],
        }],
        merge: MergeConfig {
            strategy: "sequential".to_string(),
            order: vec![],
            ai_resolution: false,
            target: "main".to_string(),
        },
        forge: None,
        kubernetes: None,
        state_backend: None,
        services,
        runtime_env: HashMap::new(),
    }
}

/// Remove any containers for the given job name that may have been left behind
/// by a prior failed test run (D041/D042 pattern — job-specific label value).
///
/// Tolerates empty output silently. Errors in cleanup are logged but not fatal.
fn pre_clean_containers(job_name: &str) {
    let filter = format!("label=smelt.job={job_name}");
    let ps = std::process::Command::new("docker")
        .args(["ps", "-q", "--filter", &filter])
        .output();

    match ps {
        Err(e) => {
            eprintln!("pre_clean_containers: docker ps failed: {e}");
        }
        Ok(out) => {
            let ids = String::from_utf8_lossy(&out.stdout);
            let ids: Vec<&str> = ids.split_whitespace().collect();
            if !ids.is_empty() {
                let mut rm = std::process::Command::new("docker");
                rm.arg("rm").arg("-f");
                for id in &ids {
                    rm.arg(id);
                }
                if let Err(e) = rm.output() {
                    eprintln!("pre_clean_containers: docker rm failed: {e}");
                }
            }
        }
    }
}

/// Run `docker ps -q --filter label=smelt.job=<name>` and assert the output is empty.
fn assert_no_containers_for_job(job_name: &str) {
    let filter = format!("label=smelt.job={job_name}");
    let ps = std::process::Command::new("docker")
        .args(["ps", "-q", "--filter", &filter])
        .output()
        .expect("docker ps should work");
    let remaining = String::from_utf8_lossy(&ps.stdout);
    assert!(
        remaining.trim().is_empty(),
        "containers for job '{job_name}' should be gone after teardown, still running:\n{remaining}"
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Test 1: provision + exec + teardown with an empty services list.
///
/// Uses `alpine:3` as the agent image, no sidecars. Provisions, runs `echo hello`,
/// asserts exit code 0 and stdout, tears down, then confirms no containers remain.
#[tokio::test]
async fn test_compose_provision_exec_teardown() {
    pre_clean_containers("compose-test-basic");

    let Some(provider) = compose_provider_or_skip() else {
        return;
    };

    let manifest = compose_manifest("compose-test-basic", vec![]);
    let container = provider.provision(&manifest).await.unwrap();

    let handle = provider
        .exec(&container, &["echo".into(), "hello".into()])
        .await
        .unwrap();

    assert_eq!(handle.exit_code, 0, "echo should exit 0");
    assert_eq!(
        handle.stdout.trim(),
        "hello",
        "stdout should be 'hello', got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.unwrap();

    assert_no_containers_for_job("compose-test-basic");
}

/// Test 2: healthcheck wait with a real Postgres sidecar.
///
/// Provisions a `postgres:16-alpine` service with a `pg_isready` healthcheck.
/// Proves that `provision()` only returns after postgres is healthy by the fact
/// that it returns without error at all (timeout = 120s). Then executes a
/// connectivity check from the agent container to confirm network reachability.
#[tokio::test]
async fn test_compose_healthcheck_wait_postgres() {
    pre_clean_containers("compose-test-postgres");

    let Some(provider) = compose_provider_or_skip() else {
        return;
    };

    // Build the postgres service with a pg_isready healthcheck and POSTGRES_PASSWORD env.
    let mut hc_table = toml::value::Table::new();
    hc_table.insert(
        "test".to_string(),
        toml::Value::Array(vec![
            toml::Value::String("CMD".to_string()),
            toml::Value::String("pg_isready".to_string()),
            toml::Value::String("-U".to_string()),
            toml::Value::String("postgres".to_string()),
        ]),
    );
    hc_table.insert(
        "interval".to_string(),
        toml::Value::String("2s".to_string()),
    );
    hc_table.insert("retries".to_string(), toml::Value::Integer(10));

    let mut env_table = toml::value::Table::new();
    env_table.insert(
        "POSTGRES_PASSWORD".to_string(),
        toml::Value::String("test".to_string()),
    );

    let mut extra: IndexMap<String, toml::Value> = IndexMap::new();
    extra.insert("healthcheck".to_string(), toml::Value::Table(hc_table));
    extra.insert("environment".to_string(), toml::Value::Table(env_table));

    let postgres_service = ComposeService {
        name: "postgres".to_string(),
        image: "postgres:16-alpine".to_string(),
        extra,
    };

    let manifest = compose_manifest("compose-test-postgres", vec![postgres_service]);

    // provision() must return without error — that itself proves the healthcheck wait worked.
    let container = provider
        .provision(&manifest)
        .await
        .expect("provision must succeed after postgres is healthy");

    // Exec a connectivity check from the agent container to confirm postgres is reachable.
    // `nc -z postgres 5432` checks TCP reachability on the shared compose network.
    let handle = provider
        .exec(
            &container,
            &[
                "sh".into(),
                "-c".into(),
                "nc -z postgres 5432 && echo ok".into(),
            ],
        )
        .await
        .expect("exec should not fail");

    assert_eq!(
        handle.exit_code, 0,
        "connectivity check to postgres should exit 0, stderr: {}",
        handle.stderr
    );
    assert!(
        handle.stdout.contains("ok"),
        "stdout should contain 'ok', got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.unwrap();

    assert_no_containers_for_job("compose-test-postgres");
}

/// Test 3: teardown after a failing exec command.
///
/// Provisions with no sidecars, runs a command that exits 1, then verifies
/// teardown still succeeds and no containers remain. Validates the idempotent
/// teardown path described in D023/D038.
#[tokio::test]
async fn test_compose_teardown_after_exec_error() {
    pre_clean_containers("compose-test-teardown-err");

    let Some(provider) = compose_provider_or_skip() else {
        return;
    };

    let manifest = compose_manifest("compose-test-teardown-err", vec![]);
    let container = provider.provision(&manifest).await.unwrap();

    let handle = provider
        .exec(&container, &["sh".into(), "-c".into(), "exit 1".into()])
        .await
        .unwrap();

    assert_eq!(
        handle.exit_code, 1,
        "exit 1 command should return exit_code 1"
    );

    // Teardown must not panic or return an error even after exec failure.
    provider
        .teardown(&container)
        .await
        .expect("teardown must succeed after exec error");

    assert_no_containers_for_job("compose-test-teardown-err");
}
