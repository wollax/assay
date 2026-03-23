---
estimated_steps: 8
estimated_files: 4
---

# T02: Implement generate_pod_spec(), KubernetesProvider stub, lib.rs wiring, and example manifest

**Slice:** S01 — Manifest Extension
**Milestone:** M005

## Description

Create `crates/smelt-core/src/k8s.rs` with: (1) `generate_pod_spec()` — a pure synchronous function that builds a `k8s_openapi::api::core::v1::Pod` value with the init container, emptyDir volume, SSH Secret volume (mode 0400 = `defaultMode: Some(256)`), and resource limits; (2) `KubernetesProvider` — an empty stub struct with a full `RuntimeProvider` impl where all 5 methods call `todo!()`; (3) snapshot unit tests using `serde_json` serialization. Wire `pub mod k8s` and `pub use k8s::KubernetesProvider` into `lib.rs`. Create `examples/job-manifest-k8s.toml`. Verify the whole workspace is green and `--dry-run` parses the kubernetes example.

## Steps

1. Create `crates/smelt-core/src/k8s.rs` with a module-level doc comment:
   ```rust
   //! Kubernetes runtime provider and Pod spec generation for Smelt.
   ```
   Add imports:
   ```rust
   use k8s_openapi::api::core::v1::{
       Container, EmptyDirVolumeSource, KeyToPath, Pod, PodSpec,
       ResourceRequirements, SecretVolumeSource, Volume, VolumeMount,
   };
   use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
   use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
   use std::collections::BTreeMap;
   
   use crate::error::SmeltError;
   use crate::manifest::JobManifest;
   use crate::provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};
   ```

2. Implement `generate_pod_spec`:
   ```rust
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
   ```
   Inside the function:
   - Misuse guard: `let kube_cfg = manifest.kubernetes.as_ref().ok_or_else(|| SmeltError::provider("k8s", "generate_pod_spec called without [kubernetes] config block"))?;`
   - SSH Secret volume: `SecretVolumeSource { secret_name: Some(format!("smelt-ssh-{job_name}")), default_mode: Some(256), items: Some(vec![KeyToPath { key: "id_rsa".into(), path: "id_rsa".into(), mode: None }]), optional: None }`
   - emptyDir volume: `Volume { name: "workspace".into(), empty_dir: Some(EmptyDirVolumeSource::default()), ..Default::default() }`
   - SSH volume: `Volume { name: "ssh-key".into(), secret: Some(ssh_vol_source), ..Default::default() }`
   - Init container volume mounts: workspace at `/workspace`, ssh-key at `/root/.ssh` (read only: `Some(true)`)
   - Init container command: `vec!["/bin/sh", "-c", &format!("git clone {} /workspace", manifest.job.repo)]` — using the `repo` field from the manifest
   - Main container resource requirements: build `BTreeMap<String, Quantity>` for requests and limits from the optional fields; only include keys where `Option` is `Some`
   - `Pod` with `ObjectMeta { name: Some(format!("smelt-{job_name}")), namespace: Some(kube_cfg.namespace.clone()), ..Default::default() }` and `PodSpec { init_containers: Some(vec![init_container]), containers: vec![main_container], volumes: Some(vec![workspace_vol, ssh_vol]), restart_policy: Some("Never".into()), ..Default::default() }`
   - Note: `ssh_private_key` parameter is reserved for S02 (creating the K8s Secret); `generate_pod_spec` itself produces the Pod spec only. Mark the param with `let _ = ssh_private_key;` to suppress unused-variable warning.

3. Define `KubernetesProvider` stub:
   ```rust
   /// Kubernetes runtime provider.
   ///
   /// Provisions Smelt jobs as Kubernetes Pods using the ambient kubeconfig
   /// or the context specified in the job manifest's `[kubernetes]` block.
   ///
   /// Full implementation is in S02. All methods currently `todo!()`.
   pub struct KubernetesProvider;
   ```
   Implement `RuntimeProvider for KubernetesProvider` with 5 methods, each calling `todo!("KubernetesProvider not implemented until S02")`. Pay attention to the trait's generic `exec_streaming<F>` — match the exact signature from `provider.rs`.

4. Add `#[cfg(test)]` module with 3 snapshot tests:
   - `test_generate_pod_spec_snapshot`: load a minimal valid kubernetes manifest, call `generate_pod_spec(&manifest, "my-job", "fake-key")`, serialize `serde_json::to_string_pretty(&pod).unwrap()`, assert all of: `json.contains("initContainers")`, `json.contains("alpine/git")`, `json.contains("\"defaultMode\": 256")`, `json.contains("emptyDir")`, `json.contains("smelt-ssh-my-job")`, `json.contains("Never")`.
   - `test_generate_pod_spec_requires_kubernetes_config`: manifest without `[kubernetes]` block → `generate_pod_spec` returns `Err`; error message contains `"kubernetes"`.
   - `test_generate_pod_spec_resource_limits`: manifest with `cpu_request = "500m"` and `memory_limit = "1Gi"` set → JSON contains `"requests"` and `"limits"`.

5. In `crates/smelt-core/src/lib.rs`, add after `pub mod compose;`:
   ```rust
   pub mod k8s;
   ```
   And add after `pub use compose::ComposeProvider;`:
   ```rust
   pub use k8s::KubernetesProvider;
   ```

6. Create `examples/job-manifest-k8s.toml` with the following structure:
   ```toml
   [job]
   name = "my-k8s-job"
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
   memory_request = "512Mi"
   cpu_limit = "2000m"
   memory_limit = "2Gi"
   ```

7. Run `cargo build -p smelt-core` to check for compilation errors; fix any type mismatches in `k8s_openapi` field names (refer to `k8s_openapi::api::core::v1::SecretVolumeSource` docs for the exact field names — most are `Option<T>`).

8. Run `cargo test --workspace` and confirm all tests pass. Run `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` and confirm it exits 0.

## Must-Haves

- [ ] `generate_pod_spec()` compiles and returns a valid `Pod` with init container, emptyDir, SSH Secret volume with `default_mode: Some(256)`, and correct restart policy `"Never"`
- [ ] Misuse guard: `generate_pod_spec()` returns `Err` when `manifest.kubernetes.is_none()`
- [ ] `ssh_private_key` parameter does not appear in generated Pod YAML (reserved for S02 Secret creation)
- [ ] `KubernetesProvider` compiles with all 5 `RuntimeProvider` methods implemented (as `todo!()`)
- [ ] `pub mod k8s` and `pub use k8s::KubernetesProvider` wired into `lib.rs`
- [ ] `examples/job-manifest-k8s.toml` parses and passes `validate()` (verified by `--dry-run`)
- [ ] `cargo test -p smelt-core` exits 0 with all 3 new snapshot tests passing
- [ ] `cargo test --workspace` exits 0 with zero FAILED lines

## Verification

- `cargo test -p smelt-core -- generate_pod_spec 2>&1` — 3 tests pass
- `cargo test --workspace 2>&1 | grep FAILED` — empty output
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run 2>&1` — exits 0
- `cargo doc --package smelt-core 2>&1 | grep "warning\[missing_docs\]"` — empty output (no missing doc warnings)

## Observability Impact

- Signals added/changed: `generate_pod_spec()` returns a typed `Pod` value that can be serialized to JSON/YAML for inspection — a future agent debugging S02 can call `serde_json::to_string_pretty(&pod)` to see the exact spec being applied
- How a future agent inspects this: `cargo test -p smelt-core -- generate_pod_spec --nocapture` prints the full JSON snapshot; failing tests show which JSON substring is absent
- Failure state exposed: `SmeltError::provider("k8s", "generate_pod_spec called without [kubernetes] config block")` — unambiguous, contains the missing config name

## Inputs

- T01 output: `crates/smelt-core/src/manifest.rs` — `KubernetesConfig` struct, `JobManifest.kubernetes` field, `kube` + `k8s-openapi` in Cargo.toml
- `crates/smelt-core/src/compose.rs` — `generate_compose_file()` as the signature reference pattern
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait for the stub impl (exact method signatures)
- `crates/smelt-core/src/lib.rs` — `pub use compose::ComposeProvider` pattern for wiring
- `examples/job-manifest-compose.toml` — structure reference for the new example

## Expected Output

- `crates/smelt-core/src/k8s.rs` — new module with `generate_pod_spec()`, `KubernetesProvider` stub, 3 snapshot tests
- `crates/smelt-core/src/lib.rs` — `pub mod k8s` and `pub use k8s::KubernetesProvider` added
- `examples/job-manifest-k8s.toml` — new kind-compatible example manifest
