---
estimated_steps: 7
estimated_files: 2
---

# T04: Implement teardown(), complete integration tests, verify full lifecycle against kind

**Slice:** S02 — KubernetesProvider Lifecycle
**Milestone:** M005

## Description

Completes the `KubernetesProvider` lifecycle by implementing `teardown()` and populating the 4 integration tests in `k8s_lifecycle.rs` that prove all methods work against a real kind cluster. This task retires both high risks from the roadmap: exec WebSocket (via `test_k8s_provision_exec_teardown` + `test_k8s_exec_streaming_callback`) and Pod readiness detection (via `test_k8s_readiness_confirmed`). The SSH file permission risk is retired by `test_k8s_ssh_file_permissions`.

Pre-condition: a kind cluster is running locally and `SMELT_K8S_TEST=1` is set. The `smelt` namespace must exist in the cluster (`kubectl create ns smelt` if needed).

## Steps

1. Implement `teardown()` in `k8s.rs`: `parse_container_id(container)` → `(ns, pod_name)`; look up `PodState` from `self.state` lock (use `pod_name` key or derive `secret_name = format!("smelt-ssh-{}", pod_name.trim_start_matches("smelt-"))`); `let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &ns); match pods_api.delete(&pod_name, &DeleteParams::default()).await { Ok(_) => {}, Err(kube::Error::Api(s)) if s.is_not_found() => {}, Err(e) => warn!("pod delete non-fatal: {e}") }`; same for `secrets_api.delete(&secret_name, ...)`; remove entry from `self.state`.
2. Add pre-clean helper to `k8s_lifecycle.rs`: `fn pre_clean_k8s(namespace: &str, job_name: &str)` — runs `kubectl delete pod smelt-{job_name} --ignore-not-found -n {namespace}` and `kubectl delete secret smelt-ssh-{job_name} --ignore-not-found -n {namespace}` as blocking subprocesses; tolerates errors silently. Call this at the start of each test before provisioning.
3. Fill in `test_k8s_provision_exec_teardown`: set `SMELT_TEST_SSH_KEY` to a real or test SSH private key (read from env or skip if not set); call `k8s_provider_or_skip()`; `pre_clean_k8s("smelt", "smelt-test")`; `provider.provision(&k8s_manifest()).await` — assert `Ok`, assert `container.as_str().starts_with("smelt/")`; `provider.exec(&container, &["sh", "-c", "echo hello"].map(str::to_string).to_vec()).await` — assert exit_code == 0, assert stdout contains "hello"; `provider.teardown(&container).await` — assert `Ok`; verify cleanup: run `kubectl get pod smelt-smelt-test -n smelt` and expect failure (not found).
4. Fill in `test_k8s_exec_streaming_callback`: `k8s_provider_or_skip()`; `pre_clean_k8s`; provision; `let chunks = Arc::new(Mutex::new(Vec::<String>::new())); let chunks_clone = Arc::clone(&chunks); provider.exec_streaming(&container, &["echo", "streaming-hello"].map(str::to_string).to_vec(), move |s| { chunks_clone.lock().unwrap().push(s.to_string()); }).await` — assert Ok, assert chunks not empty, assert chunks joined contains "streaming-hello"; teardown.
5. Fill in `test_k8s_ssh_file_permissions`: `k8s_provider_or_skip()`; `pre_clean_k8s`; provision; `provider.exec(&container, &["stat", "/root/.ssh/id_rsa"].map(str::to_string).to_vec()).await` — assert exit_code == 0; assert `handle.stdout` contains `"0400"` or the permission bits indicating user-read-only (stat output format: `File: /root/.ssh/id_rsa ... Access: (0400/-r--------)`); teardown.
6. Fill in `test_k8s_readiness_confirmed`: `k8s_provider_or_skip()`; `pre_clean_k8s`; call provision and record the time before and after; assert provision returns `Ok` (meaning it only returned after init + main were ready); call `exec` with `["sh", "-c", "echo ready"]` — if the main container were not Running, exec would fail — assert exit_code == 0; teardown.
7. Run `cargo test --workspace` (without SMELT_K8S_TEST) to confirm zero regressions; then `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored` and confirm 4 passed; verify namespace clean with kubectl.

## Must-Haves

- [ ] `teardown()` deletes Pod and SSH Secret; tolerates 404 (already gone) via `kube::Error::Api(s) if s.is_not_found()`; logs non-fatal errors with `warn!` (D023)
- [ ] `teardown()` removes the `PodState` entry from `self.state`
- [ ] `test_k8s_provision_exec_teardown`: provision returns `ContainerId` with correct format; exec `echo hello` exits 0 with "hello" in stdout; teardown confirms pod absent from cluster
- [ ] `test_k8s_exec_streaming_callback`: callback accumulator receives at least one chunk; joined content contains the echoed string
- [ ] `test_k8s_ssh_file_permissions`: `stat /root/.ssh/id_rsa` output confirms mode 0400 (user-read-only) — proves the `defaultMode: 256` from S01 is respected at runtime
- [ ] `test_k8s_readiness_confirmed`: provision() returns Ok only after the main container is Running (exec succeeds immediately after provision)
- [ ] All 4 tests are gated by `k8s_provider_or_skip()` — return early (not fail) when `SMELT_K8S_TEST` unset
- [ ] `cargo test --workspace` (no SMELT_K8S_TEST) — 0 failures, integration tests skipped
- [ ] `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored` — 4 passed

## Verification

- `cargo test --workspace` — all green, 0 failures (run without SMELT_K8S_TEST)
- `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored 2>&1 | tail -20` — shows "4 passed"
- `kubectl get pods -n smelt` after tests — no smelt-* pods remain
- `kubectl get secrets -n smelt` after tests — no smelt-ssh-* secrets remain

## Observability Impact

- Signals added/changed: `teardown()` adds `warn!("pod delete non-fatal: {e}")` and `warn!("secret delete non-fatal: {e}")` for non-404 errors; `info!("teardown complete", pod = %pod_name)` on success
- How a future agent inspects this: `kubectl get pods -n smelt` and `kubectl get secrets -n smelt` are the definitive namespace-cleanliness checks; test output shows which of the 4 tests passed/failed with assertion messages
- Failure state exposed: test failure messages include ContainerId, exit codes, stdout content, and kubectl verification output; `warn!` logs surface non-fatal teardown errors without hiding them

## Inputs

- `crates/smelt-core/src/k8s.rs` — all 4 methods implemented except `teardown()` (T01–T03)
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — 4 `#[ignore]` stubs with `k8s_provider_or_skip()` and `k8s_manifest()` helpers (T01)
- A running kind cluster and `SMELT_K8S_TEST=1` for integration verification; `SMELT_TEST_SSH_KEY` set to an SSH private key that has push access to the test repo (or a key whose format is valid even if the repo doesn't exist — for permission-only testing)
- S02 Research: `is_not_found()` helper; teardown pattern; D023 (teardown guarantee)

## Expected Output

- `crates/smelt-core/src/k8s.rs` — `teardown()` fully implemented; all 5 `RuntimeProvider` methods no longer `todo!()`
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — 4 tests populated and passing against kind cluster; `pre_clean_k8s()` helper
- Namespace `smelt` in kind cluster clean after tests run
