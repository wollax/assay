//! Integration tests for the Docker container lifecycle.
//!
//! These tests require a running Docker daemon. They exercise the full
//! `RuntimeProvider` lifecycle: provision → exec → collect → teardown.
//!
//! Tests use `alpine:3` as a lightweight base image. When the Docker daemon
//! is unavailable, tests skip gracefully instead of failing.

use std::collections::HashMap;

use base64::Engine as _;
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
///
/// `repo` should be a local filesystem path (bind-mounted into the container).
fn test_manifest_with_repo(name: &str, repo: &str) -> JobManifest {
    JobManifest {
        job: JobMeta {
            name: name.to_string(),
            repo: repo.to_string(),
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

/// Build a test manifest with repo set to the current directory.
fn test_manifest(name: &str) -> JobManifest {
    // Use the project root (or cwd) as a valid local path.
    test_manifest_with_repo(name, ".")
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

/// Test the full CLI lifecycle: `smelt run <manifest>` with a local repo path.
/// This exercises provision → health-check exec → teardown from the real binary.
#[tokio::test]
async fn test_cli_run_lifecycle() {
    // Skip if Docker daemon is unavailable
    if docker_provider_or_skip().is_none() {
        return;
    }

    // Create a temp dir as the repo path and a manifest pointing to it
    let repo_dir = tempfile::tempdir().unwrap();
    let manifest_dir = tempfile::tempdir().unwrap();
    let manifest_path = manifest_dir.path().join("manifest.toml");
    let manifest_content = format!(
        r#"
[job]
name = "cli-lifecycle"
repo = "{}"
base_ref = "main"

[environment]
runtime = "docker"
image = "alpine:3"

[credentials]
provider = "none"
model = "none"

[[session]]
name = "test"
spec = "test"
harness = "echo ok"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#,
        repo_dir.path().display()
    );
    std::fs::write(&manifest_path, manifest_content).unwrap();

    let cmd = assert_cmd::Command::cargo_bin("smelt")
        .expect("binary should exist")
        .arg("run")
        .arg(manifest_path.to_str().unwrap())
        .timeout(std::time::Duration::from_secs(120))
        .output()
        .expect("failed to run smelt");

    let stderr = String::from_utf8_lossy(&cmd.stderr);

    // Verify lifecycle phase messages appear in order
    assert!(
        stderr.contains("Writing manifest..."),
        "output should contain manifest write phase, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Executing assay run..."),
        "output should contain assay execution phase, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Assay complete"),
        "output should contain assay completion, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Container removed"),
        "output should confirm teardown, stderr:\n{stderr}"
    );

    // The assay binary won't exist in alpine, so exit code will be non-zero.
    // But the lifecycle messages should show mount and manifest write succeeded.
    // Don't assert success — the real assay isn't installed in the test container.

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

// ── Bind-mount tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_bind_mount_read() {
    let Some(provider) = docker_provider_or_skip() else { return };

    // Create a temp dir with a test file
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test.txt"), "hello from host").unwrap();

    let manifest = test_manifest_with_repo("mount-read", dir.path().to_str().unwrap());
    let container = provider.provision(&manifest).await.expect("provision");

    // Read the test file inside the container at /workspace
    let cmd = vec!["cat".to_string(), "/workspace/test.txt".to_string()];
    let handle = provider.exec(&container, &cmd).await.expect("exec cat");
    assert_eq!(handle.exit_code, 0, "cat should exit 0");
    assert!(
        handle.stdout.contains("hello from host"),
        "should read host file content, got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_bind_mount_write() {
    let Some(provider) = docker_provider_or_skip() else { return };

    let dir = tempfile::tempdir().unwrap();
    let manifest = test_manifest_with_repo("mount-write", dir.path().to_str().unwrap());
    let container = provider.provision(&manifest).await.expect("provision");

    // Create a file inside the container at /workspace
    let cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        "echo 'written by container' > /workspace/newfile.txt".to_string(),
    ];
    let handle = provider.exec(&container, &cmd).await.expect("exec touch");
    assert_eq!(handle.exit_code, 0, "write should exit 0");

    // Verify the file exists on the host
    let content = std::fs::read_to_string(dir.path().join("newfile.txt"))
        .expect("file should exist on host");
    assert!(
        content.contains("written by container"),
        "host file should have container content, got: {:?}",
        content
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_bind_mount_working_dir() {
    let Some(provider) = docker_provider_or_skip() else { return };

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("marker.txt"), "found it").unwrap();

    let manifest = test_manifest_with_repo("mount-workdir", dir.path().to_str().unwrap());
    let container = provider.provision(&manifest).await.expect("provision");

    // `pwd` should return /workspace (the working_dir set in exec)
    let cmd = vec!["pwd".to_string()];
    let handle = provider.exec(&container, &cmd).await.expect("exec pwd");
    assert_eq!(handle.exit_code, 0);
    assert!(
        handle.stdout.trim().contains("/workspace"),
        "working dir should be /workspace, got: {:?}",
        handle.stdout
    );

    // `cat marker.txt` without path prefix should work via working_dir
    let cmd = vec!["cat".to_string(), "marker.txt".to_string()];
    let handle = provider.exec(&container, &cmd).await.expect("exec cat");
    assert_eq!(handle.exit_code, 0);
    assert!(
        handle.stdout.contains("found it"),
        "should read file via working_dir, got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_repo_url_rejected() {
    let Some(provider) = docker_provider_or_skip() else { return };

    let manifest = test_manifest_with_repo("url-rejected", "https://github.com/example/repo");
    let err = provider.provision(&manifest).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("not a URL"),
        "should reject URL repo with clear message, got: {msg}"
    );
}

// ── CLI-level tests ────────────────────────────────────────────────────

// ── Assay mock tests ───────────────────────────────────────────────────

/// Test the full assay mock execution: mount repo → write manifest → run mock assay.
///
/// Creates a temp repo dir with a marker file, provisions a container, writes
/// the Smelt manifest into it, then runs a mock script that validates:
/// 1. The repo is mounted and the marker file is readable
/// 2. The manifest file is present and contains expected session data
#[tokio::test]
async fn test_assay_mock_execution() {
    let Some(provider) = docker_provider_or_skip() else { return };

    // Create a temp dir with a marker file as the "repo"
    let repo_dir = tempfile::tempdir().unwrap();
    std::fs::write(repo_dir.path().join("marker.txt"), "smelt-test").unwrap();

    // Build manifest with two sessions, one depending on the other
    let mut manifest = test_manifest_with_repo("assay-mock", repo_dir.path().to_str().unwrap());
    manifest.session = vec![
        smelt_core::manifest::SessionDef {
            name: "alpha".to_string(),
            spec: "First session".to_string(),
            harness: "echo ok".to_string(),
            timeout: 60,
            depends_on: vec![],
        },
        smelt_core::manifest::SessionDef {
            name: "beta".to_string(),
            spec: "Second session".to_string(),
            harness: "echo ok".to_string(),
            timeout: 120,
            depends_on: vec!["alpha".to_string()],
        },
    ];

    let container = provider.provision(&manifest).await.expect("provision");

    // Write the assay manifest into the container
    let toml_content = smelt_core::AssayInvoker::build_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml_content)
        .await
        .expect("write manifest");

    // Write a mock assay script that validates the mount and manifest
    let mock_script = r#"#!/bin/sh
set -e
# Check marker file from mounted repo
if [ ! -f /workspace/marker.txt ]; then
    echo "MOCK_ASSAY: ERROR — marker.txt not found" >&2
    exit 1
fi
marker_content=$(cat /workspace/marker.txt)
echo "MOCK_ASSAY: marker=$marker_content"

# Check manifest file
if [ ! -f /tmp/smelt-manifest.toml ]; then
    echo "MOCK_ASSAY: ERROR — manifest not found" >&2
    exit 1
fi

# Count sessions (lines matching 'name = ')
session_count=$(grep -c 'name = ' /tmp/smelt-manifest.toml)
session_names=$(grep 'name = ' /tmp/smelt-manifest.toml | sed 's/.*name = "\(.*\)"/\1/' | tr '\n' ',' | sed 's/,$//')
echo "MOCK_ASSAY: found $session_count sessions: $session_names"
"#;
    let write_script_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /tmp/mock-assay.sh && chmod +x /tmp/mock-assay.sh",
            base64::engine::general_purpose::STANDARD.encode(mock_script.as_bytes())
        ),
    ];
    let write_handle = provider.exec(&container, &write_script_cmd).await.expect("write mock script");
    assert_eq!(write_handle.exit_code, 0, "writing mock script should succeed");

    // Execute the mock assay script
    let run_cmd = vec!["sh".to_string(), "/tmp/mock-assay.sh".to_string()];
    let handle = provider.exec(&container, &run_cmd).await.expect("exec mock assay");

    assert_eq!(handle.exit_code, 0, "mock assay should exit 0, stderr: {}", handle.stderr);
    assert!(
        handle.stdout.contains("MOCK_ASSAY: marker=smelt-test"),
        "should confirm marker file, got: {:?}",
        handle.stdout
    );
    assert!(
        handle.stdout.contains("found 2 sessions"),
        "should find 2 sessions, got: {:?}",
        handle.stdout
    );
    assert!(
        handle.stdout.contains("alpha"),
        "should contain session name 'alpha', got: {:?}",
        handle.stdout
    );
    assert!(
        handle.stdout.contains("beta"),
        "should contain session name 'beta', got: {:?}",
        handle.stdout
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

/// Test that a non-zero assay exit code is surfaced correctly.
#[tokio::test]
async fn test_assay_mock_failure() {
    let Some(provider) = docker_provider_or_skip() else { return };

    let repo_dir = tempfile::tempdir().unwrap();
    let manifest = test_manifest_with_repo("assay-failure", repo_dir.path().to_str().unwrap());

    let container = provider.provision(&manifest).await.expect("provision");

    // Write the assay manifest
    let toml_content = smelt_core::AssayInvoker::build_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml_content)
        .await
        .expect("write manifest");

    // Write a mock script that fails with exit code 1 and stderr output
    let fail_script = r#"#!/bin/sh
echo "MOCK_ASSAY: starting" >&1
echo "MOCK_ASSAY: fatal error — session 'test' harness timed out" >&2
exit 1
"#;
    let write_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /tmp/mock-fail.sh && chmod +x /tmp/mock-fail.sh",
            base64::engine::general_purpose::STANDARD.encode(fail_script.as_bytes())
        ),
    ];
    let wh = provider.exec(&container, &write_cmd).await.expect("write fail script");
    assert_eq!(wh.exit_code, 0);

    // Execute the failing mock
    let run_cmd = vec!["sh".to_string(), "/tmp/mock-fail.sh".to_string()];
    let handle = provider.exec(&container, &run_cmd).await.expect("exec mock fail");

    assert_eq!(handle.exit_code, 1, "mock should exit with code 1");
    assert!(
        handle.stderr.contains("fatal error"),
        "stderr should contain the failure message, got: {:?}",
        handle.stderr
    );

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
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
