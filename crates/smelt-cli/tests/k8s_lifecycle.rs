//! Integration tests for the Kubernetes Pod container lifecycle.
//!
//! These tests require a reachable Kubernetes cluster (e.g. kind) and the
//! `SMELT_K8S_TEST` environment variable to be set. They exercise the full
//! `KubernetesProvider` lifecycle: provision → exec → exec_streaming → teardown.
//!
//! Set `SMELT_K8S_TEST=1` to opt in. Tests skip gracefully when the variable
//! is absent or the cluster is unreachable.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};

use smelt_core::GitOps as _;
use smelt_core::k8s::KubernetesProvider;
use smelt_core::manifest::{
    CredentialConfig, Environment, JobManifest, JobMeta, KubernetesConfig, MergeConfig, SessionDef,
};
use smelt_core::provider::RuntimeProvider;
use smelt_core::{GitCli, ResultCollector};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal [`JobManifest`] for Kubernetes lifecycle tests.
fn k8s_manifest() -> JobManifest {
    JobManifest {
        job: JobMeta {
            name: "smelt-test".to_string(),
            repo: "git@github.com:example/smelt-test.git".to_string(),
            base_ref: "main".to_string(),
        },
        environment: Environment {
            runtime: "kubernetes".to_string(),
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
        kubernetes: Some(KubernetesConfig {
            namespace: "smelt".to_string(),
            context: None,
            ssh_key_env: "SMELT_TEST_SSH_KEY".to_string(),
            cpu_request: None,
            memory_request: None,
            cpu_limit: None,
            memory_limit: None,
        }),
        services: vec![],
    }
}

/// Try to connect to a Kubernetes cluster and return a `KubernetesProvider`.
///
/// Returns `None` (skipping) if:
/// - `SMELT_K8S_TEST` is not set in the environment, or
/// - `KubernetesProvider::new()` fails (cluster unreachable, kubeconfig missing, etc.).
///
/// Returns `Some(provider)` when both checks pass.
async fn k8s_provider_or_skip() -> Option<KubernetesProvider> {
    if std::env::var("SMELT_K8S_TEST").is_err() {
        eprintln!("Skipping: SMELT_K8S_TEST not set");
        return None;
    }

    match KubernetesProvider::new(&k8s_manifest()).await {
        Ok(provider) => Some(provider),
        Err(e) => {
            eprintln!("Skipping: cluster unavailable: {e}");
            None
        }
    }
}

/// Delete leftover pod and secret from a previous test run, tolerating absence.
///
/// Runs `kubectl delete` as a blocking subprocess. Errors are silently ignored
/// so that tests always start with a clean slate regardless of prior state.
fn pre_clean_k8s(namespace: &str, job_name: &str) {
    let pod_name = format!("smelt-{job_name}");
    let secret_name = format!("smelt-ssh-{job_name}");

    let _ = Command::new("kubectl")
        .args([
            "delete",
            "pod",
            &pod_name,
            "--ignore-not-found",
            "-n",
            namespace,
        ])
        .output();

    let _ = Command::new("kubectl")
        .args([
            "delete",
            "secret",
            &secret_name,
            "--ignore-not-found",
            "-n",
            namespace,
        ])
        .output();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Test 1: Full lifecycle — provision (init clone + agent Running), exec, teardown.
#[tokio::test]
#[ignore]
async fn test_k8s_provision_exec_teardown() {
    let Some(provider) = k8s_provider_or_skip().await else {
        return;
    };

    pre_clean_k8s("smelt", "smelt-test");

    // Provision
    let container = provider
        .provision(&k8s_manifest())
        .await
        .expect("provision should succeed");

    assert!(
        container.as_str().starts_with("smelt/"),
        "ContainerId should start with 'smelt/', got: {}",
        container.as_str()
    );

    // Exec
    let handle = provider
        .exec(
            &container,
            &["sh", "-c", "echo hello"].map(str::to_string).to_vec(),
        )
        .await
        .expect("exec should succeed");

    assert_eq!(
        handle.exit_code, 0,
        "exit_code should be 0, got: {}",
        handle.exit_code
    );
    assert!(
        handle.stdout.contains("hello"),
        "stdout should contain 'hello', got: {:?}",
        handle.stdout
    );

    // Teardown
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");

    // Verify pod is gone from cluster
    let pod_name = container.as_str().split('/').nth(1).unwrap_or("");
    let output = Command::new("kubectl")
        .args(["get", "pod", pod_name, "-n", "smelt"])
        .output()
        .expect("kubectl should run");

    assert!(
        !output.status.success(),
        "kubectl get pod should fail (pod should be gone), but it succeeded. pod: {}",
        pod_name
    );
}

/// Test 2: exec_streaming — verify the output callback fires with command output.
#[tokio::test]
#[ignore]
async fn test_k8s_exec_streaming_callback() {
    let Some(provider) = k8s_provider_or_skip().await else {
        return;
    };

    pre_clean_k8s("smelt", "smelt-test");

    // Provision
    let container = provider
        .provision(&k8s_manifest())
        .await
        .expect("provision should succeed");

    // exec_streaming with accumulator
    let chunks = Arc::new(Mutex::new(Vec::<String>::new()));
    let chunks_clone = Arc::clone(&chunks);

    let handle = provider
        .exec_streaming(
            &container,
            &["echo", "streaming-hello"].map(str::to_string).to_vec(),
            move |s| {
                chunks_clone.lock().unwrap().push(s.to_string());
            },
        )
        .await
        .expect("exec_streaming should succeed");

    assert_eq!(
        handle.exit_code, 0,
        "exit_code should be 0, got: {}",
        handle.exit_code
    );

    let all_chunks = chunks.lock().unwrap();
    assert!(
        !all_chunks.is_empty(),
        "callback should have received at least one chunk"
    );

    let joined = all_chunks.join("");
    assert!(
        joined.contains("streaming-hello"),
        "joined chunks should contain 'streaming-hello', got: {:?}",
        joined
    );

    // Teardown
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");
}

/// Test 3: SSH key file permissions — confirm the mounted key is mode 0o400.
#[tokio::test]
#[ignore]
async fn test_k8s_ssh_file_permissions() {
    let Some(provider) = k8s_provider_or_skip().await else {
        return;
    };

    pre_clean_k8s("smelt", "smelt-test");

    // Provision
    let container = provider
        .provision(&k8s_manifest())
        .await
        .expect("provision should succeed");

    // stat the SSH key to check permissions
    let handle = provider
        .exec(
            &container,
            &["stat", "/root/.ssh/id_rsa"].map(str::to_string).to_vec(),
        )
        .await
        .expect("exec stat should succeed");

    assert_eq!(
        handle.exit_code, 0,
        "stat should exit 0, got: {}. stderr: {}",
        handle.exit_code, handle.stderr
    );

    // stat output on Alpine includes: Access: (0400/-r--------) or similar
    assert!(
        handle.stdout.contains("0400"),
        "stat output should contain '0400' (user-read-only), got stdout: {:?}",
        handle.stdout
    );

    // Teardown
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");
}

// ── S03 helpers ───────────────────────────────────────────────────────────────

/// Returns the value of `SMELT_TEST_GIT_REMOTE` if set, or `None`.
fn get_test_git_remote() -> Option<String> {
    std::env::var("SMELT_TEST_GIT_REMOTE").ok()
}

/// Test 4: Readiness confirmed — provision only returns after Pod agent container is Running.
#[tokio::test]
#[ignore]
async fn test_k8s_readiness_confirmed() {
    let Some(provider) = k8s_provider_or_skip().await else {
        return;
    };

    pre_clean_k8s("smelt", "smelt-test");

    // Provision — should only return after init+main containers are ready
    let container = provider
        .provision(&k8s_manifest())
        .await
        .expect("provision should succeed and return only after readiness");

    // Immediately exec — if main container were not Running this would fail
    let handle = provider
        .exec(
            &container,
            &["sh", "-c", "echo ready"].map(str::to_string).to_vec(),
        )
        .await
        .expect("exec immediately after provision should succeed — proves readiness");

    assert_eq!(
        handle.exit_code, 0,
        "exec after provision should exit 0, got: {}",
        handle.exit_code
    );
    assert!(
        handle.stdout.contains("ready"),
        "stdout should contain 'ready', got: {:?}",
        handle.stdout
    );

    // Teardown
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");
}

/// Test 5: Push-from-Pod result collection — the S03 end-to-end path.
///
/// Proves: Pod git-pushes a new branch to `$SMELT_GIT_REMOTE`, host
/// `fetch_ref` brings the branch local, `ResultCollector::collect()` returns
/// `no_changes == false` with at least one commit and "result.txt" changed.
///
/// Requires:
/// - `SMELT_K8S_TEST=1` (kind cluster reachable)
/// - `SMELT_TEST_GIT_REMOTE=<ssh-url>` (reachable SSH remote; SSH key in
///   `SMELT_TEST_SSH_KEY`)
#[tokio::test]
#[ignore]
async fn test_k8s_push_from_pod_result_collection() {
    // Guard 1: k8s cluster available.
    let Some(provider) = k8s_provider_or_skip().await else {
        return;
    };

    // Guard 2: SSH remote available.
    let Some(git_remote) = get_test_git_remote() else {
        eprintln!("SMELT_TEST_GIT_REMOTE not set — skipping push-from-pod test");
        return;
    };

    // ── Setup ──────────────────────────────────────────────────────────────

    pre_clean_k8s("smelt", "s03-test");

    // Build a manifest that points at the test remote so SMELT_GIT_REMOTE is
    // injected correctly by generate_pod_spec().
    let mut manifest = k8s_manifest();
    manifest.job.repo = git_remote.clone();
    manifest.job.name = "s03-test".to_string();

    // Unique branch name to avoid cross-run collisions.
    let push_branch = format!(
        "smelt-s03-push-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    eprintln!("==> [S03] Using push branch: {push_branch}");

    // ── Provision ─────────────────────────────────────────────────────────

    eprintln!("==> [S03] Provisioning Pod ...");
    let container = provider
        .provision(&manifest)
        .await
        .expect("provision should succeed");

    eprintln!("==> [S03] Pod provisioned: {}", container.as_str());

    // ── Exec push script inside Pod ────────────────────────────────────────

    let script = format!(
        r#"
set -e
cd /workspace
git config user.email test@smelt.local
git config user.name smelt
git checkout -b {push_branch}
echo result > result.txt
git add result.txt
git commit -m 'push-from-pod test'
GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git push "$SMELT_GIT_REMOTE" {push_branch}:{push_branch}
"#,
        push_branch = push_branch
    );

    eprintln!("==> [S03] Running push script inside Pod ...");
    let exec_handle = provider
        .exec(
            &container,
            &["/bin/sh".to_string(), "-c".to_string(), script],
        )
        .await
        .expect("exec should succeed");

    if exec_handle.exit_code != 0 {
        eprintln!("==> [S03] Push script failed — tearing down Pod");
        let _ = provider.teardown(&container).await;
        panic!(
            "Push script exited with code {}.\nstdout: {}\nstderr: {}",
            exec_handle.exit_code, exec_handle.stdout, exec_handle.stderr
        );
    }

    eprintln!("==> [S03] Push script succeeded (exit 0)");

    // ── Host: clone remote, fetch branch, collect ──────────────────────────

    let tmp = tempfile::tempdir().expect("create temp dir");
    let tmp_path = tmp.path();

    eprintln!("==> [S03] Cloning remote into temp dir ...");
    let clone_status = Command::new("git")
        .args([
            "clone",
            &git_remote,
            tmp_path.to_str().expect("tempdir path is valid UTF-8"),
        ])
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .status()
        .expect("git clone should run");

    assert!(clone_status.success(), "git clone should succeed");

    // Record the base_ref (HEAD of the clone before the Pod pushed).
    let base_ref_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(tmp_path)
        .output()
        .expect("git rev-parse HEAD should run");
    let base_ref = String::from_utf8_lossy(&base_ref_output.stdout)
        .trim()
        .to_string();
    eprintln!("==> [S03] base_ref = {base_ref}");

    // fetch_ref brings the Pod-pushed branch local.
    let git_binary = which::which("git").expect("git on PATH");
    let git = GitCli::new(git_binary, tmp_path.to_path_buf());

    eprintln!("==> [S03] Fetching pushed branch from remote ...");
    git.fetch_ref("origin", &format!("+{push_branch}:{push_branch}"))
        .await
        .expect("fetch_ref should succeed");

    eprintln!("==> [S03] Collecting results ...");
    let collector = ResultCollector::new(git, tmp_path.to_path_buf());
    let result = collector
        .collect(&base_ref, &push_branch)
        .await
        .expect("collect should succeed");

    eprintln!(
        "==> [S03] Collected: {} commits on branch '{}', no_changes={}",
        result.commit_count, result.branch, result.no_changes
    );

    assert!(
        !result.no_changes,
        "ResultCollector should report changes (no_changes must be false), got: {:?}",
        result
    );
    assert!(
        result.commit_count >= 1,
        "commit_count should be >= 1, got: {}",
        result.commit_count
    );
    assert!(
        result.files_changed.contains(&"result.txt".to_string()),
        "files_changed should contain 'result.txt', got: {:?}",
        result.files_changed
    );

    // ── Teardown ───────────────────────────────────────────────────────────

    eprintln!("==> [S03] Tearing down Pod ...");
    provider
        .teardown(&container)
        .await
        .expect("teardown should succeed");

    eprintln!("==> [S03] Pod torn down");

    // Best-effort: delete the remote test branch to keep the repo clean.
    let _ = Command::new("git")
        .args(["push", "origin", "--delete", &push_branch])
        .current_dir(tmp_path)
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .status();

    eprintln!("==> [S03] Done. push-from-pod result collection test passed.");
}
