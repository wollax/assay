---
id: T02
parent: S02
milestone: M005
provides:
  - KubernetesProvider::provision() fully implemented in crates/smelt-core/src/k8s.rs
  - SSH Secret created with key "id_rsa" and ByteString value — matches SecretVolumeSource key in generate_pod_spec()
  - Pod created via generate_pod_spec() — secret_name=smelt-ssh-{job_name}, pod_name=smelt-{job_name}
  - Readiness poll (60×2s=120s) checking init container "git-clone" terminated.exit_code and main container "smelt-agent" state.running
  - Image pull backoff fast-fail via container_statuses[smelt-agent].state.waiting.reason (ImagePullBackOff, ErrImagePull)
  - Secret cleanup on Pod creation failure (delete before returning error)
  - PodState inserted into self.state on success; ContainerId format = "{ns}/{pod_name}"
  - tracing::info! at pod creation and readiness achieved; tracing::warn! on Secret delete rollback failure
key_files:
  - crates/smelt-core/src/k8s.rs
key_decisions:
  - "AttachParams import removed (not needed until T03) to keep build warning-free"
  - "PodState retains #[allow(dead_code)] — fields are written in provision() but read by T03 (exec) and T04 (teardown)"
patterns_established:
  - "provision() uses if-let Err rollback pattern for Secret cleanup — delete on Pod creation failure, ignore 404 via warn!"
observability_surfaces:
  - "tracing::info!(pod, namespace) at 'pod created, polling readiness' and 'pod ready'"
  - "tracing::warn!(secret, namespace, error) on Secret delete rollback failure"
  - "SmeltError::Provider carries operation ('k8s'), pod name in timeout errors, init exit_code in init-failure errors, image pull reason in pull-backoff errors"
  - "kubectl describe pod smelt-<name> -n <ns> — readiness status and init container exit code"
  - "kubectl logs smelt-<name> -c git-clone -n <ns> — init container stderr (git clone errors)"
  - "RUST_LOG=smelt_core=debug cargo test — provision lifecycle trace output"
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Implement provision() — SSH Secret + Pod creation + readiness polling

**`KubernetesProvider::provision()` fully implemented: reads SSH key from env, creates Secret, creates Pod via generate_pod_spec(), polls readiness with init-done + main-running + image-pull-backoff fast-fail + 120s timeout**

## What Happened

Implemented `KubernetesProvider::provision()` in `crates/smelt-core/src/k8s.rs`. Added imports: `Secret`, `ByteString`, `Duration`, `tokio::time::sleep`, `tracing::{info, warn}`.

The implementation follows the plan exactly:

1. Extracts `kube_cfg`, `ns`, `job_name`, derives `pod_name = format!("smelt-{job_name}")` and `secret_name = format!("smelt-ssh-{job_name}")` — identical formulas to those baked into `generate_pod_spec()`.
2. Reads the SSH private key from `kube_cfg.ssh_key_env` via `std::env::var`, returning `SmeltError::Provider` (not panic) if absent.
3. Creates a `Secret` with `BTreeMap<String, ByteString>` containing `"id_rsa"` key — matches the `SecretVolumeSource` key in S01's `generate_pod_spec()`.
4. On Pod creation failure, deletes the Secret (cleanup before returning error). Delete errors are `warn!`-logged but not propagated.
5. Polls readiness in a `for _ in 0..60u32` loop (60 × 2s = 120s):
   - Checks `init_container_statuses[name=="git-clone"].state.terminated.exit_code` — `0` sets `init_done = true`, non-zero returns immediate `Err`.
   - Checks `container_statuses[name=="smelt-agent"].state.running.is_some()` — sets `main_running = true`.
   - Checks `container_statuses[name=="smelt-agent"].state.waiting.reason` for `"ImagePullBackOff"` or `"ErrImagePull"` — returns immediate `Err`.
   - Breaks when both `init_done && main_running`.
6. After loop, returns timeout `Err` if not ready, with pod name and namespace in message.
7. Inserts `PodState { namespace, pod_name, secret_name }` into `self.state`, returns `ContainerId::new(format!("{ns}/{pod_name}"))`.

## Verification

- `cargo build -p smelt-core` — ✓ PASS, no warnings
- `cargo test -p smelt-core` — ✓ PASS, 148 tests pass, 0 failures (k8s unit tests: test_generate_pod_spec_snapshot, test_generate_pod_spec_requires_kubernetes_config, test_generate_pod_spec_resource_limits all pass)
- Code review: `take_status()` NOT called; Secret cleanup on Pod creation error confirmed; init container status path uses `name == "git-clone"` matching S01; `ContainerId` format is `"{ns}/{pod_name}"` as specified

## Diagnostics

- `tracing::info!(pod, namespace)` at "pod created, polling readiness" and "pod ready" — visible with `RUST_LOG=smelt_core=debug`
- `tracing::warn!(secret, namespace, error)` on Secret delete rollback failure
- `SmeltError::Provider` shapes: env-var error includes var name; Pod creation error includes pod name; timeout error includes pod name and namespace; init failure error includes exit code; image pull error includes the waiting reason
- Inspection: `kubectl describe pod smelt-<name> -n smelt` for readiness; `kubectl logs smelt-<name> -c git-clone -n smelt` for init container output

## Deviations

None. All 8 steps executed as specified.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/k8s.rs` — provision() fully implemented; AttachParams import removed (moved to T03); PodState retains #[allow(dead_code)]
