---
estimated_steps: 8
estimated_files: 2
---

# T02: Implement provision() — SSH Secret + Pod creation + readiness polling

**Slice:** S02 — KubernetesProvider Lifecycle
**Milestone:** M005

## Description

Implements `KubernetesProvider::provision()` end-to-end: reads the SSH private key from the env var specified in `KubernetesConfig.ssh_key_env`, creates a K8s Secret from it, calls `generate_pod_spec()` to get the Pod spec, creates the Pod, and polls `Pod.status` until the init container has exited 0 AND the main container is `Running`. Handles failure paths: init container failure (non-zero exit code), image pull backoff, and timeout (120s). Retires the Pod readiness detection risk and SSH file permission risk from the roadmap.

The Secret name (`smelt-ssh-<job-name>`) and Pod name (`smelt-<job-name>`) must exactly match the names baked into `generate_pod_spec()` — these are the critical wiring points between T02 and S01.

## Steps

1. Add imports to `k8s.rs`: `use k8s_openapi::api::core::v1::Secret; use k8s_openapi::ByteString; use std::time::Duration; use tokio::time::sleep; use tracing::{info, warn};`.
2. Implement `provision()`: extract `kube_cfg = manifest.kubernetes.as_ref().ok_or_else(...)`, `ns = &kube_cfg.namespace`, `job_name = &manifest.job.name`, `pod_name = format!("smelt-{job_name}")`, `secret_name = format!("smelt-ssh-{job_name}")`.
3. Read SSH key: `let key_bytes = std::env::var(&kube_cfg.ssh_key_env).map_err(|_| SmeltError::provider("k8s", format!("env var '{}' not set or not unicode", kube_cfg.ssh_key_env)))?.into_bytes();`. Create Secret object: `BTreeMap` with `"id_rsa"` → `ByteString(key_bytes)`; `ObjectMeta` with name and namespace; create via `Api::<Secret>::namespaced(self.client.clone(), ns).create(&PostParams::default(), &secret).await` — wrap kube error with `SmeltError::provider_with_source`.
4. Call `generate_pod_spec(manifest, job_name, "")` to get the Pod object. Create via `Api::<Pod>::namespaced(self.client.clone(), ns).create(&PostParams::default(), &pod).await` — on error, attempt to delete the Secret (`secrets_api.delete(&secret_name, ...)` ignoring 404) before returning the error.
5. Log `info!(pod = %pod_name, namespace = %ns, "pod created, polling readiness")`.
6. Readiness poll loop (`for _ in 0..60u32`): fetch `pods_api.get(&pod_name).await?`; check init container state: `pod.status.init_container_statuses[name=="git-clone"].state.terminated` — if `exit_code == 0` set `init_done = true`; if `exit_code != 0` return `Err(SmeltError::provider("k8s", format!("init container failed with exit code {}", t.exit_code)))`; check main container: `container_statuses[name=="smelt-agent"].state.running.is_some()` → `main_running = true`; also check `state.waiting.reason` for `"ImagePullBackOff"` or `"ErrImagePull"` → return early error; if `init_done && main_running` break; else `sleep(Duration::from_secs(2)).await`.
7. After loop: if not `(init_done && main_running)`, return `Err(SmeltError::provider("k8s", format!("pod {pod_name} did not become ready within 120s")))`.
8. Insert `PodState { namespace: ns.to_string(), pod_name: pod_name.clone(), secret_name: secret_name.clone() }` into `self.state`. Return `Ok(ContainerId::new(format!("{ns}/{pod_name}")))`.

## Must-Haves

- [ ] `provision()` reads `ssh_key_env` from env and returns `SmeltError::Provider` (not panic) when env var is absent
- [ ] Secret is created with key `"id_rsa"` and `ByteString` value — matches the `SecretVolumeSource` key in `generate_pod_spec()` (S01 contract)
- [ ] Pod is created via `generate_pod_spec()` — Secret name and Pod name use the same formulas as S01
- [ ] Readiness poll checks `init_container_statuses[name=="git-clone"].state.terminated.exit_code` for success/failure
- [ ] Readiness poll checks `container_statuses[name=="smelt-agent"].state.running` for main container ready
- [ ] Image pull backoff detected via `state.waiting.reason` and surfaces as immediate `Err`
- [ ] Timeout (60×2s = 120s) returns descriptive error including pod name and namespace
- [ ] On Pod creation failure, Secret is deleted (cleanup before returning error)
- [ ] `PodState` inserted into `self.state` on success with `ContainerId` format `<ns>/<pod-name>`
- [ ] `cargo build -p smelt-core` compiles cleanly

## Verification

- `cargo build -p smelt-core` — no compile errors
- `cargo test -p smelt-core` — all 154+ existing tests still pass (provision is new code, no regressions)
- Manual code review: confirm `take_status()` is NOT called here (that's exec); confirm Secret cleanup on Pod creation error; confirm init container status path matches `name == "git-clone"` (matching S01's init container name)

## Observability Impact

- Signals added/changed: `tracing::info!` at pod creation and readiness achieved; `tracing::warn!` on teardown cleanup errors; `tracing::info!` with `pod = %pod_name, namespace = %ns` context
- How a future agent inspects this: `RUST_LOG=smelt_core=debug cargo test` shows provision lifecycle; `kubectl describe pod smelt-<name> -n smelt` shows readiness status and init container exit code; `kubectl logs smelt-<name> -c git-clone -n smelt` shows init container stderr (git clone errors)
- Failure state exposed: `SmeltError::Provider` carries operation context ("k8s"), the pod name in timeout errors, the init exit code in init-failure errors, and the image pull reason in pull-backoff errors

## Inputs

- `crates/smelt-core/src/k8s.rs` — `KubernetesProvider` struct with `client` and `state` from T01; `generate_pod_spec()` already produces the correct Pod object with `smelt-ssh-{job_name}` Secret reference
- S01 Forward Intelligence: Secret name must be `format!("smelt-ssh-{job_name}")` — exact match with `SecretVolumeSource` in `generate_pod_spec()`; init container name is `"git-clone"`; main container name is `"smelt-agent"`

## Expected Output

- `crates/smelt-core/src/k8s.rs` — `provision()` fully implemented: Secret creation, Pod creation, 60-iteration readiness poll with init-done + main-running check + image-pull-backoff fast-fail + timeout error
