---
id: T01
parent: S02
milestone: M005
provides:
  - kube dep updated to features = ["runtime", "ws"] in smelt-core/Cargo.toml
  - "io-util" added to workspace tokio features in root Cargo.toml
  - PodState { namespace, pod_name, secret_name } defined as private struct in k8s.rs
  - KubernetesProvider struct replaced with real fields: client: Client, state: Arc<Mutex<HashMap<ContainerId, PodState>>>
  - KubernetesProvider::new(manifest) implemented — branches on optional kubeconfig context, wraps errors as SmeltError::Provider
  - crates/smelt-cli/tests/k8s_lifecycle.rs with k8s_manifest(), k8s_provider_or_skip(), and 4 #[ignore] test stubs
key_files:
  - crates/smelt-core/Cargo.toml
  - Cargo.toml
  - crates/smelt-core/src/k8s.rs
  - crates/smelt-cli/tests/k8s_lifecycle.rs
key_decisions:
  - "PodState fields annotated #[allow(dead_code)] — fields are scaffolded for T02-T04 use and will be used without further change"
  - "RuntimeProvider stub messages updated to 'implement in T02/T03/T04' for clarity"
patterns_established:
  - "k8s_provider_or_skip() pattern mirrors compose_provider_or_skip() — env-gated + error-gated skip helper for integration tests"
drill_down_paths:
  - .kata/milestones/M005/slices/S02/tasks/T01-PLAN.md
duration: 30min
verification_result: pass
completed_at: 2026-03-23T00:00:00Z
---

# T01: Add ws feature, define PodState, implement KubernetesProvider::new(), scaffold failing integration tests

**kube ws+runtime features enabled, PodState/KubernetesProvider struct defined with real fields, async new() constructor implemented, k8s_lifecycle.rs scaffolded with 4 #[ignore] tests**

## What Happened

Updated `crates/smelt-core/Cargo.toml` to replace `kube = { version = "3", default-features = true }` with `kube = { version = "3", features = ["runtime", "ws"] }`. The `ws` feature unlocks `AttachedProcess`/exec WebSocket support needed in T03; `runtime` provides `Api`, `Client`, `kube-runtime` crate. Added `"io-util"` to the workspace tokio features in root `Cargo.toml` (required for `AsyncReadExt::read_to_end()` used in T03).

In `k8s.rs`, added imports for `kube::{Client, Config, api::{Api, AttachParams, DeleteParams, PostParams}}`, `kube::config::KubeConfigOptions`, `std::sync::{Arc, Mutex}`, and `std::collections::HashMap`. Defined private `PodState { namespace, pod_name, secret_name }` struct with `#[allow(dead_code)]` since fields are scaffolded for T02-T04. Replaced the zero-field `KubernetesProvider` struct with real fields `client: Client` and `state: Arc<Mutex<HashMap<ContainerId, PodState>>>`.

Implemented `KubernetesProvider::new(manifest)`: when `manifest.kubernetes.context` is `Some(ctx)`, uses `KubeConfigOptions { context: Some(ctx), ..Default::default() }` + `Config::from_kubeconfig(&opts).await` + `Client::try_from(config)` to connect to the named context; otherwise calls `Client::try_default().await` for ambient kubeconfig/in-cluster credentials. Both error paths wrap via `SmeltError::provider_with_source("k8s", "failed to build kube client", e)`.

Created `crates/smelt-cli/tests/k8s_lifecycle.rs` following the `compose_lifecycle.rs` pattern: `k8s_manifest()` builds a `JobManifest` with `runtime = "kubernetes"`, `namespace = "smelt"`, `ssh_key_env = "SMELT_TEST_SSH_KEY"`, `job.name = "smelt-test"`, `job.repo = "git@github.com:example/smelt-test.git"`; `k8s_provider_or_skip()` checks `SMELT_K8S_TEST` env then calls `KubernetesProvider::new()`, skipping gracefully on either absence; 4 `#[tokio::test] #[ignore]` stubs call `k8s_provider_or_skip()` and `todo!("fill in T04")`.

## Deviations

None. All 7 steps executed as specified. The test stub names match the plan exactly (`test_k8s_provision_exec_teardown`, `test_k8s_exec_streaming_callback`, `test_k8s_ssh_file_permissions`, `test_k8s_readiness_confirmed`).

## Files Created/Modified

- `crates/smelt-core/Cargo.toml` — kube dep now `features = ["runtime", "ws"]`
- `Cargo.toml` — tokio workspace features now include `"io-util"`
- `crates/smelt-core/src/k8s.rs` — PodState struct, updated KubernetesProvider struct, new() constructor, updated todo!() messages
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — new file: k8s_manifest(), k8s_provider_or_skip(), 4 #[ignore] test stubs

## Verification Results (Slice-level checks)

- `cargo check -p smelt-core` — ✓ PASS (new kube features compile, all imports resolve)
- `cargo check -p smelt-cli --tests` — ✓ PASS (k8s_lifecycle.rs type-checks, KubernetesProvider::new accessible)
- `cargo test --workspace` — ✓ PASS (154 unit tests pass, 4 k8s tests ignored, 0 failures)
- `grep -n "features.*ws" crates/smelt-core/Cargo.toml` — ✓ PASS (line 27 confirmed)
- `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored` — ✗ EXPECTED FAIL (tests are todo!() stubs — will pass after T04)
