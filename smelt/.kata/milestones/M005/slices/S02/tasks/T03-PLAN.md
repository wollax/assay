---
estimated_steps: 6
estimated_files: 1
---

# T03: Implement exec(), exec_streaming(), and collect() no-op

**Slice:** S02 — KubernetesProvider Lifecycle
**Milestone:** M005

## Description

Implements the three remaining `RuntimeProvider` methods that don't touch the Pod lifecycle — `exec()` (buffered WebSocket exec), `exec_streaming()` (callback-per-chunk WebSocket exec), and `collect()` (no-op for S02). This is where the `kube` exec WebSocket risk is retired at the code level — the `AttachedProcess` API is adapted to the `ExecHandle` / `FnMut(&str)` callback model.

Critical constraint: `take_status()` MUST be called before reading stdout/stderr streams — calling it after or not at all causes the status channel to be dropped, making exit codes unavailable. This is the most important pitfall in the `AttachedProcess` API.

## Steps

1. Add import: `use tokio::io::AsyncReadExt;`. Add private helper `fn parse_container_id(id: &ContainerId) -> crate::Result<(String, String)>`: split `id.as_str()` on `'/'` into `(namespace, pod_name)` — return `SmeltError::provider("k8s", format!("invalid ContainerId format: '{}'", id.as_str()))` if not exactly 2 parts.
2. Implement `exec()`: (a) `parse_container_id(container)` → `(ns, pod_name)`; (b) `let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &ns)`; (c) `let ap = AttachParams { stdout: true, stderr: true, stdin: false, tty: false, ..Default::default() }` — **NOT** `AttachParams::interactive_tty()` (that sets `tty: true` which corrupts binary output); (d) `let mut attached = pods_api.exec(&pod_name, command, &ap).await?`; (e) **immediately**: `let status_fut = attached.take_status().expect("status channel must exist for non-tty exec")` — this MUST happen before any stream reads; (f) drain stdout: `if let Some(mut out) = attached.stdout() { out.read_to_end(&mut stdout_buf).await? }`; drain stderr: same; (g) `attached.join().await?` — waits for WebSocket task to complete; (h) `let status = status_fut.await`; (i) `let exit_code = status.as_ref().and_then(|s| s.code).unwrap_or(-1)`; (j) return `ExecHandle { container: container.clone(), exec_id: format!("{pod_name}-exec"), exit_code, stdout: String::from_utf8_lossy(&stdout_buf).into_owned(), stderr: String::from_utf8_lossy(&stderr_buf).into_owned() }`.
3. Implement `exec_streaming()`: (a) same setup through `attached.take_status()` as exec(); (b) define `const BUF_SIZE: usize = 4096`; (c) run stdout and stderr read loops concurrently via `tokio::join!`: each loop does `let mut buf = [0u8; BUF_SIZE]; loop { let n = stream.read(&mut buf).await?; if n == 0 { break }; let chunk = std::str::from_utf8(&buf[..n]).unwrap_or(""); output_cb(chunk); full_buf.extend_from_slice(&buf[..n]); }`; (d) call `attached.join().await?`; await `status_fut`; extract exit code; return `ExecHandle` with full buffered output.
4. Note on exec_streaming concurrency: the `output_cb` callback is `FnMut(&str) + Send + 'static` (D049). In the `tokio::join!` pattern, both branches access `output_cb` — this won't work directly as `join!` runs branches in the same task context. Use sequential reads (stdout loop then stderr loop) instead of true concurrent `join!` to avoid the `FnMut` shared-access issue. Both loops still complete before the status is awaited, which is correct.
5. Implement `collect()` no-op: `Ok(CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] })`.
6. Run `cargo build -p smelt-core` and fix any type errors. Run `cargo test -p smelt-core` to confirm no regressions.

## Must-Haves

- [ ] `parse_container_id()` helper correctly splits `"<ns>/<pod-name>"` format
- [ ] `take_status()` is called BEFORE any stdout/stderr read in both `exec()` and `exec_streaming()`
- [ ] `AttachParams { stdout: true, stderr: true, stdin: false, tty: false, ..Default::default() }` — NOT `interactive_tty()`
- [ ] `attached.join().await?` is called AFTER streams are drained (not before)
- [ ] `exec_streaming()` callback bound satisfies `FnMut(&str) + Send + 'static` (D049) — sequential stdout then stderr loops
- [ ] `collect()` returns `Ok(CollectResult { exit_code: 0, .. })` (no-op for S02)
- [ ] `cargo build -p smelt-core` compiles cleanly
- [ ] `cargo test -p smelt-core` — all existing tests pass

## Verification

- `cargo build -p smelt-core` — no compile errors
- `cargo test -p smelt-core` — all tests pass (the exec methods have no unit tests yet; full proof is in T04 integration tests)
- Manual code review: verify `take_status()` call order; verify `tty: false` in AttachParams; verify `join()` is after stream drain

## Observability Impact

- Signals added/changed: None beyond what T02 added — exec operations don't add tracing yet (the exit code and output are in the returned ExecHandle)
- How a future agent inspects this: `ExecHandle.exit_code`, `ExecHandle.stdout`, `ExecHandle.stderr` carry the full result; integration tests in T04 assert on these fields directly; `kubectl logs <pod> -n smelt` also shows output for debugging
- Failure state exposed: `SmeltError::provider_with_source("k8s", "exec failed", e)` wraps `kube::Error` from the WebSocket attach; `parse_container_id` error makes malformed ContainerId immediately visible

## Inputs

- `crates/smelt-core/src/k8s.rs` — `KubernetesProvider` with `provision()` implemented (T02); `ws` feature active (T01); `io-util` tokio feature active (T01)
- S02 Research: `take_status()` ordering pitfall; `AttachParams` non-interactive pattern; sequential stdout/stderr read approach for callback
- `provider.rs` — `ExecHandle`, `CollectResult`, `ContainerId` types

## Expected Output

- `crates/smelt-core/src/k8s.rs` — `exec()`, `exec_streaming()`, `collect()` implemented; `parse_container_id()` private helper; all 5 `RuntimeProvider` methods no longer `todo!()`
