---
id: T02
parent: S01
milestone: M009
provides:
  - "#![deny(missing_docs)] lint gate on smelt-cli"
  - Doc comments on pub mod commands and pub mod serve in lib.rs
  - Doc comments on all 6 pub mod re-exports in commands/mod.rs
  - Doc comments on JobSource variants, JobStatus variants, SshOutput fields
  - Audited smelt-core PodState #[allow(dead_code)] — justified with updated rationale
key_files:
  - crates/smelt-cli/src/lib.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-core/src/k8s.rs
key_decisions:
  - "PodState #[allow(dead_code)] kept — namespace and pod_name are stored for future exec/collect use; only secret_name is read today. Comment updated to reflect actual status."
  - "tests/docker_lifecycle.rs does not exist as a source file — only build artifacts in target/. No audit needed."
patterns_established:
  - "#![deny(missing_docs)] in lib.rs as self-enforcing lint gate for all future public items in smelt-cli"
observability_surfaces:
  - "cargo build -p smelt-cli will fail on any future undocumented public item — the lint is self-enforcing"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Added doc comments to lib.rs, commands/, audited smelt-core annotations, and enabled deny(missing_docs)

**Enabled `#![deny(missing_docs)]` on smelt-cli, documented all remaining public items (lib.rs, commands/mod.rs, types.rs variants, ssh.rs fields), and audited smelt-core `#[allow(dead_code)]`.**

## What Happened

Added `///` doc comments to `pub mod commands` and `pub mod serve` in lib.rs, and all 6 `pub mod` re-exports in commands/mod.rs (init, list, run, serve, status, watch). Added `#![deny(missing_docs)]` to lib.rs. The compiler then surfaced 10 additional undocumented items: JobSource variants (2), JobStatus variants (6), and SshOutput struct fields (2). All were documented.

Audited smelt-core's `#[allow(dead_code)]` on PodState in k8s.rs. Removed it to test — compiler confirmed `namespace` and `pod_name` fields are never read (only `secret_name` is consumed in `teardown`). Restored the `#[allow]` with an updated rationale reflecting the actual state: fields stored for future exec/collect methods, not yet consumed.

The task plan referenced `tests/docker_lifecycle.rs:133` for a second audit — this file does not exist as source code (only build artifacts in `target/`). No action needed.

## Verification

| Check | Result |
|-------|--------|
| `grep 'deny(missing_docs)' crates/smelt-cli/src/lib.rs` | ✓ matches |
| `cargo build -p smelt-cli` | ✓ 0 warnings, 0 errors |
| `cargo doc --workspace --no-deps` | ✓ 0 warnings |
| `cargo test --workspace` | ✓ 286 passed, 0 failed |
| `cargo clippy --workspace -- -D warnings` | ✗ 16 pre-existing errors in smelt-core (compose.rs, k8s.rs) — identical on main branch |
| `grep -rn '#[allow(dead_code)]' crates/smelt-cli/src/` | ✓ only retry_backoff_secs (serde forward-compat, justified in T01) |

## Diagnostics

`cargo build -p smelt-cli` will fail on any future undocumented public item — the lint is self-enforcing. No runtime diagnostics; this is compile-time lint work.

## Deviations

- `tests/docker_lifecycle.rs` does not exist as source — plan referenced a non-existent file. No audit was possible or needed.
- `cargo clippy --workspace -- -D warnings` does not exit 0 due to 16 pre-existing collapsible-if warnings in smelt-core (compose.rs, k8s.rs). Verified identical count on main branch. These are not regressions from this slice.

## Known Issues

- Pre-existing clippy warnings in smelt-core (16 collapsible-if lint errors in compose.rs and k8s.rs) cause `cargo clippy --workspace -- -D warnings` to fail. Not introduced by this slice.

## Files Created/Modified

- `crates/smelt-cli/src/lib.rs` — Added `#![deny(missing_docs)]` and doc comments on pub mod re-exports
- `crates/smelt-cli/src/commands/mod.rs` — Added doc comments on all 6 pub mod re-exports
- `crates/smelt-cli/src/serve/types.rs` — Added doc comments on JobSource and JobStatus enum variants
- `crates/smelt-cli/src/serve/ssh.rs` — Added doc comments on SshOutput.stdout and SshOutput.stderr fields
- `crates/smelt-core/src/k8s.rs` — Updated `#[allow(dead_code)]` rationale on PodState
