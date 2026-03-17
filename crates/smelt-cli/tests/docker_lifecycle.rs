//! Integration tests for the Docker container lifecycle.
//!
//! These tests require a running Docker daemon. They exercise the full
//! `RuntimeProvider` lifecycle: provision → exec → collect → teardown.
//!
//! Tests use `alpine:3` as a lightweight base image. When the Docker daemon
//! is unavailable, tests skip gracefully instead of failing.

use std::collections::HashMap;

use bollard::query_parameters::InspectContainerOptions;
use smelt_core::docker::DockerProvider;
use smelt_core::manifest::{
    CredentialConfig, Environment, JobManifest, JobMeta, MergeConfig, SessionDef,
};
use smelt_core::provider::RuntimeProvider;

/// Try to connect to Docker. Returns `None` if the daemon is unavailable,
/// allowing tests to skip gracefully instead of panicking.
fn docker_provider_or_skip() -> Option<DockerProvider> {
    match DockerProvider::new() {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Skipping Docker test — daemon not available: {e}");
            None
        }
    }
}

/// Build a minimal [`JobManifest`] suitable for Docker lifecycle tests.
fn test_manifest(name: &str) -> JobManifest {
    JobManifest {
        job: JobMeta {
            name: name.to_string(),
            repo: "https://github.com/example/test".to_string(),
            base_ref: "main".to_string(),
        },
        environment: Environment {
            runtime: "docker".to_string(),
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
    }
}

/// Verify that the container no longer exists via the bollard API.
async fn assert_container_removed(provider: &DockerProvider, container_id: &str) {
    let result = provider
        .client()
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await;
    assert!(
        result.is_err(),
        "container {container_id} should have been removed"
    );
}

// ── RuntimeProvider-level tests ────────────────────────────────────────

#[tokio::test]
async fn test_provision_and_teardown() {
    let Some(provider) = docker_provider_or_skip() else { return };
    let manifest = test_manifest("provision-teardown");

    // Provision a container
    let container = provider.provision(&manifest).await.expect("provision should succeed");
    let container_id = container.as_str().to_string();

    // Verify the container exists
    let info = provider
        .client()
        .inspect_container(&container_id, None::<InspectContainerOptions>)
        .await
        .expect("container should exist after provision");
    assert!(info.id.is_some(), "container should have an ID");

    // Tear down
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");

    // Verify the container is gone
    assert_container_removed(&provider, &container_id).await;
}

#[tokio::test]
async fn test_exec() {
    let Some(provider) = docker_provider_or_skip() else { return };
    let manifest = test_manifest("exec-hello");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute `echo "hello world"`
    let cmd = vec![
        "echo".to_string(),
        "hello world".to_string(),
    ];
    let handle = provider.exec(&container, &cmd).await.expect("exec should succeed");
    assert_eq!(handle.container, container);
    assert_eq!(handle.exit_code, 0, "echo should exit 0");
    assert!(
        handle.stdout.contains("hello world"),
        "stdout should contain 'hello world', got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_exec_nonzero_exit() {
    let Some(provider) = docker_provider_or_skip() else { return };
    let manifest = test_manifest("exec-nonzero");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute a command that exits with code 42
    let cmd = vec!["sh".to_string(), "-c".to_string(), "exit 42".to_string()];
    let handle = provider.exec(&container, &cmd).await.expect("exec should succeed");
    assert_eq!(handle.container, container);
    assert_eq!(handle.exit_code, 42, "should capture non-zero exit code");

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_exec_long_running() {
    let Some(provider) = docker_provider_or_skip() else { return };
    let manifest = test_manifest("exec-long-running");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute a multi-second command that prints incrementally
    let cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        "for i in 1 2 3; do echo step-$i; sleep 1; done".to_string(),
    ];
    let handle = provider.exec(&container, &cmd).await.expect("exec should succeed");
    assert_eq!(handle.exit_code, 0, "long-running command should exit 0");
    assert!(
        handle.stdout.contains("step-1"),
        "stdout should contain 'step-1', got: {:?}",
        handle.stdout
    );
    assert!(
        handle.stdout.contains("step-2"),
        "stdout should contain 'step-2', got: {:?}",
        handle.stdout
    );
    assert!(
        handle.stdout.contains("step-3"),
        "stdout should contain 'step-3', got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_teardown_on_error() {
    let Some(provider) = docker_provider_or_skip() else { return };
    let manifest = test_manifest("teardown-on-error");

    let container = provider.provision(&manifest).await.expect("provision");
    let container_id = container.as_str().to_string();

    // Execute a command that fails
    let cmd = vec!["sh".to_string(), "-c".to_string(), "exit 1".to_string()];
    let _ = provider.exec(&container, &cmd).await;

    // Teardown must still succeed even after a failed command
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed even after exec failure");

    assert_container_removed(&provider, &container_id).await;
}

// ── CLI-level tests ────────────────────────────────────────────────────

/// Test the full CLI lifecycle: `smelt run examples/job-manifest.toml`
/// This exercises provision → health-check exec → teardown from the real binary.
#[tokio::test]
async fn test_cli_run_lifecycle() {
    // Skip if Docker daemon is unavailable
    if docker_provider_or_skip().is_none() {
        return;
    }

    let cmd = assert_cmd::Command::cargo_bin("smelt")
        .expect("binary should exist")
        .arg("run")
        .arg("examples/job-manifest.toml")
        .timeout(std::time::Duration::from_secs(120))
        .output()
        .expect("failed to run smelt");

    let stderr = String::from_utf8_lossy(&cmd.stderr);

    // The health check output is streamed to stderr via eprint! in exec()
    assert!(
        stderr.contains("smelt: container ready"),
        "output should contain health check message, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Container removed"),
        "output should confirm teardown, stderr:\n{stderr}"
    );
    assert!(
        cmd.status.success(),
        "smelt run should exit 0, stderr:\n{stderr}"
    );

    // Verify no leaked containers
    let ps = std::process::Command::new("docker")
        .args(["ps", "-a", "--filter", "label=smelt.job", "-q"])
        .output()
        .expect("docker ps should work");
    let remaining = String::from_utf8_lossy(&ps.stdout);
    assert!(
        remaining.trim().is_empty(),
        "no smelt containers should remain, got: {remaining}"
    );
}

/// Test that running with an invalid manifest produces exit code 1 and an error message.
#[tokio::test]
async fn test_cli_run_invalid_manifest() {
    let cmd = assert_cmd::Command::cargo_bin("smelt")
        .expect("binary should exist")
        .arg("run")
        .arg("nonexistent-manifest.toml")
        .timeout(std::time::Duration::from_secs(10))
        .output()
        .expect("failed to run smelt");

    assert!(
        !cmd.status.success(),
        "smelt run with bad manifest should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&cmd.stderr);
    assert!(
        stderr.contains("Error"),
        "stderr should contain error message, got:\n{stderr}"
    );
}
