//! Integration tests for the Docker container lifecycle.
//!
//! These tests require a running Docker daemon. They exercise the full
//! `RuntimeProvider` lifecycle: provision → exec → collect → teardown.
//!
//! Tests use `alpine:3` as a lightweight base image. When the Docker daemon
//! is unavailable, tests skip gracefully instead of failing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use base64::Engine as _;
use bollard::query_parameters::InspectContainerOptions;
use smelt_core::docker::DockerProvider;
use smelt_core::manifest::{
    CredentialConfig, Environment, JobManifest, JobMeta, MergeConfig, SessionDef,
};
use smelt_core::provider::RuntimeProvider;

/// Returns the workspace root directory (two levels up from this crate's manifest).
///
/// `CARGO_MANIFEST_DIR` points to `crates/smelt-cli/`, so `../..` is the workspace root.
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| manifest_dir.join("../.."))
}

/// Build the assay binary for Linux aarch64 inside a `rust:alpine` Docker container.
///
/// - Detects the assay source directory via `ASSAY_SOURCE_DIR` env var, or falls back to
///   `<workspace_root>/../../assay` (sibling repo pattern for local dev).
/// - Returns `None` immediately if neither source location exists, or if the Docker build fails.
/// - On success, caches the binary at `target/smelt-test-cache/assay-linux-aarch64` and returns
///   `Some(cache_path)`. Subsequent calls return the cached path without rebuilding.
fn build_linux_assay_binary() -> Option<std::path::PathBuf> {
    let workspace = workspace_root();

    // Resolve cache path first — cache hit bypasses source detection entirely
    let cache_dir = workspace.join("target/smelt-test-cache");
    let cache_path = cache_dir.join("assay-linux-aarch64");

    if cache_path.exists() {
        return Some(cache_path);
    }

    // Cache miss — need to build from source; detect assay source directory
    let assay_src = if let Ok(dir) = std::env::var("ASSAY_SOURCE_DIR") {
        let p = std::path::PathBuf::from(dir);
        if p.exists() {
            p
        } else {
            eprintln!(
                "Skipping: ASSAY_SOURCE_DIR set but does not exist: {}",
                p.display()
            );
            return None;
        }
    } else {
        // Sibling repo pattern: <workspace_root>/../../assay
        let sibling = workspace.join("../../assay");
        match sibling.canonicalize() {
            Ok(p) if p.exists() => p,
            _ => {
                eprintln!(
                    "Skipping: assay source not found at {} (set ASSAY_SOURCE_DIR to override)",
                    sibling.display()
                );
                return None;
            }
        }
    };

    // Ensure cache directories exist
    let build_dir = cache_dir.join("assay-build");
    if let Err(e) = std::fs::create_dir_all(&build_dir) {
        eprintln!(
            "Failed to create cache build dir {}: {e}",
            build_dir.display()
        );
        return None;
    }

    // Resolve cargo home for registry cache mount
    let cargo_home = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap()));

    let assay_src_str = assay_src.to_string_lossy();
    let build_dir_str = build_dir.to_string_lossy();
    let registry_mount = format!("{cargo_home}/registry:/usr/local/cargo/registry");

    eprintln!(
        "Building Linux aarch64 assay binary from {} ...",
        assay_src_str
    );

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--platform",
            "linux/arm64",
            "-v",
            &format!("{assay_src_str}:/assay:ro"),
            "-v",
            &registry_mount,
            "-v",
            &format!("{build_dir_str}:/build"),
            "-e",
            "CARGO_TARGET_DIR=/build",
            "-w",
            "/assay",
            "rust:alpine",
            "sh",
            "-c",
            "apk add --no-cache musl-dev && cargo build --bin assay 2>&1",
        ])
        .output();

    match output {
        Err(e) => {
            eprintln!("Failed to run docker for Linux assay build: {e}");
            return None;
        }
        Ok(out) if !out.status.success() => {
            eprintln!(
                "Docker build of Linux assay binary failed (exit {:?}):",
                out.status.code()
            );
            eprintln!("{}", String::from_utf8_lossy(&out.stdout));
            eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            return None;
        }
        Ok(_) => {}
    }

    // Copy the built binary to the cache path
    let built_binary = build_dir.join("debug/assay");
    if !built_binary.exists() {
        eprintln!(
            "Build succeeded but binary not found at {}",
            built_binary.display()
        );
        return None;
    }

    if let Err(e) = std::fs::copy(&built_binary, &cache_path) {
        eprintln!("Failed to copy binary to cache: {e}");
        return None;
    }

    eprintln!(
        "Cached Linux aarch64 assay binary at {}",
        cache_path.display()
    );
    Some(cache_path)
}

/// Inject a host file into a running container using `docker cp`.
///
/// Returns `true` iff the `docker cp` command exits successfully.
/// This avoids the base64-exec approach, which is too slow/unreliable for large binaries.
#[allow(dead_code)]
fn inject_binary_to_container(
    container_id: &str,
    host_path: &std::path::Path,
    dest_path: &str,
) -> bool {
    let status = std::process::Command::new("docker")
        .args([
            "cp",
            &host_path.to_string_lossy().to_string(),
            &format!("{container_id}:{dest_path}"),
        ])
        .status();
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("docker cp failed with exit code {:?}", s.code());
            false
        }
        Err(e) => {
            eprintln!("Failed to run docker cp: {e}");
            false
        }
    }
}

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
        forge: None,
        kubernetes: None,
        services: vec![],
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("provision-teardown");

    // Provision a container
    let container = provider
        .provision(&manifest)
        .await
        .expect("provision should succeed");
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("exec-hello");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute `echo "hello world"`
    let cmd = vec!["echo".to_string(), "hello world".to_string()];
    let handle = provider
        .exec(&container, &cmd)
        .await
        .expect("exec should succeed");
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("exec-nonzero");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute a command that exits with code 42
    let cmd = vec!["sh".to_string(), "-c".to_string(), "exit 42".to_string()];
    let handle = provider
        .exec(&container, &cmd)
        .await
        .expect("exec should succeed");
    assert_eq!(handle.container, container);
    assert_eq!(handle.exit_code, 42, "should capture non-zero exit code");

    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

#[tokio::test]
async fn test_exec_long_running() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("exec-long-running");

    let container = provider.provision(&manifest).await.expect("provision");

    // Execute a multi-second command that prints incrementally
    let cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        "for i in 1 2 3; do echo step-$i; sleep 1; done".to_string(),
    ];
    let handle = provider
        .exec(&container, &cmd)
        .await
        .expect("exec should succeed");
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
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

    // Pre-clean any stale smelt containers from prior test runs
    let stale = std::process::Command::new("docker")
        .args(["ps", "-aq", "--filter", "label=smelt.job"])
        .output()
        .expect("docker ps");
    for id in String::from_utf8_lossy(&stale.stdout).split_whitespace() {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", id])
            .output();
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

    #[allow(deprecated)]
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

    // Verify no containers from THIS job remain (filter by job name to avoid
    // interference from concurrent tests running other smelt jobs)
    let ps = std::process::Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "label=smelt.job=cli-lifecycle",
            "-q",
        ])
        .output()
        .expect("docker ps should work");
    let remaining = String::from_utf8_lossy(&ps.stdout);
    assert!(
        remaining.trim().is_empty(),
        "no cli-lifecycle containers should remain, got: {remaining}"
    );
}

// ── Bind-mount tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_bind_mount_read() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

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
    let content =
        std::fs::read_to_string(dir.path().join("newfile.txt")).expect("file should exist on host");
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

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
    let toml_content = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
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
    let write_handle = provider
        .exec(&container, &write_script_cmd)
        .await
        .expect("write mock script");
    assert_eq!(
        write_handle.exit_code, 0,
        "writing mock script should succeed"
    );

    // Execute the mock assay script
    let run_cmd = vec!["sh".to_string(), "/tmp/mock-assay.sh".to_string()];
    let handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec mock assay");

    assert_eq!(
        handle.exit_code, 0,
        "mock assay should exit 0, stderr: {}",
        handle.stderr
    );
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
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    let repo_dir = tempfile::tempdir().unwrap();
    let manifest = test_manifest_with_repo("assay-failure", repo_dir.path().to_str().unwrap());

    let container = provider.provision(&manifest).await.expect("provision");

    // Write the assay manifest
    let toml_content = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
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
    let wh = provider
        .exec(&container, &write_cmd)
        .await
        .expect("write fail script");
    assert_eq!(wh.exit_code, 0);

    // Execute the failing mock
    let run_cmd = vec!["sh".to_string(), "/tmp/mock-fail.sh".to_string()];
    let handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec mock fail");

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
    #[allow(deprecated)]
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

// ── Result collection through Docker pipeline ──────────────────────────

/// Test the full collection pipeline: provision → mock-Assay-that-creates-commits → collect → verify target branch → teardown.
///
/// Creates a real git repo, provisions a Docker container with a bind-mount,
/// runs a mock script that creates commits inside /workspace, then verifies
/// that `ResultCollector::collect()` creates the target branch on the host.
#[tokio::test]
async fn test_collect_creates_target_branch() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    // 1. Create a temp git repo with an initial commit
    let repo_dir = tempfile::tempdir().unwrap();
    let git_bin = which::which("git").expect("git on PATH");
    let run_git = |args: &[&str]| {
        let out = std::process::Command::new(&git_bin)
            .args(args)
            .current_dir(repo_dir.path())
            .output()
            .expect("git command");
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };

    run_git(&["init"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);
    std::fs::write(repo_dir.path().join("README.md"), "# test\n").unwrap();
    run_git(&["add", "README.md"]);
    run_git(&["commit", "-m", "initial"]);

    // Record the initial HEAD hash as base_ref
    let base_ref = {
        let out = run_git(&["rev-parse", "HEAD"]);
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    // 2. Build manifest pointing to the temp repo
    let mut manifest = test_manifest_with_repo("collect-test", repo_dir.path().to_str().unwrap());
    manifest.job.base_ref = base_ref.clone();
    manifest.merge.target = "smelt/result".to_string();

    // 3. Provision container
    let container = provider.provision(&manifest).await.expect("provision");

    // Install git in alpine (not present by default)
    let install = provider
        .exec(
            &container,
            &[
                "sh".to_string(),
                "-c".to_string(),
                "apk add --no-cache git".to_string(),
            ],
        )
        .await
        .expect("install git");
    assert_eq!(install.exit_code, 0, "git install should succeed");

    // 4. Write a mock script that creates a file and commits it in /workspace
    let mock_script = r#"#!/bin/sh
set -e
cd /workspace
git config user.email "assay@smelt.dev"
git config user.name "Assay Mock"
echo "generated by assay" > result.txt
git add result.txt
git commit -m "assay: add result"
"#;
    let write_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /tmp/mock-assay.sh && chmod +x /tmp/mock-assay.sh",
            base64::engine::general_purpose::STANDARD.encode(mock_script.as_bytes())
        ),
    ];
    let wh = provider
        .exec(&container, &write_cmd)
        .await
        .expect("write mock script");
    assert_eq!(wh.exit_code, 0, "writing mock script should succeed");

    // 5. Execute the mock script (creates a commit in the bind-mounted repo)
    let run_cmd = vec!["sh".to_string(), "/tmp/mock-assay.sh".to_string()];
    let handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec mock assay");
    assert_eq!(
        handle.exit_code, 0,
        "mock assay should exit 0, stdout: {}, stderr: {}",
        handle.stdout, handle.stderr
    );

    // 6. Call ResultCollector::collect() on the host repo
    let git_cli = smelt_core::GitCli::new(git_bin.clone(), repo_dir.path().to_path_buf());
    let collector = smelt_core::ResultCollector::new(git_cli, repo_dir.path().to_path_buf());
    let result = collector
        .collect(&base_ref, "smelt/result")
        .await
        .expect("collect should succeed");

    // 7. Teardown container
    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;

    // 8. Assertions
    // a) no_changes should be false
    assert!(
        !result.no_changes,
        "should have detected changes from mock assay"
    );

    // b) Target branch "smelt/result" exists on the host repo
    let branch_check = std::process::Command::new(&git_bin)
        .args(["rev-parse", "--verify", "smelt/result"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    assert!(
        branch_check.status.success(),
        "target branch 'smelt/result' should exist on host"
    );

    // c) rev_list_count >= 1
    assert!(
        result.commit_count >= 1,
        "should have at least 1 commit, got: {}",
        result.commit_count
    );

    // d) diff_name_only contains the file created by mock script
    assert!(
        result.files_changed.contains(&"result.txt".to_string()),
        "files_changed should contain 'result.txt', got: {:?}",
        result.files_changed
    );
}

/// Test the full end-to-end pipeline: provision → install git → write mock assay binary →
/// write smelt manifest → exec assay via `build_run_command` → collect result branch → teardown.
///
/// The mock assay binary is placed at `/usr/local/bin/assay` so it is on PATH, which is
/// exactly how `AssayInvoker::build_run_command()` constructs the command. After exec,
/// `ResultCollector::collect()` runs on the host repo to create the target branch.
#[tokio::test]
async fn test_full_e2e_pipeline() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    // 1. Create a temp git repo with an initial commit
    let repo_dir = tempfile::tempdir().unwrap();
    let git_bin = which::which("git").expect("git on PATH");
    let run_git = |args: &[&str]| {
        let out = std::process::Command::new(&git_bin)
            .args(args)
            .current_dir(repo_dir.path())
            .output()
            .expect("git command");
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };

    run_git(&["init"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);
    std::fs::write(repo_dir.path().join("README.md"), "# test\n").unwrap();
    run_git(&["add", "README.md"]);
    run_git(&["commit", "-m", "initial"]);

    let base_ref = {
        let out = run_git(&["rev-parse", "HEAD"]);
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    // 2. Build manifest pointing at the temp repo
    let mut manifest = test_manifest_with_repo("e2e-pipeline", repo_dir.path().to_str().unwrap());
    manifest.job.base_ref = base_ref.clone();
    manifest.merge.target = "smelt/e2e-result".to_string();

    // 3. Provision container
    let container = provider.provision(&manifest).await.expect("provision");

    // 4. Install git in alpine (not present by default)
    let install = provider
        .exec(
            &container,
            &[
                "sh".to_string(),
                "-c".to_string(),
                "apk add --no-cache git".to_string(),
            ],
        )
        .await
        .expect("install git");
    assert_eq!(
        install.exit_code, 0,
        "git install should succeed: {}",
        install.stderr
    );

    // 5. Write mock assay binary to /usr/local/bin/assay (on PATH)
    //    The script reads the manifest, creates a file + commit in /workspace, and exits 0.
    //    This matches exactly how AssayInvoker::build_run_command() invokes it.
    let mock_assay_script = r#"#!/bin/sh
set -e
cd /workspace
git config user.email "assay@smelt.dev"
git config user.name "Assay Mock"
echo "generated by assay" > assay-output.txt
git add assay-output.txt
git commit -m "assay: generated output"
exit 0
"#;
    let encoded = base64::engine::general_purpose::STANDARD.encode(mock_assay_script.as_bytes());
    let write_assay_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /usr/local/bin/assay && chmod +x /usr/local/bin/assay",
            encoded
        ),
    ];
    let write_handle = provider
        .exec(&container, &write_assay_cmd)
        .await
        .expect("write assay binary");
    assert_eq!(
        write_handle.exit_code, 0,
        "writing mock assay should succeed: {}",
        write_handle.stderr
    );

    // 6. Write smelt manifest into container via AssayInvoker
    let toml = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml)
        .await
        .expect("write manifest to container");

    // 7. Exec assay via build_run_command — ["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "<max>"]
    let cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let handle = provider.exec(&container, &cmd).await.expect("exec assay");
    assert_eq!(
        handle.exit_code, 0,
        "assay run should exit 0: stdout={} stderr={}",
        handle.stdout, handle.stderr
    );

    // 8. Collect result onto target branch
    let git_cli = smelt_core::GitCli::new(git_bin.clone(), repo_dir.path().to_path_buf());
    let collector = smelt_core::ResultCollector::new(git_cli, repo_dir.path().to_path_buf());
    let result = collector
        .collect(&base_ref, "smelt/e2e-result")
        .await
        .expect("collect should succeed");

    // 9. Teardown container
    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;

    // 10. Assertions
    assert!(!result.no_changes, "should have changes from mock assay");
    assert!(
        result.commit_count >= 1,
        "should have at least 1 commit, got {}",
        result.commit_count
    );
    assert!(
        result
            .files_changed
            .contains(&"assay-output.txt".to_string()),
        "expected assay-output.txt in files_changed, got: {:?}",
        result.files_changed
    );

    // Verify target branch exists on host
    let branch_check = std::process::Command::new(&git_bin)
        .args(["rev-parse", "--verify", "smelt/e2e-result"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    assert!(
        branch_check.status.success(),
        "target branch 'smelt/e2e-result' should exist on host"
    );
}

// ── Timeout & cancellation tests ───────────────────────────────────────

// ── Timeout & cancellation tests ───────────────────────────────────────
//
// These tests exercise the `tokio::select!` pattern at the provider level:
// provision a container, start a long-running exec, then race it against a
// timeout or cancellation signal. Verifies that teardown always runs and
// the container is cleaned up.

/// Test that a timeout racing against a long exec triggers teardown.
///
/// Provisions a container, starts `sleep 120` inside it, races against a
/// 2-second timeout, then tears down the container.
#[tokio::test]
async fn test_timeout_triggers_teardown() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("timeout-teardown");

    let container = provider.provision(&manifest).await.expect("provision");
    let container_id = container.as_str().to_string();

    let timeout_duration = std::time::Duration::from_secs(2);
    let cmd = vec!["sh".to_string(), "-c".to_string(), "sleep 120".to_string()];

    let start = Instant::now();

    let exec_future = provider.exec(&container, &cmd);
    let outcome = tokio::select! {
        result = exec_future => {
            panic!("exec should not complete before timeout, got: {:?}", result.map(|h| h.exit_code));
        }
        _ = tokio::time::sleep(timeout_duration) => {
            "timeout"
        }
    };

    let elapsed = start.elapsed();
    assert_eq!(outcome, "timeout");
    assert!(
        elapsed.as_secs() < 10,
        "should complete promptly at ~2s, took {}s",
        elapsed.as_secs()
    );

    // Teardown must succeed after timeout
    provider
        .teardown(&container)
        .await
        .expect("teardown after timeout");
    assert_container_removed(&provider, &container_id).await;
}

/// Test that cancellation racing against a long exec triggers teardown.
///
/// Provisions a container, starts `sleep 120` inside it, fires a cancel
/// signal after 2 seconds, then tears down the container.
#[tokio::test]
async fn test_cancellation_triggers_teardown() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("cancel-teardown");

    let container = provider.provision(&manifest).await.expect("provision");
    let container_id = container.as_str().to_string();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Fire cancel after 2 seconds
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let _ = tx.send(());
    });

    let cmd = vec!["sh".to_string(), "-c".to_string(), "sleep 120".to_string()];

    let start = Instant::now();

    let exec_future = provider.exec(&container, &cmd);
    let outcome = tokio::select! {
        result = exec_future => {
            panic!("exec should not complete before cancel, got: {:?}", result.map(|h| h.exit_code));
        }
        _ = rx => {
            "cancelled"
        }
    };

    let elapsed = start.elapsed();
    assert_eq!(outcome, "cancelled");
    assert!(
        elapsed.as_secs() < 10,
        "should complete promptly at ~2s, took {}s",
        elapsed.as_secs()
    );

    // Teardown must succeed after cancellation
    provider
        .teardown(&container)
        .await
        .expect("teardown after cancel");
    assert_container_removed(&provider, &container_id).await;
}

/// Test multi-session manifest round-trip: provision → write 2-session manifest with depends_on →
/// read manifest back from container → verify both session names and depends_on → exec assay → teardown.
///
/// Confirms that `AssayInvoker::build_run_manifest_toml` serializes both sessions and the
/// `depends_on` relationship correctly, and that the mock assay sees the manifest.
#[tokio::test]
async fn test_multi_session_e2e() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    // Build manifest with two sessions, second depending on first
    let mut manifest = test_manifest("multi-session-e2e");
    manifest.session = vec![
        SessionDef {
            name: "session-one".to_string(),
            spec: "spec-one".to_string(),
            harness: "echo one".to_string(),
            timeout: 60,
            depends_on: vec![],
        },
        SessionDef {
            name: "session-two".to_string(),
            spec: "spec-two".to_string(),
            harness: "echo two".to_string(),
            timeout: 60,
            depends_on: vec!["session-one".to_string()],
        },
    ];

    // Provision container
    let container = provider.provision(&manifest).await.expect("provision");

    // Write mock assay binary to /usr/local/bin/assay (on PATH):
    // verifies /tmp/smelt-manifest.toml exists, then exits 0.
    let mock_assay_script = r#"#!/bin/sh
set -e
test -f /tmp/smelt-manifest.toml || { echo "manifest missing"; exit 1; }
exit 0
"#;
    let encoded = base64::engine::general_purpose::STANDARD.encode(mock_assay_script.as_bytes());
    let write_assay_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /usr/local/bin/assay && chmod +x /usr/local/bin/assay",
            encoded
        ),
    ];
    let wh = provider
        .exec(&container, &write_assay_cmd)
        .await
        .expect("write assay binary");
    assert_eq!(
        wh.exit_code, 0,
        "writing mock assay should succeed: {}",
        wh.stderr
    );

    // Write smelt manifest into container via AssayInvoker
    let toml = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml)
        .await
        .expect("write manifest to container");

    // Read manifest back from container and verify contents
    let cat_cmd = vec!["cat".to_string(), "/tmp/smelt-manifest.toml".to_string()];
    let cat_handle = provider
        .exec(&container, &cat_cmd)
        .await
        .expect("cat manifest");
    assert_eq!(
        cat_handle.exit_code, 0,
        "cat manifest should succeed, stderr: {}",
        cat_handle.stderr
    );
    let manifest_toml = &cat_handle.stdout;

    // Both session names must be present
    assert!(
        manifest_toml.contains("session-one"),
        "manifest should contain 'session-one', got:\n{manifest_toml}"
    );
    assert!(
        manifest_toml.contains("session-two"),
        "manifest should contain 'session-two', got:\n{manifest_toml}"
    );
    // The depends_on relationship must be serialized
    assert!(
        manifest_toml.contains("depends_on"),
        "manifest should contain 'depends_on', got:\n{manifest_toml}"
    );
    assert!(
        manifest_toml.contains("\"session-one\""),
        "manifest should contain 'session-one' in depends_on context, got:\n{manifest_toml}"
    );

    // Execute assay via build_run_command — must exit 0
    let run_cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec assay");
    assert_eq!(
        handle.exit_code, 0,
        "assay run should exit 0: stdout={} stderr={}",
        handle.stdout, handle.stderr
    );

    // Teardown and verify container removed
    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;
}

/// Test the error path: when assay exits non-zero, teardown is still called and
/// no orphaned smelt containers remain.
///
/// This closes the error branch: assay non-zero exit → teardown → no orphans.
#[tokio::test]
async fn test_e2e_assay_failure_no_orphans() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    // Pre-clean any stale smelt containers to avoid false positives
    let stale = std::process::Command::new("docker")
        .args(["ps", "-aq", "--filter", "label=smelt.job"])
        .output()
        .expect("docker ps");
    for id in String::from_utf8_lossy(&stale.stdout).split_whitespace() {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", id])
            .output();
    }

    let manifest = test_manifest("failure-no-orphans");

    // Provision container
    let container = provider.provision(&manifest).await.expect("provision");
    let container_id = container.as_str().to_string();

    // Write failing mock assay to /usr/local/bin/assay
    let fail_script = b"#!/bin/sh\nexit 1\n";
    let encoded = base64::engine::general_purpose::STANDARD.encode(fail_script);
    let write_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "echo '{}' | base64 -d > /usr/local/bin/assay && chmod +x /usr/local/bin/assay",
            encoded
        ),
    ];
    let wh = provider
        .exec(&container, &write_cmd)
        .await
        .expect("write failing assay");
    assert_eq!(
        wh.exit_code, 0,
        "writing failing mock should succeed: {}",
        wh.stderr
    );

    // Write smelt manifest into container
    let toml = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml)
        .await
        .expect("write manifest to container");

    // Exec assay via build_run_command — must exit 1
    let run_cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec failing assay");
    assert_eq!(
        handle.exit_code, 1,
        "failing assay should exit 1, got exit_code={}",
        handle.exit_code
    );

    // Teardown must succeed even after assay failure
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed after assay failure");

    // Container must be removed
    assert_container_removed(&provider, &container_id).await;

    // No containers for THIS job should remain (filter by job-specific label value
    // to avoid false positives from other concurrent tests using the same label key).
    let ps = std::process::Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            "label=smelt.job=failure-no-orphans",
        ])
        .output()
        .expect("docker ps should work");
    let remaining = String::from_utf8_lossy(&ps.stdout);
    assert!(
        remaining.trim().is_empty(),
        "no failure-no-orphans containers should remain after teardown, got:\n{remaining}"
    );
}

/// Test that double teardown is safe (existing 404 tolerance in DockerProvider).
#[tokio::test]
async fn test_double_teardown_safe() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("double-teardown");

    let container = provider.provision(&manifest).await.expect("provision");
    let container_id = container.as_str().to_string();

    provider.teardown(&container).await.expect("first teardown");
    // Second teardown should not error (404 tolerance)
    provider
        .teardown(&container)
        .await
        .expect("second teardown should be safe");
    assert_container_removed(&provider, &container_id).await;
}

// ── Linux assay binary builder tests ──────────────────────────────────

/// Integration test: run the real Linux assay binary inside a Docker container and
/// assert it progresses past the manifest/spec parse phase without TOML schema errors.
///
/// This test proves that:
/// - Smelt's generated TOML files pass assay's `deny_unknown_fields` validation
/// - The `[[sessions]]` key is correct (not `[[session]]`)
/// - Spec files written to `/workspace/.assay/specs/<name>.toml` are found by assay
/// - The `.assay/config.toml` created by Phase 5.5 satisfies assay's project root detection
///
/// Assay will fail after parsing (no Claude API key / worktree setup), but the test only
/// asserts on the parse-phase outcome. Exit code is expected to be non-zero.
///
/// Skips gracefully when Docker or the Linux assay binary are unavailable.
#[tokio::test]
async fn test_real_assay_manifest_parsing() {
    // Step 1: Get Docker provider (skip if unavailable)
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };

    // Step 2: Build (or locate cached) Linux assay binary (skip if unavailable)
    let Some(binary_path) = build_linux_assay_binary() else {
        eprintln!("Skipping test_real_assay_manifest_parsing — assay Linux binary unavailable");
        return;
    };

    // Build a two-session manifest — alpha (no deps) and beta (depends on alpha)
    let repo_dir = tempfile::tempdir().unwrap();
    let mut manifest =
        test_manifest_with_repo("real-assay-parse", repo_dir.path().to_str().unwrap());
    manifest.job.base_ref = "main".to_string();
    manifest.session = vec![
        SessionDef {
            name: "parse-test-alpha".to_string(),
            spec: "First parse test session".to_string(),
            harness: "echo ok".to_string(),
            timeout: 60,
            depends_on: vec![],
        },
        SessionDef {
            name: "parse-test-beta".to_string(),
            spec: "Second parse test session".to_string(),
            harness: "echo ok".to_string(),
            timeout: 60,
            depends_on: vec!["parse-test-alpha".to_string()],
        },
    ];

    // Step 3: Provision container
    let container = provider
        .provision(&manifest)
        .await
        .expect("provision should succeed");

    // Step 4: Inject real Linux assay binary into container and make it executable
    let injected =
        inject_binary_to_container(container.as_str(), &binary_path, "/usr/local/bin/assay");
    if !injected {
        provider.teardown(&container).await.ok();
        panic!("inject_binary_to_container failed — cannot proceed with test");
    }

    let chmod_handle = provider
        .exec(
            &container,
            &[
                "chmod".to_string(),
                "+x".to_string(),
                "/usr/local/bin/assay".to_string(),
            ],
        )
        .await
        .expect("chmod exec should not error");
    assert_eq!(
        chmod_handle.exit_code, 0,
        "chmod +x /usr/local/bin/assay should succeed, stderr: {}",
        chmod_handle.stderr
    );

    // Step 5: Phase 5.5 — mirror the exact sequence from execute_run()

    // 5a: Write assay config into container (idempotent mkdir + config.toml)
    let config_cmd = smelt_core::AssayInvoker::build_write_assay_config_command(&manifest.job.name);
    let config_handle = provider
        .exec(&container, &config_cmd)
        .await
        .expect("exec assay config write should not error");
    assert_eq!(
        config_handle.exit_code, 0,
        "assay config write should exit 0, stderr: {}",
        config_handle.stderr
    );

    // 5b: Ensure specs directory exists
    let specs_dir_cmd = smelt_core::AssayInvoker::build_ensure_specs_dir_command();
    let specs_dir_handle = provider
        .exec(&container, &specs_dir_cmd)
        .await
        .expect("exec ensure specs dir should not error");
    assert_eq!(
        specs_dir_handle.exit_code, 0,
        "ensure specs dir should exit 0, stderr: {}",
        specs_dir_handle.stderr
    );

    // 5c: Write per-session spec TOML files
    for s in manifest.session.iter() {
        let spec_name = smelt_core::AssayInvoker::sanitize_session_name(&s.name);
        let spec_toml = smelt_core::AssayInvoker::build_spec_toml(s);
        let spec_handle = smelt_core::AssayInvoker::write_spec_file_to_container(
            &provider, &container, &spec_name, &spec_toml,
        )
        .await
        .expect("write_spec_file_to_container should not error");
        assert_eq!(
            spec_handle.exit_code, 0,
            "spec file write for '{spec_name}' should exit 0, stderr: {}",
            spec_handle.stderr
        );
    }

    // 5d: Write assay run manifest into container
    let manifest_toml = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
    smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &manifest_toml)
        .await
        .expect("write_manifest_to_container should succeed");

    // Step 6: Exec `assay run` and capture output
    let run_cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let assay_handle = provider
        .exec(&container, &run_cmd)
        .await
        .expect("exec assay run should not error at the transport level");

    // Print unconditionally — visible with --nocapture, essential for diagnosing parse failures
    eprintln!("assay stdout: {}", assay_handle.stdout);
    eprintln!("assay stderr: {}", assay_handle.stderr);

    // Teardown before asserting (ensures container is cleaned up on panic too,
    // since Rust tests run assertion panics after this point)
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");
    assert_container_removed(&provider, container.as_str()).await;

    // Step 7: Assert parse phase succeeded
    // Primary signal: assay progressed past manifest parse and emitted "Manifest loaded:"
    assert!(
        assay_handle.stderr.contains("Manifest loaded:"),
        "assay should have progressed past parse phase (expected 'Manifest loaded:' in stderr);\
        \nassay stdout: {}\nassay stderr: {}",
        assay_handle.stdout,
        assay_handle.stderr
    );

    // Negative assertions — these would indicate TOML schema or setup failures
    assert!(
        !assay_handle.stderr.contains("No Assay project found"),
        "Phase 5.5 config write should have succeeded (got 'No Assay project found');\
        \nassay stderr: {}",
        assay_handle.stderr
    );
    assert!(
        !assay_handle.stderr.contains("unknown field"),
        "No deny_unknown_fields violations expected;\
        \nassay stderr: {}",
        assay_handle.stderr
    );
    assert!(
        !assay_handle.stderr.contains("ManifestParse"),
        "No manifest TOML parse errors expected;\
        \nassay stderr: {}",
        assay_handle.stderr
    );
    assert!(
        !assay_handle.stderr.contains("ManifestValidation"),
        "No manifest validation errors expected;\
        \nassay stderr: {}",
        assay_handle.stderr
    );

    // NOTE: exit_code is intentionally NOT asserted as 0 — assay will fail after
    // parse phase without a Claude API key / worktree setup.
}

/// Test that `exec_streaming()` delivers output chunks in order and that `ExecHandle`
/// is still populated with the full buffered output.
///
/// Uses `printf 'a\nb\nc\n'` (available in alpine:3) as the command.
/// Chunks are accumulated via `Arc<Mutex<Vec<String>>>` to satisfy `Send + 'static`.
/// Asserts:
/// - At least one chunk was delivered
/// - Joined chunks equal `"a\nb\nc\n"` (order preserved)
/// - `handle.stdout` contains `"a"` (ExecHandle is populated)
#[tokio::test]
async fn test_exec_streaming_delivers_chunks_in_order() {
    let Some(provider) = docker_provider_or_skip() else {
        return;
    };
    let manifest = test_manifest("exec-streaming-order");

    let container = provider.provision(&manifest).await.expect("provision");

    let chunks: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let chunks_cb = Arc::clone(&chunks);

    let cmd = vec!["printf".to_string(), "a\\nb\\nc\\n".to_string()];
    let handle = provider
        .exec_streaming(&container, &cmd, move |chunk| {
            chunks_cb.lock().unwrap().push(chunk.to_string());
        })
        .await
        .expect("exec_streaming should succeed");

    // Teardown before asserting (D039 teardown-before-assert pattern)
    provider.teardown(&container).await.expect("teardown");
    assert_container_removed(&provider, container.as_str()).await;

    let collected = chunks.lock().unwrap().clone();
    let joined = collected.join("");

    // Print for diagnosability without --nocapture
    for (i, c) in collected.iter().enumerate() {
        eprintln!("chunk[{i}] = {c:?}");
    }
    eprintln!("handle.stdout = {:?}", handle.stdout);

    assert!(
        !collected.is_empty(),
        "streaming callback should have been invoked at least once"
    );
    assert_eq!(
        joined, "a\nb\nc\n",
        "joined chunks should equal 'a\\nb\\nc\\n', got: {joined:?}"
    );
    assert!(
        handle.stdout.contains("a"),
        "ExecHandle.stdout should contain 'a', got: {:?}",
        handle.stdout
    );
}

/// Smoke test for `build_linux_assay_binary()`.
///
/// Calls the builder and checks that:
/// - The function returns `Some(path)` when assay source + Docker are available, OR
/// - Returns `None` and the test skips gracefully when either is unavailable.
///
/// On success, verifies the cached binary exists, has non-zero size, and the
/// path ends with `assay-linux-aarch64`.
#[tokio::test]
async fn test_build_linux_assay_binary_caches() {
    let result = build_linux_assay_binary();

    match result {
        None => {
            // Assay source or Docker unavailable — skip gracefully
            eprintln!(
                "test_build_linux_assay_binary_caches: skipped (assay source or Docker not available)"
            );
            return;
        }
        Some(path) => {
            assert!(
                path.exists(),
                "cached binary should exist at {}",
                path.display()
            );
            let meta = path.metadata().expect("should be able to read metadata");
            assert!(
                meta.len() > 0,
                "cached binary should have non-zero size, got {} bytes",
                meta.len()
            );
            assert!(
                path.ends_with("assay-linux-aarch64"),
                "cache path should end with 'assay-linux-aarch64', got: {}",
                path.display()
            );
            eprintln!(
                "test_build_linux_assay_binary_caches: PASS — binary at {} ({} bytes)",
                path.display(),
                meta.len()
            );

            // Second call must return the same cached path without rebuilding (fast path)
            let second = build_linux_assay_binary();
            assert!(
                second.is_some(),
                "second call should also return Some (cache hit)"
            );
            assert_eq!(
                second.unwrap(),
                path,
                "cache hit should return the same path"
            );
        }
    }
}
