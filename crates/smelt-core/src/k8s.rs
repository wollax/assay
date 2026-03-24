//! Kubernetes runtime provider and Pod spec generation for Smelt.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use k8s_openapi::api::core::v1::{
    Container, EmptyDirVolumeSource, EnvVar, KeyToPath, Pod, PodSpec, ResourceRequirements, Secret,
    SecretVolumeSource, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::ByteString;
use kube::api::{Api, AttachParams, DeleteParams, PostParams};
use kube::config::KubeConfigOptions;
use kube::{Client, Config};
use tokio::io::AsyncReadExt;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::error::SmeltError;
use crate::manifest::JobManifest;
use crate::provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};

/// Generate a Kubernetes Pod spec for a Smelt job.
///
/// Returns a fully-typed `Pod` value with:
/// - An init container (`alpine/git`) that clones the job repo via SSH
///   into a shared `/workspace` emptyDir volume.
/// - A main agent container using `manifest.environment.image` with
///   `/workspace` mounted.
/// - An SSH Secret volume with `defaultMode: 256` (0o400) to satisfy
///   the SSH client's key permission requirement.
/// - Resource requests and limits from the `[kubernetes]` config block.
///
/// # Errors
///
/// Returns [`crate::SmeltError::Provider`] if `manifest.kubernetes` is `None`.
pub fn generate_pod_spec(
    manifest: &JobManifest,
    job_name: &str,
    ssh_private_key: &str,
) -> crate::Result<Pod> {
    // Suppress unused-variable warning — ssh_private_key is reserved for S02
    // (creating the Kubernetes Secret object); this function only builds the Pod spec.
    let _ = ssh_private_key;

    let kube_cfg = manifest.kubernetes.as_ref().ok_or_else(|| {
        SmeltError::provider(
            "k8s",
            "generate_pod_spec called without [kubernetes] config block",
        )
    })?;

    // ── SSH Secret volume source ──────────────────────────────────────────────

    let ssh_vol_source = SecretVolumeSource {
        secret_name: Some(format!("smelt-ssh-{job_name}")),
        default_mode: Some(256), // 0o400 — SSH client requires key is not group/world readable
        items: Some(vec![KeyToPath {
            key: "id_rsa".into(),
            path: "id_rsa".into(),
            mode: None,
        }]),
        optional: None,
    };

    // ── Volumes ───────────────────────────────────────────────────────────────

    let workspace_vol = Volume {
        name: "workspace".into(),
        empty_dir: Some(EmptyDirVolumeSource::default()),
        ..Default::default()
    };

    let ssh_vol = Volume {
        name: "ssh-key".into(),
        secret: Some(ssh_vol_source),
        ..Default::default()
    };

    // ── Init container (git clone) ────────────────────────────────────────────

    let init_container = Container {
        name: "git-clone".into(),
        image: Some("alpine/git".into()),
        command: Some(vec![
            "/bin/sh".into(),
            "-c".into(),
            format!("git clone {} /workspace", manifest.job.repo),
        ]),
        volume_mounts: Some(vec![
            VolumeMount {
                name: "workspace".into(),
                mount_path: "/workspace".into(),
                ..Default::default()
            },
            VolumeMount {
                name: "ssh-key".into(),
                mount_path: "/root/.ssh".into(),
                read_only: Some(true),
                ..Default::default()
            },
        ]),
        ..Default::default()
    };

    // ── Resource requirements ─────────────────────────────────────────────────

    let mut requests: BTreeMap<String, Quantity> = BTreeMap::new();
    let mut limits: BTreeMap<String, Quantity> = BTreeMap::new();

    if let Some(ref v) = kube_cfg.cpu_request {
        requests.insert("cpu".into(), Quantity(v.clone()));
    }
    if let Some(ref v) = kube_cfg.memory_request {
        requests.insert("memory".into(), Quantity(v.clone()));
    }
    if let Some(ref v) = kube_cfg.cpu_limit {
        limits.insert("cpu".into(), Quantity(v.clone()));
    }
    if let Some(ref v) = kube_cfg.memory_limit {
        limits.insert("memory".into(), Quantity(v.clone()));
    }

    let resources = ResourceRequirements {
        requests: if requests.is_empty() {
            None
        } else {
            Some(requests)
        },
        limits: if limits.is_empty() { None } else { Some(limits) },
        ..Default::default()
    };

    // ── Main agent container ──────────────────────────────────────────────────

    let main_container = Container {
        name: "smelt-agent".into(),
        image: Some(manifest.environment.image.clone()),
        volume_mounts: Some(vec![VolumeMount {
            name: "workspace".into(),
            mount_path: "/workspace".into(),
            ..Default::default()
        }]),
        resources: Some(resources),
        env: Some(vec![EnvVar {
            name: "SMELT_GIT_REMOTE".into(),
            value: Some(manifest.job.repo.clone()),
            ..Default::default()
        }]),
        ..Default::default()
    };

    // ── Pod ───────────────────────────────────────────────────────────────────

    let pod = Pod {
        metadata: ObjectMeta {
            name: Some(format!("smelt-{job_name}")),
            namespace: Some(kube_cfg.namespace.clone()),
            ..Default::default()
        },
        spec: Some(PodSpec {
            init_containers: Some(vec![init_container]),
            containers: vec![main_container],
            volumes: Some(vec![workspace_vol, ssh_vol]),
            restart_policy: Some("Never".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(pod)
}

// ── PodState ──────────────────────────────────────────────────────────────────

/// Internal state tracked per provisioned pod.
///
/// Stored in `KubernetesProvider::state` keyed by `ContainerId` so that
/// `exec`, `teardown`, and `collect` can locate the right namespace, pod, and
/// SSH Secret without re-reading the manifest.
#[allow(dead_code)] // namespace and pod_name stored for future exec/collect; only secret_name read today
struct PodState {
    namespace: String,
    pod_name: String,
    secret_name: String,
}

// ── KubernetesProvider ────────────────────────────────────────────────────────

/// Kubernetes runtime provider.
///
/// Provisions Smelt jobs as Kubernetes Pods using the ambient kubeconfig
/// or the context specified in the job manifest's `[kubernetes]` block.
///
/// Full implementation is in S02 (T02–T04). All `RuntimeProvider` methods
/// currently `todo!()`.
pub struct KubernetesProvider {
    /// Authenticated kube client — connects to the ambient or specified context.
    client: Client,
    /// Per-pod state keyed by `ContainerId` (the pod name used as the handle).
    state: Arc<Mutex<HashMap<ContainerId, PodState>>>,
}

impl KubernetesProvider {
    /// Build a `KubernetesProvider` from a job manifest.
    ///
    /// If `manifest.kubernetes.context` is set, connects using that kubeconfig
    /// context. Otherwise falls back to the ambient kubeconfig / in-cluster
    /// credentials via `Client::try_default()`.
    ///
    /// # Errors
    ///
    /// Returns [`SmeltError::Provider`] when the kube client cannot be
    /// constructed (missing kubeconfig, invalid context, API server
    /// unreachable, etc.).
    pub async fn new(manifest: &JobManifest) -> crate::Result<Self> {
        let client = match manifest
            .kubernetes
            .as_ref()
            .and_then(|k| k.context.as_ref())
        {
            Some(ctx) => {
                let opts = KubeConfigOptions {
                    context: Some(ctx.clone()),
                    ..Default::default()
                };
                let config = Config::from_kubeconfig(&opts)
                    .await
                    .map_err(|e| SmeltError::provider_with_source("k8s", "failed to build kube client", e))?;
                Client::try_from(config)
                    .map_err(|e| SmeltError::provider_with_source("k8s", "failed to build kube client", e))?
            }
            None => Client::try_default()
                .await
                .map_err(|e| SmeltError::provider_with_source("k8s", "failed to build kube client", e))?,
        };

        Ok(Self {
            client,
            state: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

/// Split a `ContainerId` of the form `"<namespace>/<pod-name>"` into its parts.
///
/// Returns `SmeltError::Provider` if the value does not contain exactly one `/`.
fn parse_container_id(id: &ContainerId) -> crate::Result<(String, String)> {
    let s = id.as_str();
    let parts: Vec<&str> = s.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(SmeltError::provider(
            "k8s",
            format!("invalid ContainerId format: '{s}' — expected '<namespace>/<pod-name>'"),
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

impl RuntimeProvider for KubernetesProvider {
    async fn provision(&self, manifest: &JobManifest) -> crate::Result<ContainerId> {
        let kube_cfg = manifest.kubernetes.as_ref().ok_or_else(|| {
            SmeltError::provider("k8s", "provision called without [kubernetes] config block")
        })?;
        let ns = &kube_cfg.namespace;
        let job_name = &manifest.job.name;
        let pod_name = format!("smelt-{job_name}");
        let secret_name = format!("smelt-ssh-{job_name}");

        // ── Read SSH key from environment ─────────────────────────────────────
        let key_bytes = std::env::var(&kube_cfg.ssh_key_env)
            .map_err(|_| {
                SmeltError::provider(
                    "k8s",
                    format!(
                        "env var '{}' not set or not unicode",
                        kube_cfg.ssh_key_env
                    ),
                )
            })?
            .into_bytes();

        // ── Create SSH Secret ─────────────────────────────────────────────────
        let mut secret_data: BTreeMap<String, ByteString> = BTreeMap::new();
        secret_data.insert("id_rsa".to_string(), ByteString(key_bytes));

        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(secret_name.clone()),
                namespace: Some(ns.clone()),
                ..Default::default()
            },
            data: Some(secret_data),
            ..Default::default()
        };

        let secrets_api: Api<Secret> = Api::namespaced(self.client.clone(), ns);
        secrets_api
            .create(&PostParams::default(), &secret)
            .await
            .map_err(|e| {
                SmeltError::provider_with_source(
                    "k8s",
                    format!("failed to create Secret '{secret_name}'"),
                    e,
                )
            })?;

        // ── Create Pod ────────────────────────────────────────────────────────
        let pod = generate_pod_spec(manifest, job_name, "")?;
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), ns);

        if let Err(e) = pods_api.create(&PostParams::default(), &pod).await {
            // Clean up the Secret before propagating the Pod creation error
            if let Err(del_err) = secrets_api
                .delete(&secret_name, &DeleteParams::default())
                .await
            {
                warn!(
                    secret = %secret_name,
                    namespace = %ns,
                    error = %del_err,
                    "failed to delete Secret during Pod creation rollback"
                );
            }
            return Err(SmeltError::provider_with_source(
                "k8s",
                format!("failed to create Pod '{pod_name}'"),
                e,
            ));
        }

        info!(pod = %pod_name, namespace = %ns, "pod created, polling readiness");

        // ── Readiness poll (60 × 2s = 120s) ──────────────────────────────────
        let mut init_done = false;
        let mut main_running = false;

        for _ in 0..60u32 {
            let fetched = pods_api.get(&pod_name).await.map_err(|e| {
                SmeltError::provider_with_source(
                    "k8s",
                    format!("failed to fetch Pod '{pod_name}' status"),
                    e,
                )
            })?;

            let status = fetched.status.as_ref();

            // Check init container (git-clone)
            if !init_done {
                if let Some(init_statuses) =
                    status.and_then(|s| s.init_container_statuses.as_ref())
                {
                    if let Some(git_clone) =
                        init_statuses.iter().find(|c| c.name == "git-clone")
                    {
                        if let Some(state) = &git_clone.state {
                            if let Some(terminated) = &state.terminated {
                                if terminated.exit_code == 0 {
                                    init_done = true;
                                } else {
                                    return Err(SmeltError::provider(
                                        "k8s",
                                        format!(
                                            "init container failed with exit code {}",
                                            terminated.exit_code
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // Check main container (smelt-agent)
            if !main_running {
                if let Some(container_statuses) =
                    status.and_then(|s| s.container_statuses.as_ref())
                {
                    if let Some(agent) =
                        container_statuses.iter().find(|c| c.name == "smelt-agent")
                    {
                        if let Some(state) = &agent.state {
                            if state.running.is_some() {
                                main_running = true;
                            } else if let Some(waiting) = &state.waiting {
                                if let Some(reason) = &waiting.reason {
                                    if reason == "ImagePullBackOff" || reason == "ErrImagePull" {
                                        return Err(SmeltError::provider(
                                            "k8s",
                                            format!(
                                                "image pull failed for Pod '{pod_name}': {reason}"
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if init_done && main_running {
                break;
            }

            sleep(Duration::from_secs(2)).await;
        }

        if !(init_done && main_running) {
            return Err(SmeltError::provider(
                "k8s",
                format!("pod {pod_name} in namespace {ns} did not become ready within 120s"),
            ));
        }

        info!(pod = %pod_name, namespace = %ns, "pod ready");

        // ── Record state and return ContainerId ───────────────────────────────
        let container_id = ContainerId::new(format!("{ns}/{pod_name}"));
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.insert(
                container_id.clone(),
                PodState {
                    namespace: ns.to_string(),
                    pod_name: pod_name.clone(),
                    secret_name: secret_name.clone(),
                },
            );
        }

        Ok(container_id)
    }

    async fn exec(
        &self,
        container: &ContainerId,
        command: &[String],
    ) -> crate::Result<ExecHandle> {
        let (ns, pod_name) = parse_container_id(container)?;

        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &ns);
        let ap = AttachParams {
            stdout: true,
            stderr: true,
            stdin: false,
            tty: false,
            ..Default::default()
        };

        let mut attached = pods_api
            .exec(&pod_name, command, &ap)
            .await
            .map_err(|e| SmeltError::provider_with_source("k8s", "exec failed", e))?;

        // IMPORTANT: take_status() MUST be called before any stdout/stderr reads.
        let status_fut = attached
            .take_status()
            .expect("status channel must exist for non-tty exec");

        // Drain stdout
        let mut stdout_buf: Vec<u8> = Vec::new();
        if let Some(mut out) = attached.stdout() {
            out.read_to_end(&mut stdout_buf)
                .await
                .map_err(|e| SmeltError::provider_with_source("k8s", "failed to read stdout", e))?;
        }

        // Drain stderr
        let mut stderr_buf: Vec<u8> = Vec::new();
        if let Some(mut err) = attached.stderr() {
            err.read_to_end(&mut stderr_buf)
                .await
                .map_err(|e| SmeltError::provider_with_source("k8s", "failed to read stderr", e))?;
        }

        // Wait for the WebSocket task to complete (AFTER streams are drained)
        attached
            .join()
            .await
            .map_err(|e| SmeltError::provider_with_source("k8s", "exec WebSocket task failed", e))?;

        let status = status_fut.await;
        let exit_code = status.as_ref().and_then(|s| s.code).unwrap_or(-1);

        Ok(ExecHandle {
            container: container.clone(),
            exec_id: format!("{pod_name}-exec"),
            exit_code,
            stdout: String::from_utf8_lossy(&stdout_buf).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_buf).into_owned(),
        })
    }

    async fn exec_streaming<F>(
        &self,
        container: &ContainerId,
        command: &[String],
        mut output_cb: F,
    ) -> crate::Result<ExecHandle>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let (ns, pod_name) = parse_container_id(container)?;

        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &ns);
        let ap = AttachParams {
            stdout: true,
            stderr: true,
            stdin: false,
            tty: false,
            ..Default::default()
        };

        let mut attached = pods_api
            .exec(&pod_name, command, &ap)
            .await
            .map_err(|e| SmeltError::provider_with_source("k8s", "exec_streaming failed", e))?;

        // IMPORTANT: take_status() MUST be called before any stdout/stderr reads.
        let status_fut = attached
            .take_status()
            .expect("status channel must exist for non-tty exec");

        const BUF_SIZE: usize = 4096;
        let mut full_stdout: Vec<u8> = Vec::new();
        let mut full_stderr: Vec<u8> = Vec::new();

        // Sequential stdout then stderr — avoids FnMut shared-access issue with join!
        if let Some(mut out) = attached.stdout() {
            let mut buf = [0u8; BUF_SIZE];
            loop {
                let n = out
                    .read(&mut buf)
                    .await
                    .map_err(|e| SmeltError::provider_with_source("k8s", "failed to read stdout", e))?;
                if n == 0 {
                    break;
                }
                let chunk = std::str::from_utf8(&buf[..n]).unwrap_or("");
                output_cb(chunk);
                full_stdout.extend_from_slice(&buf[..n]);
            }
        }

        if let Some(mut err) = attached.stderr() {
            let mut buf = [0u8; BUF_SIZE];
            loop {
                let n = err
                    .read(&mut buf)
                    .await
                    .map_err(|e| SmeltError::provider_with_source("k8s", "failed to read stderr", e))?;
                if n == 0 {
                    break;
                }
                let chunk = std::str::from_utf8(&buf[..n]).unwrap_or("");
                output_cb(chunk);
                full_stderr.extend_from_slice(&buf[..n]);
            }
        }

        // Wait for WebSocket task to complete (AFTER streams are drained)
        attached
            .join()
            .await
            .map_err(|e| SmeltError::provider_with_source("k8s", "exec_streaming WebSocket task failed", e))?;

        let status = status_fut.await;
        let exit_code = status.as_ref().and_then(|s| s.code).unwrap_or(-1);

        Ok(ExecHandle {
            container: container.clone(),
            exec_id: format!("{pod_name}-exec"),
            exit_code,
            stdout: String::from_utf8_lossy(&full_stdout).into_owned(),
            stderr: String::from_utf8_lossy(&full_stderr).into_owned(),
        })
    }

    async fn collect(
        &self,
        _container: &ContainerId,
        _manifest: &JobManifest,
    ) -> crate::Result<CollectResult> {
        // S02 no-op: artifact collection is deferred to a later milestone.
        Ok(CollectResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            artifacts: vec![],
        })
    }

    async fn teardown(&self, container: &ContainerId) -> crate::Result<()> {
        let (ns, pod_name) = parse_container_id(container)?;

        // Derive secret name from pod name: "smelt-<job_name>" → "smelt-ssh-<job_name>"
        let secret_name = {
            let guard = self.state.lock().expect("state mutex poisoned");
            guard
                .get(container)
                .map(|s| s.secret_name.clone())
                .unwrap_or_else(|| {
                    // Fallback: derive from pod name convention
                    format!(
                        "smelt-ssh-{}",
                        pod_name.trim_start_matches("smelt-")
                    )
                })
        };

        // ── Delete Pod ────────────────────────────────────────────────────────
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &ns);
        match pods_api.delete(&pod_name, &DeleteParams::default()).await {
            Ok(_) => {}
            Err(kube::Error::Api(s)) if s.is_not_found() => {
                // Already gone — idempotent
            }
            Err(e) => {
                warn!(pod = %pod_name, namespace = %ns, error = %e, "pod delete non-fatal");
            }
        }

        // ── Delete SSH Secret ─────────────────────────────────────────────────
        let secrets_api: Api<Secret> = Api::namespaced(self.client.clone(), &ns);
        match secrets_api
            .delete(&secret_name, &DeleteParams::default())
            .await
        {
            Ok(_) => {}
            Err(kube::Error::Api(s)) if s.is_not_found() => {
                // Already gone — idempotent
            }
            Err(e) => {
                warn!(secret = %secret_name, namespace = %ns, error = %e, "secret delete non-fatal");
            }
        }

        // ── Remove from state ─────────────────────────────────────────────────
        {
            let mut guard = self.state.lock().expect("state mutex poisoned");
            guard.remove(container);
        }

        info!(pod = %pod_name, namespace = %ns, "teardown complete");
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Minimal kubernetes manifest TOML for testing.
    const KUBERNETES_MANIFEST_TOML: &str = r#"
[job]
name = "test-job"
repo = "git@github.com:example/repo.git"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "ghcr.io/example/assay-agent:latest"

[credentials]
provider = "anthropic"
model = "claude-opus-4-5"

[credentials.env]
ANTHROPIC_API_KEY = "ANTHROPIC_API_KEY"

[[session]]
name = "code-review"
spec = "review"
harness = "default"
timeout = 1800

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt"
ssh_key_env = "SMELT_SSH_KEY"
"#;

    /// Manifest with resource limits for the resource_limits snapshot test.
    const KUBERNETES_MANIFEST_WITH_RESOURCES_TOML: &str = r#"
[job]
name = "test-job"
repo = "git@github.com:example/repo.git"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "ghcr.io/example/assay-agent:latest"

[credentials]
provider = "anthropic"
model = "claude-opus-4-5"

[credentials.env]
ANTHROPIC_API_KEY = "ANTHROPIC_API_KEY"

[[session]]
name = "code-review"
spec = "review"
harness = "default"
timeout = 1800

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt"
ssh_key_env = "SMELT_SSH_KEY"
cpu_request = "500m"
memory_limit = "1Gi"
"#;

    /// Manifest without a `[kubernetes]` block — used to test the misuse guard.
    const NO_KUBERNETES_MANIFEST_TOML: &str = r#"
[job]
name = "test-job"
repo = "git@github.com:example/repo.git"
base_ref = "main"

[environment]
runtime = "docker"
image = "ghcr.io/example/assay-agent:latest"

[credentials]
provider = "anthropic"
model = "claude-opus-4-5"

[credentials.env]
ANTHROPIC_API_KEY = "ANTHROPIC_API_KEY"

[[session]]
name = "code-review"
spec = "review"
harness = "default"
timeout = 1800

[merge]
strategy = "sequential"
target = "main"
"#;

    #[test]
    fn test_generate_pod_spec_snapshot() {
        let manifest = JobManifest::from_str(KUBERNETES_MANIFEST_TOML, Path::new("test.toml"))
            .expect("manifest should parse");

        let pod =
            generate_pod_spec(&manifest, "my-job", "fake-key").expect("pod spec should generate");

        let json = serde_json::to_string_pretty(&pod).expect("pod should serialize to JSON");

        assert!(
            json.contains("initContainers"),
            "expected 'initContainers' in JSON:\n{json}"
        );
        assert!(
            json.contains("alpine/git"),
            "expected 'alpine/git' init container image in JSON:\n{json}"
        );
        assert!(
            json.contains("\"defaultMode\": 256"),
            "expected '\"defaultMode\": 256' (0o400) in JSON:\n{json}"
        );
        assert!(
            json.contains("emptyDir"),
            "expected 'emptyDir' workspace volume in JSON:\n{json}"
        );
        assert!(
            json.contains("smelt-ssh-my-job"),
            "expected 'smelt-ssh-my-job' secret name in JSON:\n{json}"
        );
        assert!(
            json.contains("Never"),
            "expected restart policy 'Never' in JSON:\n{json}"
        );
        assert!(
            json.contains("\"SMELT_GIT_REMOTE\""),
            "expected 'SMELT_GIT_REMOTE' env var name in JSON:\n{json}"
        );
        assert!(
            json.contains("\"git@github.com:example/repo.git\""),
            "expected repo URL as SMELT_GIT_REMOTE value in JSON:\n{json}"
        );
    }

    #[test]
    fn test_generate_pod_spec_requires_kubernetes_config() {
        let manifest = JobManifest::from_str(NO_KUBERNETES_MANIFEST_TOML, Path::new("test.toml"))
            .expect("manifest should parse");

        let result = generate_pod_spec(&manifest, "my-job", "fake-key");

        assert!(
            result.is_err(),
            "expected Err when [kubernetes] block is absent"
        );

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("kubernetes"),
            "expected error message to contain 'kubernetes', got: {err}"
        );
    }

    #[test]
    fn test_generate_pod_spec_resource_limits() {
        let manifest =
            JobManifest::from_str(KUBERNETES_MANIFEST_WITH_RESOURCES_TOML, Path::new("test.toml"))
                .expect("manifest should parse");

        let pod =
            generate_pod_spec(&manifest, "resource-job", "fake-key").expect("pod spec should generate");

        let json = serde_json::to_string_pretty(&pod).expect("pod should serialize to JSON");

        assert!(
            json.contains("requests"),
            "expected 'requests' in JSON:\n{json}"
        );
        assert!(
            json.contains("limits"),
            "expected 'limits' in JSON:\n{json}"
        );
    }
}
