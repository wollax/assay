---
id: T01
parent: S01
milestone: M009
provides:
  - Zero cargo-doc warnings across workspace
  - Doc comments on all public items in serve/ module (config, queue, types, ssh, mod)
  - Resolved stale #[allow(dead_code)] annotations in serve/ production and test code
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "retry_backoff_secs #[allow(dead_code)] kept — serde deserialization alone does not count as a read for the dead_code lint; field is forward-compatible config, not yet consumed by dispatch loop"
  - "MockSshClient::with_probe_result #[allow(dead_code)] removed — method is used in 12+ test call sites across dispatch.rs and tests.rs"
patterns_established:
  - "D070 backtick-only convention applied to ssh.rs intra-doc link (build_ssh_args)"
observability_surfaces:
  - none (compile-time lint work, no runtime behavior change)
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Fix broken intra-doc link and add doc comments to serve/ module public items

**Fixed the only cargo-doc warning (broken intra-doc link in ssh.rs), added `///` doc comments to all ~23 undocumented public items across 5 serve/ files, and resolved both `#[allow(dead_code)]` annotations.**

## What Happened

1. Fixed the broken intra-doc link in `ssh.rs:185` — changed `[build_ssh_args]` to backtick-only `` `build_ssh_args` `` per D070, since the function is not in scope for rustdoc link resolution.

2. Added doc comments to all undocumented public items in `config.rs`: `ServerNetworkConfig` struct + 2 fields, `ServerConfig` struct + 6 fields. Audited `#[allow(dead_code)]` on `retry_backoff_secs` — serde deserialization does NOT suppress the dead_code lint (the compiler confirmed with a warning when the allow was removed), so the annotation was kept with an updated rationale comment explaining forward-compatibility intent.

3. Added doc comments to all undocumented public items in `queue.rs`: `ServerState` 5 fields + `new()` method.

4. Added doc comments to all undocumented public items in `types.rs`: `JobId::new()`, `QueuedJob` 7 fields. Added doc comments to `mod.rs`: 3 `pub mod` re-exports (`types`, `queue`, `ssh`). Audited `#[allow(dead_code)]` on `MockSshClient::with_probe_result()` in ssh.rs — the method is used in 12+ test call sites; the annotation was unnecessary and was removed.

## Verification

- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` → `0` ✓
- `cargo build -p smelt-cli 2>&1 | grep -c warning` → `0` ✓
- `cargo test --workspace` → 155 passed, 0 failed ✓
- `cargo clippy -p smelt-cli` → pre-existing errors in smelt-core dependency (not in smelt-cli changes)
- Manual audit: all `pub` items in serve/ module files have preceding `///` lines

## Diagnostics

None — this is compile-time lint work. Future agents verify via `cargo doc --workspace --no-deps` warnings.

## Deviations

- `retry_backoff_secs`: Plan expected serde deserialization to count as a "use" for dead_code lint. It does not. The `#[allow(dead_code)]` was kept (with improved rationale comment) instead of removed.

## Known Issues

- Pre-existing clippy errors in `smelt-core` (16 errors) prevent `cargo clippy --workspace -- -D warnings` from passing. These are unrelated to this task's changes.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — fixed broken intra-doc link, removed unnecessary `#[allow(dead_code)]` on test mock
- `crates/smelt-cli/src/serve/config.rs` — added doc comments to ServerNetworkConfig, ServerConfig and all fields; kept #[allow(dead_code)] on retry_backoff_secs with updated rationale
- `crates/smelt-cli/src/serve/queue.rs` — added doc comments to ServerState fields and new() method
- `crates/smelt-cli/src/serve/types.rs` — added doc comments to JobId::new() and all QueuedJob fields
- `crates/smelt-cli/src/serve/mod.rs` — added doc comments to 3 pub mod re-exports
