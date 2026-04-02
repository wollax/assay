# S01: Manifest Extension

**Goal:** Add `KubernetesConfig` to `JobManifest`, extend `validate()` with the kubernetes runtime guard, implement `generate_pod_spec()` as a pure function returning a typed `k8s_openapi::api::core::v1::Pod`, stub `KubernetesProvider` wired into `lib.rs`, and add roundtrip + validation + snapshot tests.
**Demo:** `cargo test -p smelt-core` proves `[kubernetes]` roundtrip and validation; `smelt run --dry-run` parses a kubernetes manifest without errors; `generate_pod_spec()` snapshot tests confirm init container, SSH Secret mount (mode 0400), volume, and resource limits in the Pod YAML; `cargo test --workspace` shows zero regressions.

## Must-Haves

- `KubernetesConfig` struct with `namespace`, `context`, `ssh_key_env`, and four resource-limit fields; `#[serde(deny_unknown_fields)]`, `#[serde(default)]` on all optional fields; doc comments on every public field
- `JobManifest.kubernetes: Option<KubernetesConfig>` added; existing manifests without `[kubernetes]` parse identically
- `validate()` accepts `"kubernetes"` as a valid runtime; cross-guard: `runtime == "kubernetes"` → `kubernetes` block required with non-empty `namespace` and `ssh_key_env`; `runtime != "kubernetes"` and `kubernetes` block present → error; both directions tested
- `test_validate_runtime_unknown_rejected` updated to use `"podman"` (not `"kubernetes"`) so it still tests an invalid runtime after `"kubernetes"` is added to VALID_RUNTIMES
- `generate_pod_spec(manifest, job_name, ssh_private_key)` in `k8s.rs` returns `crate::Result<Pod>` with: init container (`alpine/git` cloning via SSH into `/workspace` emptyDir), main container (using `environment.image`, `/workspace` mounted), SSH Secret volume (`defaultMode: Some(256)` = 0o400), resource requests/limits from `KubernetesConfig`; misuse guard: returns `Err` if `manifest.kubernetes.is_none()`
- `KubernetesProvider` stub in `k8s.rs`: zero-field struct, `RuntimeProvider` impl where all 5 methods call `todo!()`; doc comments on all public items
- `pub mod k8s` + `pub use k8s::KubernetesProvider` wired into `lib.rs` following `ComposeProvider` pattern
- `kube` and `k8s-openapi` (feature `v1_32`) in `smelt-core/Cargo.toml`; `kube` added without `ws` feature
- `examples/job-manifest-k8s.toml` with `runtime = "kubernetes"` and a valid `[kubernetes]` block that passes `validate()`
- `cargo test --workspace` all green; no regressions in existing tests

## Proof Level

- This slice proves: contract
- Real runtime required: no (unit tests only — no cluster, no network)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-core 2>&1 | tail -5` — all tests pass including new kubernetes roundtrip, validation, and `generate_pod_spec()` snapshot tests
- `cargo test --workspace 2>&1 | grep -E "^(test result|FAILED)"` — all green, zero FAILED lines
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run 2>&1 | head -20` — exits 0 and parses without error (even if `── Kubernetes ──` section is absent until S04)
- Manually verify snapshot test output contains `"initContainers"`, `"defaultMode": 256`, and `"emptyDir"` in the JSON blob

## Observability / Diagnostics

- Runtime signals: none (pure unit/contract slice — no async, no network)
- Inspection surfaces: `cargo test -p smelt-core -- --nocapture kubernetes` to see snapshot output; `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` to confirm CLI parsing path
- Failure visibility: `validate()` returns a collected multi-error string; test failures show which field or guard failed via assertion messages
- Redaction constraints: `ssh_private_key` in `generate_pod_spec()` is the raw key value — must never be logged; snapshot tests use a fixed fake key string, not a real one

## Integration Closure

- Upstream surfaces consumed: `crates/smelt-core/src/manifest.rs` (existing `ForgeConfig` pattern, `validate()`, `VALID_RUNTIMES`), `crates/smelt-core/src/compose.rs` (`generate_compose_file` signature reference), `crates/smelt-core/src/provider.rs` (`RuntimeProvider` trait), `crates/smelt-core/src/lib.rs` (pub use pattern)
- New wiring introduced in this slice: `KubernetesConfig` + `JobManifest.kubernetes` field; `pub mod k8s` in `lib.rs`; `k8s-openapi` and `kube` in Cargo.toml; `examples/job-manifest-k8s.toml`
- What remains before the milestone is truly usable end-to-end: S02 (KubernetesProvider full impl: kube::Client, Pod lifecycle, exec WebSocket, teardown); S03 (push-from-Pod result collection, git fetch in run.rs); S04 (AnyProvider::Kubernetes dispatch in run.rs, `── Kubernetes ──` dry-run section)

## Tasks

- [x] **T01: Add KubernetesConfig struct, extend validate(), and write manifest tests** `est:1h`
  - Why: Establishes the typed manifest surface for kubernetes — the foundation all downstream slices depend on. Closing validation means the TOML schema is locked and tested.
  - Files: `crates/smelt-core/src/manifest.rs`, `crates/smelt-core/Cargo.toml`
  - Do:
    1. Add `kube = { version = "3", default-features = true }` and `k8s-openapi = { version = "0.27", features = ["v1_32"] }` to `[dependencies]` in `smelt-core/Cargo.toml`. The `kube` client types may be needed for the stub even without `ws`.
    2. In `manifest.rs`, define `KubernetesConfig` next to `ForgeConfig`: `#[derive(Debug, Deserialize)]`, `#[serde(deny_unknown_fields)]`, doc comment on the struct, doc comments on all 7 fields (`namespace: String`, `context: Option<String>`, `ssh_key_env: String`, `cpu_request: Option<String>`, `memory_request: Option<String>`, `cpu_limit: Option<String>`, `memory_limit: Option<String>`). All `Option` fields must have `#[serde(default)]`.
    3. Add `pub kubernetes: Option<KubernetesConfig>` to `JobManifest` with `#[serde(default)]` and a doc comment. Position it after `forge` and before `services`.
    4. In `validate()`, extend `VALID_RUNTIMES` to `&["docker", "compose", "kubernetes"]`.
    5. Add cross-field guards after the existing `services` runtime guard:
       - If `runtime == "kubernetes"` and `self.kubernetes.is_none()` → push error `"kubernetes: `runtime = \"kubernetes\"` requires a `[kubernetes]` block"`.
       - If `runtime == "kubernetes"` and `kubernetes` block has empty `namespace` → push error.
       - If `runtime == "kubernetes"` and `kubernetes` block has empty `ssh_key_env` → push error.
       - If `runtime != "kubernetes"` and `self.kubernetes.is_some()` → push error `"kubernetes: `[kubernetes]` block requires `runtime = \"kubernetes\"`"`.
    6. Update `test_validate_runtime_unknown_rejected`: change `runtime = "kubernetes"` to `runtime = "podman"` so the test still validates that unknown runtimes are rejected.
    7. Add tests at the bottom of the `#[cfg(test)]` block:
       - `test_kubernetes_roundtrip_present`: TOML with `[kubernetes]` block, all 7 fields set, verify parsed struct fields.
       - `test_kubernetes_roundtrip_absent`: existing-style manifest without `[kubernetes]`, verify `manifest.kubernetes.is_none()`.
       - `test_validate_kubernetes_runtime_requires_block`: `runtime = "kubernetes"` but no `[kubernetes]` → `validate()` error.
       - `test_validate_kubernetes_block_requires_runtime`: `runtime = "docker"` + `[kubernetes]` block → `validate()` error.
       - `test_validate_kubernetes_empty_namespace`: `runtime = "kubernetes"` + `[kubernetes]` block with `namespace = ""` → error.
       - `test_validate_kubernetes_empty_ssh_key_env`: `runtime = "kubernetes"` + `[kubernetes]` block with `ssh_key_env = ""` → error.
       - `test_validate_kubernetes_valid`: fully valid kubernetes manifest passes `validate()`.
  - Verify: `cargo test -p smelt-core 2>&1 | grep -E "(FAILED|ok|test result)"` — all pass including the 7 new tests and the updated `test_validate_runtime_unknown_rejected`.
  - Done when: `cargo test -p smelt-core` exits 0 with all kubernetes-related tests passing and `test_validate_runtime_unknown_rejected` still testing the right thing.

- [x] **T02: Implement generate_pod_spec(), KubernetesProvider stub, lib.rs wiring, and example manifest** `est:1.5h`
  - Why: Delivers the boundary contract outputs that S02/S03/S04 all depend on — the typed Pod spec generator and the KubernetesProvider stub that will be fleshed out in S02. Also creates the example manifest that the dry-run verification exercises.
  - Files: `crates/smelt-core/src/k8s.rs` (new), `crates/smelt-core/src/lib.rs`, `examples/job-manifest-k8s.toml` (new)
  - Do:
    1. Create `crates/smelt-core/src/k8s.rs` with module-level doc comment.
    2. Import `k8s_openapi::api::core::v1::{Container, EmptyDirVolumeSource, EnvVar, KeyToPath, Pod, PodSpec, ResourceRequirements, Secret, SecretVolumeSource, Volume, VolumeMount}`, `k8s_openapi::apimachinery::pkg::api::resource::Quantity`, `k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta`, `std::collections::BTreeMap`, and `crate::manifest::JobManifest`.
    3. Implement `pub fn generate_pod_spec(manifest: &JobManifest, job_name: &str, ssh_private_key: &str) -> crate::Result<Pod>`:
       - Guard: if `manifest.kubernetes.is_none()` return `Err(SmeltError::provider("k8s", "generate_pod_spec called without kubernetes config"))`.
       - Read `kube_cfg = manifest.kubernetes.as_ref().unwrap()`.
       - Build SSH Secret volume source: `SecretVolumeSource { secret_name: Some(format!("smelt-ssh-{job_name}")), default_mode: Some(256), items: Some(vec![KeyToPath { key: "id_rsa".into(), path: "id_rsa".into(), mode: None }]), optional: None }`.
       - Build emptyDir volume for `/workspace`.
       - Build init container: `name: "git-clone"`, `image: Some("alpine/git".into())`, command with `git clone` via SSH using the repo URL into `/workspace`, `volume_mounts` for both emptyDir (`/workspace`) and SSH secret (`/root/.ssh`).
       - Build main container: `name: "smelt-agent"`, `image: Some(manifest.environment.image.clone())`, `volume_mounts` for emptyDir only (`/workspace`), `resources` from `kube_cfg` cpu/memory fields using `BTreeMap<String, Quantity>` — only insert fields where `Option` is `Some`.
       - Build `Pod` with `ObjectMeta { name: Some(format!("smelt-{job_name}")), namespace: Some(kube_cfg.namespace.clone()), ..Default::default() }` and `PodSpec { init_containers: Some(vec![init_container]), containers: vec![main_container], volumes: Some(vec![workspace_vol, ssh_vol]), restart_policy: Some("Never".into()), ..Default::default() }`.
       - Return `Ok(pod)`.
    4. Add `/// Kubernetes runtime provider (stub — full implementation in S02).` doc comment + `pub struct KubernetesProvider;` with empty field.
    5. Add `impl RuntimeProvider for KubernetesProvider` with all 5 methods calling `todo!("KubernetesProvider not implemented until S02")`.
    6. Add snapshot unit tests for `generate_pod_spec()`:
       - `test_generate_pod_spec_snapshot`: call with a minimal valid manifest, serialize to JSON via `serde_json::to_string_pretty(&pod).unwrap()`, assert key substrings: `"initContainers"`, `"alpine/git"`, `"defaultMode": 256`, `"emptyDir"`, `"smelt-ssh-"`, `"Never"`.
       - `test_generate_pod_spec_requires_kubernetes_config`: manifest without `[kubernetes]` block → `generate_pod_spec` returns `Err`.
       - `test_generate_pod_spec_resource_limits`: manifest with cpu/memory limits → JSON contains `"requests"` and `"limits"` keys.
    7. In `lib.rs`, add `pub mod k8s;` and `pub use k8s::KubernetesProvider;` following the `ComposeProvider` pattern.
    8. Create `examples/job-manifest-k8s.toml` — a fully valid manifest with `runtime = "kubernetes"`, a `[kubernetes]` block (`namespace = "smelt"`, `ssh_key_env = "SMELT_SSH_KEY"`, `cpu_request = "500m"`, `memory_request = "512Mi"`), and the standard `[job]`, `[environment]`, `[credentials]`, `[[session]]`, `[merge]` sections.
  - Verify:
    1. `cargo test -p smelt-core 2>&1 | grep -E "(FAILED|test result)"` — all pass including the 3 new snapshot tests.
    2. `cargo test --workspace 2>&1 | grep -E "^(test result|FAILED)"` — zero regressions.
    3. `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` — exits 0 (manifest parses and validates; `── Kubernetes ──` section absence is expected until S04).
    4. `cargo doc --package smelt-core 2>&1 | grep -c "warning"` — zero `missing_docs` warnings (doc comments on all pub items in `k8s.rs`).
  - Done when: All three verification checks pass; `cargo test --workspace` is fully green; `examples/job-manifest-k8s.toml` parses and validates without errors.

## Files Likely Touched

- `crates/smelt-core/Cargo.toml` — add `kube` and `k8s-openapi` deps
- `crates/smelt-core/src/manifest.rs` — `KubernetesConfig`, `JobManifest.kubernetes`, `validate()`, new tests
- `crates/smelt-core/src/k8s.rs` — new module: `generate_pod_spec()`, `KubernetesProvider` stub, tests
- `crates/smelt-core/src/lib.rs` — `pub mod k8s`, `pub use k8s::KubernetesProvider`
- `examples/job-manifest-k8s.toml` — new example manifest
