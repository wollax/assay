# S02: KubernetesProvider Lifecycle — Research

**Date:** 2026-03-23
**Domain:** Kubernetes / `kube` crate (3.1.0) / `k8s-openapi` (0.27.1)
**Confidence:** HIGH — all APIs confirmed by reading actual crate source in the Cargo registry

## Summary

S02 implements the full `KubernetesProvider: RuntimeProvider` — replacing the S01 stub — plus integration tests against a real kind cluster. The high risks from the roadmap are `kube` exec WebSocket (`AttachedProcess`) and Pod readiness detection. Both are now fully understood from reading the crate source, and both are lower risk than originally feared.

**`AttachedProcess` is straightforward to adapt to the `FnMut(&str)` callback model.** The `ws` feature adds `Api::<Pod>::exec()` returning `AttachedProcess`. That object exposes typed `AsyncRead` streams for stdout/stderr (`attached.stdout()`, `attached.stderr()`) and `take_status()` for an async `Status` future. Exit code is in `status.code: Option<i32>`. Streaming is implemented by `tokio::io::AsyncReadExt::read_buf()` on the streams in a loop, calling the callback per chunk before buffering. No `tokio-util::ReaderStream` is needed (avoids adding a direct dep).

**Pod readiness polling is well-defined.** K8s API returns `Pod.status.init_container_statuses[n].state.terminated` when init done, and `container_statuses[n].state.running` when the main container is live. The polling loop simply polls `Api::<Pod>::get()` every 2s up to 60 iterations (120s total), matching D081 precedent.

**Secret creation is `Api::<Secret>::namespaced(client, ns).create(&PostParams::default(), &secret_obj)`.** Secret data uses `BTreeMap<String, ByteString>` where `ByteString(Vec<u8>)` holds raw bytes. Key name is `"id_rsa"` matching the volume mount in `generate_pod_spec()`. Teardown uses `api.delete(name, &DeleteParams::default())` and tolerates `kube::Error::Api(s) if s.is_not_found()` per D023.

**Client construction with optional context** uses `kube::Config::from_kubeconfig(&KubeConfigOptions { context: manifest.kubernetes.as_ref().and_then(|k| k.context.clone()), ..Default::default() })` when context is `Some(_)`, and `kube::Client::try_default().await?` for ambient kubeconfig when context is `None`. Both paths return `kube::Client`.

**`KubernetesProvider` state** follows the `ComposeProvider` pattern: `state: Arc<Mutex<HashMap<ContainerId, PodState>>>` where `PodState { namespace: String, pod_name: String, secret_name: String }`. The `ContainerId` is `<namespace>/<pod-name>`.

## Recommendation

Implement in this order: (1) `provision()` — Secret creation + Pod creation + readiness poll; (2) `exec()` — WebSocket attach, read stdout/stderr, await status; (3) `exec_streaming()` — same WebSocket pattern but call callback per chunk; (4) `teardown()` — delete Pod + Secret idempotently; (5) integration tests tagged `#[ignore]` with `SMELT_K8S_TEST=1` guard. Add `features = ["ws"]` to the kube dep in `smelt-core/Cargo.toml` as the first code change.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Kubernetes client construction | `kube::Client::try_default()` / `kube::Config::from_kubeconfig()` | Handles all kubeconfig formats, in-cluster config, auth plugins |
| Pod exec via WebSocket | `Api::<Pod>::exec()` (behind `ws` feature) returning `AttachedProcess` | The only correct way; WebSocket multiplexed channels 0–4 per K8s exec protocol |
| Tolerate 404 on delete | `kube::Error::Api(s) if s.is_not_found()` using `Status::is_not_found()` | `Status::reason_or_code("NotFound", 404)` — clean API, no string parsing needed |
| Secret data encoding | `k8s_openapi::ByteString(Vec<u8>)` in `BTreeMap` | K8s API accepts raw bytes; the crate handles base64 encoding on serialize |

## Existing Code and Patterns

- `crates/smelt-core/src/compose.rs` — `ComposeProvider` with `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` is the exact pattern to follow for `PodState`; `provision()` builds the project state and inserts it; `teardown()` removes it and does cleanup
- `crates/smelt-core/src/docker.rs` — `exec()` loop over `LogOutput` stream and `inspect_exec` for exit code; `exec_streaming()` calls `output_cb` per chunk — K8s version uses the same pattern but with `AsyncRead` streams instead of bollard streams
- `crates/smelt-core/src/error.rs` — `SmeltError::provider_with_source()` for wrapping `kube::Error`; `SmeltError::provider()` for string-only errors
- `crates/smelt-core/src/k8s.rs` — `generate_pod_spec()` already constructs the full Pod object; `provision()` calls this after creating the Secret; the Secret name formula `format!("smelt-ssh-{job_name}")` must match the `SecretVolumeSource` already in the Pod spec
- `crates/smelt-cli/tests/compose_lifecycle.rs` — `compose_provider_or_skip()` pattern for conditional test skip; `pre_clean_*` helper for orphan cleanup; `assert_no_*` helper for verifying teardown; apply the same pattern with `k8s_provider_or_skip()` guarded by `std::env::var("SMELT_K8S_TEST").is_ok()`

## API Reference

### Client construction
```rust
// Ambient kubeconfig (context == None):
let client = kube::Client::try_default().await.map_err(|e| ...)?;

// Explicit context:
use kube::config::{KubeConfigOptions, Kubeconfig};
let opts = KubeConfigOptions { context: Some("kind-kind".to_string()), ..Default::default() };
let config = kube::Config::from_kubeconfig(&opts).await.map_err(|e| ...)?;
let client = kube::Client::try_from(config).map_err(|e| ...)?;
```

### Secret creation
```rust
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::ByteString;
use kube::api::{Api, PostParams};
use std::collections::BTreeMap;

let mut data: BTreeMap<String, ByteString> = BTreeMap::new();
data.insert("id_rsa".to_string(), ByteString(ssh_key_bytes));

let secret = Secret {
    metadata: ObjectMeta { name: Some(secret_name.clone()), namespace: Some(ns.clone()), ..Default::default() },
    data: Some(data),
    ..Default::default()
};

let secrets_api: Api<Secret> = Api::namespaced(client.clone(), &ns);
secrets_api.create(&PostParams::default(), &secret).await?;
```

### Pod creation
```rust
let pods_api: Api<Pod> = Api::namespaced(client.clone(), &ns);
pods_api.create(&PostParams::default(), &pod).await?;
```

### Pod readiness polling
```rust
loop {
    let pod = pods_api.get(&pod_name).await?;
    let status = pod.status.as_ref();

    // Check init container complete
    let init_done = status
        .and_then(|s| s.init_container_statuses.as_ref())
        .and_then(|cs| cs.iter().find(|c| c.name == "git-clone"))
        .and_then(|c| c.state.as_ref())
        .and_then(|s| s.terminated.as_ref())
        .map(|t| t.exit_code == 0)
        .unwrap_or(false);

    // Check init container failed
    let init_failed = status
        .and_then(|s| s.init_container_statuses.as_ref())
        .and_then(|cs| cs.iter().find(|c| c.name == "git-clone"))
        .and_then(|c| c.state.as_ref())
        .and_then(|s| s.terminated.as_ref())
        .map(|t| t.exit_code != 0)
        .unwrap_or(false);

    // Check main container running
    let main_running = status
        .and_then(|s| s.container_statuses.as_ref())
        .and_then(|cs| cs.iter().find(|c| c.name == "smelt-agent"))
        .and_then(|c| c.state.as_ref())
        .and_then(|s| s.running.as_ref())
        .is_some();

    if init_done && main_running { break; }
    if init_failed { return Err(...) }

    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

### Exec (buffered)
```rust
use kube::api::AttachParams;
use tokio::io::AsyncReadExt;

let ap = AttachParams::interactive_tty()   // NO — use non-interactive for scripted exec
    .stderr(true)
    .stdout(true);
// Correct: non-interactive
let ap = AttachParams { stdout: true, stderr: true, ..Default::default() };

let mut attached = pods_api.exec(&pod_name, command, &ap).await?;
let status_fut = attached.take_status().unwrap(); // must take before reading streams

let mut stdout_buf = Vec::new();
let mut stderr_buf = Vec::new();

if let Some(mut stdout) = attached.stdout() {
    stdout.read_to_end(&mut stdout_buf).await?;
}
if let Some(mut stderr) = attached.stderr() {
    stderr.read_to_end(&mut stderr_buf).await?;
}

attached.join().await?; // wait for WebSocket task to complete
let status = status_fut.await; // Option<Status>
let exit_code = status.as_ref().and_then(|s| s.code).unwrap_or(-1);
// Or check: status.map(|s| s.status.as_deref() == Some("Success")).unwrap_or(false)
```

**Important:** `status.code` in the K8s `Status` object holds the exit code for exec. `status.status` is `"Success"` or `"Failure"`. `status.reason` is `"NonZeroExitCode"` on failure. Use `status.code.unwrap_or(-1)` as the `ExecHandle.exit_code`.

### Exec (streaming)
Same as buffered, but instead of `read_to_end`, use a loop:
```rust
let mut buf = [0u8; 4096];
loop {
    let n = stdout.read(&mut buf).await?;
    if n == 0 { break; }
    let chunk = std::str::from_utf8(&buf[..n]).unwrap_or("");
    output_cb(chunk);
    stdout_buf.extend_from_slice(&buf[..n]);
}
```
Run stdout and stderr reads concurrently via `tokio::join!` or `futures::future::join`.

### Teardown (idempotent)
```rust
match pods_api.delete(&pod_name, &DeleteParams::default()).await {
    Ok(_) => {}
    Err(kube::Error::Api(s)) if s.is_not_found() => {} // already gone — fine
    Err(e) => warn!("pod delete error (non-fatal): {e}"),
}
// Same for secrets_api.delete(&secret_name, ...)
```

### Exec exit code from Status
The K8s `Status` type (from `k8s_openapi::apimachinery::pkg::apis::meta::v1`) has:
- `status: Option<String>` — `"Success"` or `"Failure"`
- `reason: Option<String>` — `"NonZeroExitCode"` when command exits non-zero
- `code: Option<i32>` — the actual exit code

This is **different from container status** (`ContainerStateTerminated.exit_code: i32` for the agent container after it exits). For `exec()`, use the `Status` from `take_status()`; for post-Assay container termination, poll `container_statuses`.

## Constraints

- `kube = { version = "3", features = ["ws"] }` is **required** for `Api::<Pod>::exec()` — currently absent from `smelt-core/Cargo.toml`; first code change
- `tokio` in `smelt-core` lacks the `"io-util"` feature which provides `AsyncReadExt::read_to_end()`. Add `"io-util"` to the workspace `tokio` features in `Cargo.toml`, OR declare `tokio = { version = "1", features = ["io-util"] }` directly in `smelt-core`. The `"io-util"` feature is **not** implied by `"macros"` or `"rt-multi-thread"`.
- `tokio-util` is a transitive dep via `kube-client` but not a direct dep of `smelt-core`. To use `tokio_util::io::ReaderStream`, add it directly. **Preferred alternative:** use bare `AsyncReadExt::read()` in a loop to avoid the dep (see streaming pattern above).
- `RPITIT (D019)` — `KubernetesProvider` must use `async fn` in trait impl, not `async_trait`. The S01 stub already does this; follow the same pattern.
- `D023` — teardown must be called in both success and error paths; K8s teardown deletes both the Pod and SSH Secret; 404 (already gone) is not an error
- `D049` — `exec_streaming` callback bound is `FnMut(&str) + Send + 'static`; the streaming impl must satisfy this without lifetime issues

## Common Pitfalls

- **`take_status()` must be called before reading stdout/stderr streams** — `take_status()` takes ownership of the `status_rx` channel inside `AttachedProcess`. If you drop `attached` without calling `take_status()`, the status is lost. Call it immediately after `pods_api.exec()` returns, before any `await`.
- **Stdout/stderr streams are `Option<impl AsyncRead>`** — they return `None` if the corresponding param (`stdout: true`, `stderr: true`) was not set in `AttachParams`. Always use `AttachParams { stdout: true, stderr: true, ..Default::default() }` for non-interactive exec; never `AttachParams::interactive_tty()` (that sets `tty: true` which breaks binary output).
- **`attached.join()` must be awaited after streams are drained** — the background WebSocket task only completes after streams are read to EOF. Join without draining causes a deadlock. Drain stdout/stderr first, then join.
- **`defaultMode: Some(256)` in `generate_pod_spec()`** — already set in S01. Do NOT re-create the Secret with a different key name — `SecretVolumeSource.secret_name` is `format!("smelt-ssh-{job_name}")` and must exactly match the Secret the `provision()` creates.
- **SSH key in container needs to be user-only mode** — the Secret volume mounts with `defaultMode: 256` (0o400), but the SSH host key check (`StrictHostKeyChecking`) will also fail for unknown hosts. The init container clone command must include `ssh -o StrictHostKeyChecking=no` or set `GIT_SSH_COMMAND`. Use: `git clone -c core.sshCommand='ssh -o StrictHostKeyChecking=no -i /root/.ssh/id_rsa' <repo> /workspace`
- **K8s `Status` vs `ContainerState` exit code** — for `exec()`, use `Status.code`; for detecting that the *agent container itself* terminated (if polling that later), use `ContainerStateTerminated.exit_code`. Don't conflate them.
- **Pod name and Secret name must match S01's formulas** — `pod_name = format!("smelt-{job_name}")`, `secret_name = format!("smelt-ssh-{job_name}")`. These are already baked into `generate_pod_spec()` in k8s.rs. If they diverge, the Pod can't find the Secret at mount time.
- **`KubeConfigOptions` context field is `Option<String>`** — passing `None` falls back to `current_context` in the kubeconfig file, which is the correct ambient behavior when `manifest.kubernetes.context` is `None`.
- **`ContainerId` format is `<namespace>/<pod-name>`** — parse it as `parts = id.split('/').collect()` in `teardown()` to recover namespace and pod name; store `PodState` in the HashMap keyed by ContainerId to avoid repeated parsing.

## Open Risks

- **SSH `StrictHostKeyChecking` in init container** — the `alpine/git` image has no pre-populated `known_hosts` for GitHub or other forges. The init container's `git clone` will hang or fail unless `StrictHostKeyChecking=no` is set via `GIT_SSH_COMMAND` or `ssh_config`. This risk is retired in S02 integration tests by confirming the clone succeeds against a kind-local bare repo (no external host key issue) and/or a real git remote.
- **Kind cluster availability in CI** — integration tests must be tagged `#[ignore]` and skip when `SMELT_K8S_TEST` is not set. The `k8s_provider_or_skip()` function must return `None` gracefully when neither kind nor `SMELT_K8S_TEST=1` is present. This keeps `cargo test --workspace` green.
- **Exec timeout** — long-running Assay sessions inside a Pod could hold the WebSocket open for minutes. No timeout is currently modeled for exec in the K8s path. For S02, this is acceptable (exec tests use `echo hello` which is instant). Real Assay sessions are handled by S04 wiring and the existing `run_with_cancellation()` timeout (D035/D037).
- **Container image pull on kind** — if the kind cluster doesn't have the agent image locally, the main container will be `Waiting` with `reason: ErrImagePull` or `ImagePullBackOff`. The readiness loop should detect this via `container_statuses[n].state.waiting.reason` and fail fast instead of waiting 120s.
- **Secret type** — creating a Secret without `type_: Some("kubernetes.io/ssh-auth")` is fine for mounting; the SSH type is just a convention. But some clusters with admission controllers might reject opaque Secrets for SSH. S02 tests will surface this. If needed, set `type_: Some("kubernetes.io/ssh-auth")` and key name to `"ssh-privatekey"` — but that requires updating `generate_pod_spec()`'s `KeyToPath.key` from `"id_rsa"` to `"ssh-privatekey"`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Kubernetes / kube-rs | (searched) | none found |

## Sources

- `kube-client-3.1.0/src/api/remote_command.rs` — full `AttachedProcess` impl: DuplexStream for stdin/stdout/stderr, `take_status()` returning `Status`, WebSocket message loop with channel demux (confirmed by direct source read)
- `kube-client-3.1.0/src/api/subresource.rs` — `Api::<Pod>::exec()` signature (behind `ws` feature), `AttachParams` struct, `Execute` trait impl for Pod
- `kube-client-3.1.0/src/lib.rs` — doc example showing `take_status()` + status assertion pattern and `stdin_writer` + stdout read loop
- `k8s-openapi-0.27.1/src/v1_32/apimachinery/pkg/apis/meta/v1/status.rs` — `Status` struct fields: `code: Option<i32>`, `status: Option<String>`, `reason: Option<String>`
- `k8s-openapi-0.27.1/src/v1_32/api/core/v1/container_state.rs` — `ContainerState { running, terminated, waiting }` — one field is `Some`, others `None`
- `k8s-openapi-0.27.1/src/v1_32/api/core/v1/container_state_terminated.rs` — `exit_code: i32` (not Option) — always present on terminated containers
- `k8s-openapi-0.27.1/src/v1_32/api/core/v1/pod_status.rs` — `init_container_statuses`, `container_statuses` fields
- `kube-core-3.1.0/src/response.rs` — `Status::is_not_found()` helper for 404-tolerant delete
- `kube-client-3.1.0/src/config/file_loader.rs` — `KubeConfigOptions { context, cluster, user }` for context-specific client construction
