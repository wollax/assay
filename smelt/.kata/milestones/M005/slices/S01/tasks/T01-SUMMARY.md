---
id: T01
parent: S01
milestone: M005
provides:
  - KubernetesConfig struct with 7 fields, deny_unknown_fields, all fields documented
  - JobManifest.kubernetes: Option<KubernetesConfig> — backward-compatible optional field
  - validate() extended to accept "kubernetes" as valid runtime
  - Bidirectional cross-guard in validate(): missing block on k8s runtime → error; k8s block on non-k8s runtime → error
  - Empty namespace/ssh_key_env each produce separate validation errors
  - test_validate_runtime_unknown_rejected fixed to use "podman" (not "kubernetes")
  - 7 new kubernetes tests: roundtrip present/absent, all 4 validation guards, valid manifest
  - kube = "3" and k8s-openapi = "0.27" added to Cargo.toml
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/Cargo.toml
  - crates/smelt-cli/tests/compose_lifecycle.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Used kube = "3" (latest stable) + k8s-openapi = "0.27" — confirmed compatible via crates.io deps API
  - No ws feature on kube per task plan — reserved for S02
  - Cross-guard placed before services guard in validate() for logical grouping
patterns_established:
  - KubernetesConfig follows same derive/serde pattern as ForgeConfig (deny_unknown_fields, #[serde(default)] on Option fields)
  - Validation cross-guards use match-on-Option with field-level error messages naming the field (kubernetes.namespace, kubernetes.ssh_key_env)
observability_surfaces:
  - cargo test -p smelt-core -- kubernetes --nocapture shows all 7 kubernetes test results
  - validate() error messages name exact fields: "kubernetes.namespace: must not be empty", etc.
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Add KubernetesConfig struct, extend validate(), and write manifest tests

**KubernetesConfig struct with bidirectional runtime cross-guard and 7 passing kubernetes tests added to manifest.rs; kube=3 + k8s-openapi=0.27 added to Cargo.toml**

## What Happened

Added `KubernetesConfig` struct immediately after the `ForgeConfig` import in `manifest.rs`. The struct uses `#[serde(deny_unknown_fields)]` matching the existing pattern, with all 7 fields documented: `namespace` and `ssh_key_env` are required `String`s; the remaining 5 (`context`, `cpu_request`, `memory_request`, `cpu_limit`, `memory_limit`) are `Option<String>` with `#[serde(default)]`.

Added `pub kubernetes: Option<KubernetesConfig>` to `JobManifest` between `forge` and `services`, with `#[serde(default)]`. Existing manifests without `[kubernetes]` parse identically — confirmed by `test_kubernetes_roundtrip_absent` reusing `VALID_MANIFEST`.

Extended `VALID_RUNTIMES` from `&["docker", "compose"]` to `&["docker", "compose", "kubernetes"]`. Added the bidirectional cross-guard before the services guard: if `runtime == "kubernetes"` and no `[kubernetes]` block → error; if `[kubernetes]` block present but `runtime != "kubernetes"` → error; within a valid kubernetes block, empty `namespace` or `ssh_key_env` each produce a separate field-named error.

Fixed `test_validate_runtime_unknown_rejected` to use `runtime = "podman"` (the actual unknown value) instead of `"kubernetes"`, since `"kubernetes"` is now valid.

Also updated two struct literal initializers in `crates/smelt-cli/tests/compose_lifecycle.rs` and `crates/smelt-cli/tests/docker_lifecycle.rs` that construct `JobManifest` directly — both needed `kubernetes: None` added.

## Verification

- `cargo test -p smelt-core 2>&1 | tail -10` → `test result: ok. 145 passed; 0 failed`
- `cargo test -p smelt-core -- test_validate_runtime_unknown_rejected` → 1 passed (rejects "podman")
- `cargo test -p smelt-core -- kubernetes` → 7 tests, all pass
- `cargo test --workspace 2>&1 | grep -E "^(test result|FAILED)"` → all green, zero FAILED lines

## Diagnostics

- `cargo test -p smelt-core -- kubernetes --nocapture` shows all 7 kubernetes test results with any failure messages
- Validation errors include field names for targeted diagnosis: `"kubernetes.namespace: must not be empty"`, `"kubernetes.ssh_key_env: must not be empty"`
- Error messages surfaced in `smelt run` output when manifest is invalid

## Deviations

None — implementation exactly matches the task plan. The `compose_lifecycle.rs` and `docker_lifecycle.rs` fixes were not called out in the task plan but were necessary to keep `cargo test --workspace` green; they are trivial `kubernetes: None` additions to struct literals.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — KubernetesConfig struct, JobManifest.kubernetes field, extended validate() with VALID_RUNTIMES and cross-guard, 7 new tests, fixed test_validate_runtime_unknown_rejected
- `crates/smelt-core/Cargo.toml` — kube = "3" and k8s-openapi = "0.27" with v1_32 feature added to [dependencies]
- `crates/smelt-cli/tests/compose_lifecycle.rs` — added kubernetes: None to make_manifest() struct literal
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added kubernetes: None to test_manifest_with_repo() struct literal
