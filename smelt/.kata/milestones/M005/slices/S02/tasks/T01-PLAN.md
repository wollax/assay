---
estimated_steps: 7
estimated_files: 4
---

# T01: Add ws feature, define PodState, implement KubernetesProvider::new(), scaffold failing integration tests

**Slice:** S02 — KubernetesProvider Lifecycle
**Milestone:** M005

## Description

This task unlocks the `kube` crate's WebSocket exec capability, defines the internal `PodState` type and `KubernetesProvider` struct with real fields, implements `KubernetesProvider::new()` for client construction, and creates the integration test scaffold that T02–T04 will populate. It is purely additive — no behavioral logic is placed in the 5 `RuntimeProvider` methods yet.

## Steps

1. Check current workspace tokio features in `Cargo.toml` (root). Add `"io-util"` to the tokio features list if absent — this is required for `AsyncReadExt::read_to_end()` used in T03.
2. In `crates/smelt-core/Cargo.toml`, change `kube = { version = "3", default-features = true }` to `kube = { version = "3", features = ["runtime", "ws"] }`. The `runtime` feature provides `Api`, `Client`, etc.; `ws` adds `AttachedProcess`/exec WebSocket support.
3. Run `cargo check -p smelt-core` to confirm the kube feature change compiles cleanly. Fix any feature resolution errors.
4. In `crates/smelt-core/src/k8s.rs`, add required imports: `use kube::{Client, Config, api::{Api, PostParams, DeleteParams, AttachParams}};`, `use kube::config::KubeConfigOptions;`, `use std::sync::{Arc, Mutex};`, `use std::collections::HashMap;`. Define private `struct PodState { namespace: String, pod_name: String, secret_name: String }`.
5. Replace the zero-field `KubernetesProvider` struct with: `pub struct KubernetesProvider { client: Client, state: Arc<Mutex<HashMap<ContainerId, PodState>>> }`. Implement `pub async fn new(manifest: &JobManifest) -> crate::Result<Self>`: branch on `manifest.kubernetes.as_ref().and_then(|k| k.context.as_ref())` — if `Some(ctx)`, use `KubeConfigOptions { context: Some(ctx.clone()), ..Default::default() }` + `Config::from_kubeconfig(&opts).await` + `Client::try_from(config)`; otherwise use `Client::try_default().await`. Wrap errors with `SmeltError::provider_with_source("k8s", "failed to build kube client", e)`.
6. Update all 5 `RuntimeProvider` impl methods to `todo!("implement in T02/T03/T04")` (replacing the old stub message).
7. Create `crates/smelt-cli/tests/k8s_lifecycle.rs`: (a) add `k8s_manifest()` helper returning a `JobManifest` with `runtime = "kubernetes"`, `image = "alpine:3"`, `namespace = "smelt"`, `ssh_key_env = "SMELT_TEST_SSH_KEY"`, `job.name = "smelt-test"`, `job.repo = "git@github.com:example/smelt-test.git"`; (b) add `async fn k8s_provider_or_skip()` that checks `std::env::var("SMELT_K8S_TEST").is_ok()` — returns `None` with `eprintln!("Skipping: SMELT_K8S_TEST not set")` if not; calls `KubernetesProvider::new(&k8s_manifest()).await` — returns `None` with `eprintln!("Skipping: cluster unavailable: {e}")` on error; returns `Some(provider)`; (c) add 4 `#[tokio::test] #[ignore]` test stubs: `test_k8s_provision_exec_teardown`, `test_k8s_exec_streaming_callback`, `test_k8s_ssh_file_permissions`, `test_k8s_readiness_confirmed` — each calls `k8s_provider_or_skip()` and immediately panics with `todo!("fill in T04")`.

## Must-Haves

- [ ] `kube = { version = "3", features = ["runtime", "ws"] }` in `crates/smelt-core/Cargo.toml`
- [ ] `"io-util"` in workspace tokio features (root `Cargo.toml`)
- [ ] `PodState { namespace, pod_name, secret_name }` defined as private struct in `k8s.rs`
- [ ] `KubernetesProvider { client: Client, state: Arc<Mutex<...>> }` replaces zero-field struct
- [ ] `KubernetesProvider::new(manifest)` handles both ambient kubeconfig and explicit context paths
- [ ] `crates/smelt-cli/tests/k8s_lifecycle.rs` exists with `k8s_provider_or_skip()`, `k8s_manifest()`, and 4 `#[ignore]` test stubs
- [ ] `cargo check -p smelt-core && cargo check -p smelt-cli --tests` both pass
- [ ] `cargo test --workspace` all green (stubs are `#[ignore]` so they don't run)

## Verification

- `cargo check -p smelt-core` — compiles with new kube features
- `cargo check -p smelt-cli --tests` — `k8s_lifecycle.rs` type-checks
- `cargo test --workspace` — all existing tests pass, 0 failures, new tests skipped (ignore)
- `grep -n "features.*ws" crates/smelt-core/Cargo.toml` — confirms ws feature present

## Observability Impact

- Signals added/changed: None yet — methods are still `todo!()`
- How a future agent inspects this: `cargo check -p smelt-core` validates the compile-time contract; `cargo test --workspace` confirms no regressions
- Failure state exposed: `KubernetesProvider::new()` returns `Err(SmeltError::Provider)` when cluster unreachable — this surfaces in the `k8s_provider_or_skip()` helper in tests

## Inputs

- `crates/smelt-core/src/k8s.rs` — current stub with zero-field struct and todo!() methods
- `crates/smelt-core/Cargo.toml` — current kube dep without ws feature
- `crates/smelt-cli/tests/compose_lifecycle.rs` — pattern reference for `compose_provider_or_skip()` and test structure

## Expected Output

- `crates/smelt-core/Cargo.toml` — kube dep has `features = ["runtime", "ws"]`
- `Cargo.toml` (root) — tokio workspace features include `"io-util"`
- `crates/smelt-core/src/k8s.rs` — `PodState` struct, updated `KubernetesProvider` struct with `client` and `state` fields, `new()` async constructor
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — new file with 4 `#[ignore]` test stubs ready for T04
