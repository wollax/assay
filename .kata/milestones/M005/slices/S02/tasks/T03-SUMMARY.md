---
id: T03
parent: S02
milestone: M005
provides:
  - "`parse_container_id()` private helper — splits `\"<ns>/<pod-name>\"` ContainerId into parts"
  - "`KubernetesProvider::exec()` — buffered WebSocket exec using AttachedProcess; drains stdout+stderr into buffers, awaits Status, returns ExecHandle with correct exit_code"
  - "`KubernetesProvider::exec_streaming()` — same WebSocket exec path with FnMut(&str)+Send+'static callback per chunk (sequential stdout then stderr); full buffered output also in returned ExecHandle"
  - "`KubernetesProvider::collect()` no-op — returns empty CollectResult { exit_code: 0, .. } for S02"
  - "Critical ordering: `take_status()` called BEFORE stdout/stderr reads in both exec methods — prevents status channel drop"
key_files:
  - crates/smelt-core/src/k8s.rs
key_decisions:
  - "exec_streaming() uses sequential stdout-then-stderr loops, NOT tokio::join! — avoids FnMut shared-access across join branches (per D049 and step-4 note in task plan)"
  - "AttachParams uses { stdout: true, stderr: true, stdin: false, tty: false, ..Default::default() } — NOT interactive_tty() which sets tty:true and corrupts binary output"
  - "parse_container_id() uses splitn(2, '/') so pod names containing '/' are handled gracefully (error if not exactly 2 non-empty parts)"
patterns_established:
  - "take_status()-before-reads pattern: status future must be taken from AttachedProcess immediately after exec(), before any stdout/stderr stream reads — documented in function and plan"
observability_surfaces:
  - "ExecHandle.exit_code / .stdout / .stderr carry full result for downstream inspection"
  - "SmeltError::provider_with_source(\"k8s\", \"exec failed\", e) wraps kube::Error from WebSocket attach — includes original error chain"
  - "parse_container_id error makes malformed ContainerId immediately visible in error message"
duration: 30m
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T03: Implement exec(), exec_streaming(), and collect() no-op

**`exec()`, `exec_streaming()`, and `collect()` implemented on `KubernetesProvider` via `AttachedProcess` WebSocket API — critical `take_status()`-before-reads ordering enforced, sequential callback pattern adopted to satisfy `FnMut` bounds.**

## What Happened

Added `use kube::api::AttachParams` and `use tokio::io::AsyncReadExt` imports. Implemented a private `parse_container_id()` helper that splits the `"<namespace>/<pod-name>"` `ContainerId` format using `splitn(2, '/')` — returns `SmeltError::provider` with a descriptive message if the format is wrong.

`exec()` follows the plan exactly: builds `AttachParams` with `tty: false` (not `interactive_tty()`), calls `pods_api.exec()`, immediately calls `attached.take_status()` before touching any stream, drains stdout then stderr with `read_to_end`, calls `attached.join().await?` after streams are drained, then awaits the status future to extract the exit code (`Status.code.unwrap_or(-1)`).

`exec_streaming()` follows the same setup but reads in `4096`-byte chunks via a loop, calling the `FnMut(&str)` callback per chunk. Per the task plan (step 4), the stdout loop runs fully before the stderr loop starts — this avoids the `FnMut` shared-access problem that `tokio::join!` would create when both branches try to capture the same `&mut output_cb`. Both loops complete before `join()` and `status_fut.await`.

`collect()` is a true no-op returning `Ok(CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] })` — artifact collection is deferred to a later milestone.

## Verification

- `cargo build -p smelt-core` — compiles cleanly, no warnings
- `cargo test -p smelt-core` — 148 unit tests pass, 3 doc-tests pass, 0 failures

Manual code review confirmed:
- `take_status()` is called on line immediately after `pods_api.exec()` in both methods, before any `attached.stdout()` / `attached.stderr()` calls
- `tty: false` in `AttachParams` struct literal (not `interactive_tty()`)
- `attached.join().await?` is after the stream drain loops in both methods
- `exec_streaming` callback bound `F: FnMut(&str) + Send + 'static` matches `RuntimeProvider` trait (D049)

## Diagnostics

- `ExecHandle.exit_code`, `.stdout`, `.stderr` carry the full result — T04 integration tests will assert on these directly
- `SmeltError::provider_with_source("k8s", "exec failed", e)` wraps `kube::Error` from WebSocket attach
- `SmeltError::provider_with_source("k8s", "failed to read stdout/stderr", e)` wraps I/O errors from stream reads
- `parse_container_id` error message: `"invalid ContainerId format: '<value>' — expected '<namespace>/<pod-name>'"` makes bad ContainerId immediately visible

## Deviations

None. The step-4 note about sequential vs `tokio::join!` was followed as written — sequential stdout-then-stderr loops implemented as specified.

## Known Issues

None. Full behavioral proof (exit codes, stdout content, callback firing) requires the T04 integration tests against a real kind cluster.

## Files Created/Modified

- `crates/smelt-core/src/k8s.rs` — `parse_container_id()` helper added; `exec()`, `exec_streaming()`, `collect()` implemented; `AttachParams` and `AsyncReadExt` imports added
