---
id: S01
parent: M005
milestone: M005
provides:
  - KubernetesConfig struct (7 fields, deny_unknown_fields, all documented) — the typed TOML schema for [kubernetes] blocks
  - JobManifest.kubernetes: Option<KubernetesConfig> — backward-compatible optional field (existing manifests unaffected)
  - validate() bidirectional cross-guard: k8s runtime without block → error; k8s block without k8s runtime → error; empty namespace/ssh_key_env → separate field-named errors
  - VALID_RUNTIMES extended to include "kubernetes"; test_validate_runtime_unknown_rejected updated to use "podman"
  - generate_pod_spec(manifest, job_name, ssh_private_key) → Result<Pod>: pure function producing k8s_openapi Pod with init container (alpine/git cloning via SSH into emptyDir /workspace), main container (environment.image), SSH Secret volume (defaultMode 256 = 0o400), resource requests/limits from KubernetesConfig optional fields
  - KubernetesProvider stub: zero-field struct, full RuntimeProvider impl (all 5 methods todo!()), documented
  - pub mod k8s + pub use k8s::KubernetesProvider wired into lib.rs following ComposeProvider pattern
  - kube = "3" + k8s-openapi = "0.27" (v1_32 feature) added to smelt-core/Cargo.toml
  - examples/job-manifest-k8s.toml — fully valid kubernetes manifest that parses, validates, and passes --dry-run
  - 10 kubernetes tests: 7 manifest roundtrip/validation, 3 generate_pod_spec snapshot tests — all passing
requires: []
affects:
  - S02
  - S03
  - S04
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/k8s.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/Cargo.toml
  - crates/smelt-cli/tests/compose_lifecycle.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest-k8s.toml
key_decisions:
  - kube = "3" (latest stable) + k8s-openapi = "0.27" — compatible pair confirmed via crates.io; no ws feature on kube (reserved for S02)
  - defaultMode: Some(256) for SSH key volume — 0o400 user-read-only required by SSH client; K8s mounts at 0444 by default without this
  - ssh_private_key parameter suppressed with let _ = ssh_private_key in generate_pod_spec — param is reserved for S02 Secret creation, not consumed by the pure spec-builder
  - KubernetesConfig validation errors name the field ("kubernetes.namespace: must not be empty") for targeted diagnosis
  - Cross-guard placed before services guard in validate() for logical grouping
patterns_established:
  - KubernetesConfig follows same derive/serde pattern as ForgeConfig (deny_unknown_fields, #[serde(default)] on Option fields)
  - generate_pod_spec follows generate_compose_file signature reference (pure function, manifest + named params → Result<T>)
  - Snapshot tests serialize to serde_json::to_string_pretty and assert substring presence for field-level verification
observability_surfaces:
  - cargo test -p smelt-core -- kubernetes --nocapture — shows all 10 kubernetes/pod-spec test results with failure messages
  - cargo test -p smelt-core -- generate_pod_spec --nocapture — prints full JSON Pod snapshot; failing tests show which substring absent
  - validate() error messages name exact fields: "kubernetes.namespace: must not be empty", etc. — surfaced in smelt run output
  - cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run — confirms full CLI parsing path including validation
drill_down_paths:
  - .kata/milestones/M005/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S01/tasks/T02-SUMMARY.md
duration: 45min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S01: Manifest Extension

**`KubernetesConfig` + `generate_pod_spec()` + `KubernetesProvider` stub wired into `smelt-core` — typed TOML schema locked, bidirectional validation guards enforced, Pod spec generator snapshot-tested, all 154 `smelt-core` tests and 23 `smelt-cli` integration tests green**

## What Happened

### T01: KubernetesConfig struct, extended validate(), manifest tests

Added `KubernetesConfig` to `manifest.rs` following the `ForgeConfig` pattern — `#[serde(deny_unknown_fields)]`, `#[serde(default)]` on all optional fields, doc comments on every field. The struct carries 7 fields: `namespace` and `ssh_key_env` as required `String`s; `context`, `cpu_request`, `memory_request`, `cpu_limit`, `memory_limit` as `Option<String>`.

Added `pub kubernetes: Option<KubernetesConfig>` to `JobManifest` between `forge` and `services` with `#[serde(default)]`. Existing manifests without `[kubernetes]` parse identically — confirmed by `test_kubernetes_roundtrip_absent` reusing `VALID_MANIFEST`.

Extended `VALID_RUNTIMES` to `&["docker", "compose", "kubernetes"]`. Added the bidirectional cross-guard: if `runtime == "kubernetes"` and no `[kubernetes]` block → error; if `[kubernetes]` block present but `runtime != "kubernetes"` → error; empty `namespace` or `ssh_key_env` each produce separate field-named errors. Fixed `test_validate_runtime_unknown_rejected` to use `"podman"`. Also updated `kubernetes: None` in two struct literal initializers in `compose_lifecycle.rs` and `docker_lifecycle.rs` to keep `cargo test --workspace` green.

7 new kubernetes tests all pass: roundtrip present/absent, 4 validation guards, valid manifest smoke test.

### T02: generate_pod_spec(), KubernetesProvider stub, lib.rs wiring, example manifest

Created `crates/smelt-core/src/k8s.rs`. The `generate_pod_spec()` function is pure and synchronous — it builds a fully-typed `k8s_openapi::api::core::v1::Pod` value:

- **Init container**: `alpine/git`, clones repo via SSH into emptyDir `/workspace`
- **SSH Secret volume**: `SecretVolumeSource { default_mode: Some(256), items: [KeyToPath { key: "id_rsa", path: "id_rsa" }] }` — 0o400 mode required by SSH client
- **Main container**: `manifest.environment.image`, `/workspace` emptyDir mounted, resource requests/limits from `KubernetesConfig` optional fields using `BTreeMap<String, Quantity>` (keys omitted when `None`)
- **Misuse guard**: returns `SmeltError::provider("k8s", ...)` when `manifest.kubernetes.is_none()`
- **`ssh_private_key` parameter**: suppressed with `let _ = ssh_private_key` — reserved for S02 Secret creation API call

`KubernetesProvider` stub (zero-field struct, all 5 `RuntimeProvider` methods call `todo!()`), `pub mod k8s` + `pub use k8s::KubernetesProvider` wired into `lib.rs`, and `examples/job-manifest-k8s.toml` (valid, passes `validate()`, exits 0 on `--dry-run`) completed the boundary contract outputs.

3 snapshot tests confirm pod spec shape, misuse guard, and resource limits serialization.

## Verification

- `cargo test -p smelt-core` → 154 passed (including 10 new kubernetes tests), 0 failed
- `cargo test --workspace` → all 9 test suites green, 0 FAILED lines
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` → exits 0, prints `Runtime: kubernetes` in execution plan
- `cargo doc --package smelt-core --no-deps` → zero `missing_docs` warnings on k8s.rs public items
- `cargo test -p smelt-core -- generate_pod_spec --nocapture` confirms snapshot JSON contains `"initContainers"`, `"defaultMode": 256`, `"emptyDir"`, `"smelt-ssh-"`, `"Never"`

## Requirements Advanced

- R021 (Multi-machine coordination via Kubernetes) — S01 locks the typed manifest schema (`KubernetesConfig`), validation logic, and Pod spec generator that all downstream slices (S02–S04) build on. This is the contract foundation; R021 will be validated when S02–S04 complete the provider lifecycle, push-from-Pod, and CLI dispatch.

## Requirements Validated

- None validated by this slice alone — S01 is contract-only (no cluster, no network). R021 validation deferred to S02–S04 integration tests.

## New Requirements Surfaced

- None discovered during execution.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

The `compose_lifecycle.rs` and `docker_lifecycle.rs` `kubernetes: None` additions were not called out in the task plan but were necessary to keep `cargo test --workspace` green. They are trivial struct-literal completions with no behavioral impact.

## Known Limitations

- `KubernetesProvider` is a stub — all 5 `RuntimeProvider` methods call `todo!()`. Full implementation deferred to S02.
- `generate_pod_spec()` does not use `ssh_private_key` yet — the Key material will be placed into the K8s Secret API call in S02.
- `--dry-run` shows `Runtime: kubernetes` but no `── Kubernetes ──` section with namespace/context/resources — that section is deferred to S04 (`print_execution_plan()` extension).
- `AnyProvider::Kubernetes` variant and CLI dispatch are deferred to S04.

## Follow-ups

- S02: Implement `KubernetesProvider` — `kube::Client`, Pod lifecycle (provision + readiness polling + exec WebSocket + teardown), `ws` feature on kube crate, integration tests against kind
- S03: Push-from-Pod result collection — `git fetch origin` in `run.rs` Phase 8 for kubernetes runtime path
- S04: `AnyProvider::Kubernetes` dispatch, `── Kubernetes ──` dry-run section, `examples/job-manifest-k8s.toml` kind-ready verification

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — KubernetesConfig struct, JobManifest.kubernetes field, extended validate() cross-guard, 7 new tests, fixed test_validate_runtime_unknown_rejected
- `crates/smelt-core/Cargo.toml` — kube = "3" and k8s-openapi = "0.27" with v1_32 feature
- `crates/smelt-core/src/k8s.rs` — new: generate_pod_spec(), KubernetesProvider stub, 3 snapshot tests
- `crates/smelt-core/src/lib.rs` — pub mod k8s + pub use k8s::KubernetesProvider
- `crates/smelt-cli/tests/compose_lifecycle.rs` — kubernetes: None added to make_manifest() struct literal
- `crates/smelt-cli/tests/docker_lifecycle.rs` — kubernetes: None added to test_manifest_with_repo() struct literal
- `examples/job-manifest-k8s.toml` — new kubernetes example manifest

## Forward Intelligence

### What the next slice should know

- `generate_pod_spec()` builds the full Pod object but does NOT create the SSH Secret — that is S02's job. S02's `provision()` must: (1) create the Secret from `ssh_private_key` via `Api::<Secret>::namespaced(client, ns).create(...)`, (2) call `generate_pod_spec()` to get the Pod object, (3) create the Pod, (4) poll readiness. The Secret name is `format!("smelt-ssh-{job_name}")` — must match between Secret creation and Pod volume reference.
- The `ws` feature on the `kube` crate is intentionally absent from S01. S02 must add `kube = { version = "3", features = ["ws"] }` to unlock `AttachedProcess` for exec WebSocket. Changing a feature flag requires `cargo check` to recompile kube — plan for it.
- `KubernetesConfig.context` is `Option<String>` — when `None`, S02 should use ambient kubeconfig context. The `kube::Config::from_kubeconfig` path vs `kube::Client::try_default()` is the relevant branch.
- Resource quantities in `generate_pod_spec()` use `k8s_openapi::apimachinery::pkg::api::resource::Quantity` — these are opaque string wrappers. K8s validates the format on Pod creation, not in Rust. Format errors surface only at cluster-apply time.

### What's fragile

- `defaultMode: Some(256)` — this is the critical SSH permission field. If it is ever serialized differently or if the K8s API coerces it, SSH will fail with `"bad permissions"` on the key file. Verify with `stat /root/.ssh/id_rsa` in S02 integration tests.
- The init container command in `generate_pod_spec()` uses a placeholder git clone invocation — S02 integration tests will expose whether the SSH URL format and the init container command are correct for a real kind cluster.

### Authoritative diagnostics

- `cargo test -p smelt-core -- generate_pod_spec --nocapture` — prints the full JSON Pod snapshot; use this to inspect the exact serialized structure before submitting to the K8s API in S02
- `SmeltError::provider("k8s", "generate_pod_spec called without [kubernetes] config block")` — the misuse guard message; contains "kubernetes" so assertion `error.contains("kubernetes")` works in tests
- `validate()` error messages name exact fields — `"kubernetes.namespace: must not be empty"` — reliable for test assertions and user-facing diagnosis

### What assumptions changed

- No assumptions changed. The manifest schema and Pod spec structure matched the plan exactly. The one adjustment (suppressing `ssh_private_key` with `let _`) was the correct call — the parameter belongs in the signature for S02 to use, not in the pure spec builder.
