# S02: KubernetesProvider Lifecycle

**Goal:** Replace the `KubernetesProvider` stub with a full `RuntimeProvider` implementation — Secret creation, Pod creation, readiness polling, WebSocket exec (buffered + streaming), and idempotent teardown — then prove all four operations against a real kind cluster via integration tests.
**Demo:** `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle` passes all four lifecycle tests: provision (init clone + agent Running), exec (echo hello, exit 0), exec_streaming (callback fires), teardown (Pod + Secret deleted from namespace). The `kube` exec WebSocket risk and Pod readiness detection risk are retired.

## Must-Haves

- `KubernetesProvider::new(manifest)` constructs a `kube::Client` from the manifest's optional context (or ambient kubeconfig) and returns `Err` cleanly when the cluster is unreachable
- `provision()` creates the SSH Secret (`smelt-ssh-<job-name>`) from the env var, creates the Pod via `generate_pod_spec()`, and polls readiness until init container exits 0 AND main container is `Running` — returns `Err` on init failure or image pull backoff; tolerates up to 60×2s (120s)
- `exec()` opens a WebSocket exec via `AttachedProcess`, drains stdout/stderr into buffers, awaits `Status`, and returns an `ExecHandle` with the correct exit code from `Status.code`
- `exec_streaming()` opens the same WebSocket exec and calls the `FnMut(&str) + Send + 'static` callback for each stdout/stderr chunk as it arrives; full buffered output also available in the returned `ExecHandle`
- `teardown()` deletes both the Pod and the SSH Secret idempotently — 404 (already gone) is not an error; logged, not propagated (D023)
- `collect()` is a no-op returning an empty `CollectResult` (Phase 8 collection is S03's job)
- Integration test file `crates/smelt-cli/tests/k8s_lifecycle.rs` exists with `k8s_provider_or_skip()` helper + 4 `#[ignore]` tests that pass when `SMELT_K8S_TEST=1` is set and a kind cluster is available
- `cargo test --workspace` stays green regardless of cluster availability

## Proof Level

- This slice proves: integration (against real kind cluster via `SMELT_K8S_TEST=1`)
- Real runtime required: yes — kind cluster for integration tests; unit/cargo tests pass without it
- Human/UAT required: no — deferred to S04-UAT.md

## Verification

- `cargo test -p smelt-core` — all existing tests pass (no regressions from Cargo.toml changes)
- `cargo test --workspace` — all workspace tests pass (no regressions)
- `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored` — all 4 lifecycle tests pass
- `kubectl get pods -n smelt` after test run shows no smelt-* pods (teardown confirmed)
- `kubectl get secrets -n smelt` after test run shows no smelt-ssh-* secrets (Secret cleanup confirmed)

## Observability / Diagnostics

- Runtime signals: `tracing::info!` at provision start, Secret created, Pod created, readiness achieved; `tracing::warn!` on teardown 404 (already gone) and non-fatal delete errors
- Inspection surfaces: `kubectl get pods -n <namespace>`, `kubectl logs <pod> -c git-clone` (init container), `kubectl describe pod <pod>` for readiness failures; `RUST_LOG=smelt_core=debug cargo test` for trace output
- Failure visibility: `SmeltError::Provider` carries the operation name ("provision", "exec", "teardown") + message; readiness timeout includes pod name and namespace; image pull backoff surfaces the waiting reason from `container_statuses.state.waiting.reason`
- Redaction constraints: SSH private key bytes are never logged — only the Secret name; container env vars (credentials) are never logged

## Integration Closure

- Upstream surfaces consumed: `generate_pod_spec()`, `KubernetesConfig`, `JobManifest.kubernetes` from S01; `RuntimeProvider` trait, `ContainerId`, `ExecHandle`, `CollectResult` from `provider.rs`
- New wiring introduced in this slice: `KubernetesProvider` goes from stub (`todo!()`) to fully functional; `kube = { version = "3", features = ["ws"] }` unlocks `AttachedProcess`; `tokio` gains `"io-util"` feature for `AsyncReadExt`
- What remains before the milestone is truly usable end-to-end: S03 (push-from-Pod result collection + `run.rs` Phase 8 git fetch); S04 (CLI dispatch `AnyProvider::Kubernetes`, dry-run `── Kubernetes ──` section)

## Tasks

- [x] **T01: Add ws feature, define PodState, implement KubernetesProvider::new(), scaffold failing integration tests** `est:45m`
  - Why: Unlocks `AttachedProcess` for exec (requires `ws` feature), defines the internal state type, wires client construction, and establishes the test scaffold that T02–T04 will make pass
  - Files: `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/k8s.rs`, `crates/smelt-cli/tests/k8s_lifecycle.rs`
  - Do: (1) Change `kube = { version = "3", default-features = true }` to `kube = { version = "3", features = ["runtime", "ws"] }` in `smelt-core/Cargo.toml`; add `"io-util"` to the workspace tokio features in root `Cargo.toml` (check current features first). (2) In `k8s.rs`: add `use kube::{Client, Config, api::{Api, PostParams, DeleteParams, AttachParams}}; use kube::config::KubeConfigOptions; use std::sync::{Arc, Mutex}; use std::collections::HashMap;` imports. (3) Define private `struct PodState { namespace: String, pod_name: String, secret_name: String }`. (4) Replace zero-field `KubernetesProvider` with `pub struct KubernetesProvider { client: Client, state: Arc<Mutex<HashMap<ContainerId, PodState>>> }`. (5) Implement `pub async fn new(manifest: &JobManifest) -> crate::Result<Self>`: if `manifest.kubernetes.as_ref().and_then(|k| k.context.as_ref())` is `Some(ctx)`, use `KubeConfigOptions { context: Some(ctx.clone()), ..Default::default() }` + `Config::from_kubeconfig(&opts).await` + `Client::try_from(config)`; otherwise `Client::try_default().await`; wrap errors with `SmeltError::provider_with_source("k8s", "failed to build kube client", e)`. (6) Keep all 5 `RuntimeProvider` methods as `todo!("implement in T02/T03/T04")` — only `new()` changes. (7) Create `crates/smelt-cli/tests/k8s_lifecycle.rs` with: a `k8s_provider_or_skip()` helper that checks `std::env::var("SMELT_K8S_TEST").is_ok()` and calls `KubernetesProvider::new(&manifest).await` returning `None` on error; a `k8s_manifest()` helper building a `JobManifest` with `runtime = "kubernetes"`, `namespace = "smelt"`, `ssh_key_env = "SMELT_TEST_SSH_KEY"`; 4 `#[tokio::test] #[ignore]` tests: `test_k8s_provision_exec_teardown`, `test_k8s_exec_streaming_callback`, `test_k8s_ssh_file_permissions`, `test_k8s_readiness_with_slow_init` — each calls `k8s_provider_or_skip()` and asserts `todo!()` for now (will be replaced in T04)
  - Verify: `cargo check -p smelt-core` compiles (ws feature present, imports resolve); `cargo check -p smelt-cli --tests` compiles the new test file; `cargo test --workspace` all green (the 4 tests are `#[ignore]` so they don't run by default)
  - Done when: `cargo check -p smelt-core && cargo check -p smelt-cli --tests && cargo test --workspace` all pass with 0 failures; `crates/smelt-cli/tests/k8s_lifecycle.rs` exists with 4 `#[ignore]` test stubs

- [x] **T02: Implement provision() — SSH Secret + Pod creation + readiness polling** `est:60m`
  - Why: Core of the lifecycle — creates the K8s resources and waits until the Pod is ready for exec; retires the Pod readiness detection risk and SSH file permission risk from the roadmap
  - Files: `crates/smelt-core/src/k8s.rs`
  - Do: (1) Add imports: `use k8s_openapi::api::core::v1::{Pod, Secret}; use k8s_openapi::ByteString; use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta; use std::collections::BTreeMap; use tokio::time::{sleep, Duration};`. (2) Implement `provision()`: read `manifest.kubernetes` (error if None), extract `namespace`, `ssh_key_env`, derive `job_name = &manifest.job.name`, `pod_name = format!("smelt-{job_name}")`, `secret_name = format!("smelt-ssh-{job_name}")`. (3) Read ssh key bytes: `std::env::var(&kube_cfg.ssh_key_env).map_err(|_| SmeltError::provider("k8s", format!("env var '{}' not set", kube_cfg.ssh_key_env)))?.into_bytes()`. (4) Create Secret: `let mut data = BTreeMap::new(); data.insert("id_rsa".to_string(), ByteString(key_bytes)); let secret = Secret { metadata: ObjectMeta { name: Some(secret_name.clone()), namespace: Some(ns.clone()), ..Default::default() }, data: Some(data), ..Default::default() }; Api::<Secret>::namespaced(client.clone(), &ns).create(&PostParams::default(), &secret).await?` (wrap error). (5) Create Pod: call `generate_pod_spec(manifest, job_name, "")` then `Api::<Pod>::namespaced(client.clone(), &ns).create(&PostParams::default(), &pod).await?` (wrap error; on error, attempt to delete the Secret for cleanup). (6) Readiness poll loop: `for _ in 0..60 { let pod = pods_api.get(&pod_name).await?; let status = pod.status.as_ref(); [check init terminated exit_code==0 AND main container running — break]; [check init terminated exit_code!=0 — error]; [check main container waiting reason "ImagePullBackOff"|"ErrImagePull" — error]; sleep(2s); } return Err(timeout)`. (7) Insert `PodState { namespace: ns, pod_name, secret_name }` into `self.state`. (8) Return `ContainerId::new(format!("{ns}/{pod_name}"))`.
  - Verify: The integration test `test_k8s_provision_exec_teardown` can be partially verified by running provision manually — but full automated verification comes in T04. For now: `cargo test -p smelt-core` still passes (no regressions in existing tests); `cargo build -p smelt-core` compiles without errors
  - Done when: `cargo build -p smelt-core` compiles cleanly; the `provision()` signature matches `RuntimeProvider` trait; readiness logic handles init-done, init-failed, image-pull-error, and timeout paths (review the code paths manually)

- [x] **T03: Implement exec(), exec_streaming(), and collect() no-op** `est:45m`
  - Why: Retires the `kube` exec WebSocket risk — these methods are the core of why the K8s exec path is different from Docker; proves `AttachedProcess` works in this codebase
  - Files: `crates/smelt-core/src/k8s.rs`
  - Do: (1) Add imports: `use tokio::io::AsyncReadExt;`. (2) Helper `fn parse_container_id(id: &ContainerId) -> crate::Result<(String, String)>`: split `id.as_str()` on `'/'` to get `(namespace, pod_name)` — error if wrong format. (3) Implement `exec()`: call `parse_container_id`; look up `PodState` from `self.state`; build `AttachParams { stdout: true, stderr: true, stdin: false, tty: false, ..Default::default() }`; call `pods_api.exec(&pod_name, command, &ap).await?`; immediately call `let status_fut = attached.take_status().unwrap()` (must happen before reading streams); use `if let Some(mut stdout) = attached.stdout() { stdout.read_to_end(&mut stdout_buf).await? }` and same for stderr; call `attached.join().await?`; await `status_fut`; extract exit code via `status.as_ref().and_then(|s| s.code).unwrap_or(-1)`; return `ExecHandle { container: container.clone(), exec_id: format!("{pod_name}-exec"), exit_code, stdout: String::from_utf8_lossy(&stdout_buf).into_owned(), stderr: String::from_utf8_lossy(&stderr_buf).into_owned() }`. (4) Implement `exec_streaming()`: same setup through `take_status()`; use `tokio::join!` to concurrently read stdout and stderr in chunk loops (`let n = stdout.read(&mut buf).await?; if n == 0 { break }; let chunk = std::str::from_utf8(&buf[..n]).unwrap_or(""); output_cb(chunk); stdout_buf.extend_from_slice(&buf[..n])`); same for stderr in join branch; call `attached.join().await?`; await status_fut; build ExecHandle same as exec(). (5) Implement `collect()` no-op: return `Ok(CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] })`.
  - Verify: `cargo build -p smelt-core` compiles; `cargo test -p smelt-core` all pass; inspect that `take_status()` is called before stream reads (review code — this is the critical ordering pitfall)
  - Done when: `cargo build -p smelt-core` clean; all 3 methods compile and match the `RuntimeProvider` trait signatures; `take_status()` is called before stdout/stderr reads in both exec methods; `exec_streaming` satisfies `F: FnMut(&str) + Send + 'static` bound (D049)

- [x] **T04: Implement teardown(), complete integration tests, verify full lifecycle** `est:60m`
  - Why: Closes the lifecycle loop; makes the integration tests green; retires both high risks (exec WebSocket + readiness) with real cluster proof; verifies SSH file permissions and slow-init readiness
  - Files: `crates/smelt-core/src/k8s.rs`, `crates/smelt-cli/tests/k8s_lifecycle.rs`
  - Do: (1) Implement `teardown()`: `parse_container_id` to get namespace + pod_name; look up `PodState` from `self.state` (use pod_name to find secret_name, or parse from ContainerId format); `match pods_api.delete(&pod_name, &DeleteParams::default()).await { Ok(_) => {}, Err(kube::Error::Api(s)) if s.is_not_found() => {}, Err(e) => warn!("pod delete non-fatal: {e}") }`; same for `secrets_api.delete(&secret_name, ...)`; remove from `self.state`. (2) Fill in `test_k8s_provision_exec_teardown`: set `SMELT_TEST_SSH_KEY` env for test; call `k8s_provider_or_skip()`; call `provision()`; assert `container.as_str()` contains "smelt-"; call `exec(&container, &["echo", "hello"])` (convert to `Vec<String>`); assert `handle.exit_code == 0`; assert `handle.stdout.contains("hello")`; call `teardown()`; verify pod gone with kubectl check or re-provision to confirm no name collision. (3) Fill in `test_k8s_exec_streaming_callback`: provision; create `Arc<Mutex<Vec<String>>>` accumulator; call `exec_streaming` with callback that pushes chunks; assert accumulator non-empty after exec. (4) Fill in `test_k8s_ssh_file_permissions`: provision; exec `["stat", "/root/.ssh/id_rsa"]`; assert output contains `0400` or `----------`; teardown. (5) Fill in `test_k8s_readiness_with_slow_init`: this test provisions a pod where init container is modified to sleep 3s then exit — but since we can't modify `generate_pod_spec()` without a manifest change, use a manifest with a repo that causes a realistic init delay; alternatively, verify that provision() returns only after the main container is Running by checking Pod status after provision returns; teardown. (6) Add pre-clean helper `pre_clean_k8s(namespace, job_name)` that calls `kubectl delete pod smelt-<job-name> --ignore-not-found -n <ns>` and similar for secret — guards against orphans from prior runs (D041 pattern). (7) Run full test suite.
  - Verify: `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored 2>&1` shows 4 passed; `cargo test --workspace` shows 0 failures; `kubectl get pods -n smelt` shows no smelt-* pods; `kubectl get secrets -n smelt` shows no smelt-ssh-* secrets
  - Done when: All 4 integration tests pass with `SMELT_K8S_TEST=1`; `cargo test --workspace` green; teardown confirmed by kubectl inspection showing clean namespace

## Files Likely Touched

- `crates/smelt-core/Cargo.toml` — add `features = ["runtime", "ws"]` to kube dep
- `Cargo.toml` (workspace root) — add `"io-util"` to tokio workspace features
- `crates/smelt-core/src/k8s.rs` — full `KubernetesProvider` implementation
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — new integration test file
