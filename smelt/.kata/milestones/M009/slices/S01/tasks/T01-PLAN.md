---
estimated_steps: 5
estimated_files: 5
---

# T01: Fix broken intra-doc link and add doc comments to serve/ module public items

**Slice:** S01 — cargo doc zero-warning + deny(missing_docs) on smelt-cli
**Milestone:** M009

## Description

This task fixes the only `cargo doc` warning (unresolved intra-doc link in ssh.rs:185), adds doc comments to all undocumented public items in the serve/ module (~23 items across 5 files), and resolves the 2 `#[allow(dead_code)]` annotations in smelt-cli production code. This covers the bulk of the missing documentation work.

## Steps

1. Fix the broken intra-doc link in `serve/ssh.rs:185` — change `[build_ssh_args]` to backtick-only `` `build_ssh_args` `` per D070 (backtick-only for items that can't resolve as doc links). Run `cargo doc --workspace --no-deps` to confirm zero warnings.
2. Add `///` doc comments to all undocumented public items in `serve/config.rs`: `ServerNetworkConfig` struct + 2 fields, `ServerConfig` struct + 4 fields (including `retry_backoff_secs`). Audit `#[allow(dead_code)]` on `retry_backoff_secs` (line 75) — serde deserialization counts as a use, so remove the `#[allow]` and compile-check. If the compiler warns, add back with an updated rationale comment.
3. Add `///` doc comments to all undocumented public items in `serve/queue.rs`: `ServerState` 4 pub fields + `new()` method.
4. Add `///` doc comments to all undocumented public items in `serve/types.rs`: `JobId::new()`, `QueuedJob` 7 pub fields. Add doc comments to `serve/mod.rs`: 3 `pub mod` re-exports. Audit `#[allow(dead_code)]` on `MockSshClient::with_probe_result()` in ssh.rs:532 — this is inside `#[cfg(test)]`, verify it compiles without the annotation; keep or remove accordingly.
5. Run `cargo doc --workspace --no-deps` to confirm zero warnings. Run `cargo test --workspace` to confirm zero regressions.

## Must-Haves

- [ ] Broken intra-doc link in ssh.rs:185 fixed (backtick-only per D070)
- [ ] `cargo doc --workspace --no-deps` exits 0 with zero warnings
- [ ] All public items in serve/config.rs, serve/queue.rs, serve/ssh.rs, serve/types.rs, serve/mod.rs have `///` doc comments
- [ ] `#[allow(dead_code)]` on config.rs:75 (`retry_backoff_secs`) resolved
- [ ] `#[allow(dead_code)]` on ssh.rs:532 (`MockSshClient::with_probe_result`) resolved
- [ ] `cargo test --workspace` passes with zero regressions

## Verification

- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` outputs `0`
- `cargo test --workspace` passes (286+ tests, 0 failures)
- `grep -rn "pub " crates/smelt-cli/src/serve/ --include="*.rs" | grep -v "pub(crate)\|#\[cfg(test)\]"` — every listed item has a preceding `///` line

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `cargo doc --workspace --no-deps` warnings
- Failure state exposed: None

## Inputs

- S01-RESEARCH.md gap analysis — exact file/line/item list for all undocumented items
- D070 — backtick-only convention for items that can't resolve as doc links

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — broken link fixed, `#[allow(dead_code)]` on mock resolved
- `crates/smelt-cli/src/serve/config.rs` — all items documented, `#[allow(dead_code)]` on retry_backoff_secs resolved
- `crates/smelt-cli/src/serve/queue.rs` — all items documented
- `crates/smelt-cli/src/serve/types.rs` — all items documented
- `crates/smelt-cli/src/serve/mod.rs` — all pub mod re-exports documented
