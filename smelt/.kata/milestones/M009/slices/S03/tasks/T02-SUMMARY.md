---
id: T02
parent: S03
milestone: M009
provides:
  - serve/ssh/ directory module replacing monolithic 976-line ssh.rs
  - ssh/mod.rs (111 lines) — SshOutput, SshClient trait, re-exports
  - ssh/client.rs (318 lines) — SubprocessSshClient + impl SshClient
  - ssh/operations.rs (86 lines) — deliver_manifest, sync_state_back, run_remote_job
  - ssh/mock.rs (498 lines) — MockSshClient + all 14 SSH unit/integration tests
key_files:
  - crates/smelt-cli/src/serve/ssh/mod.rs
  - crates/smelt-cli/src/serve/ssh/client.rs
  - crates/smelt-cli/src/serve/ssh/operations.rs
  - crates/smelt-cli/src/serve/ssh/mock.rs
key_decisions:
  - "D130: SSH tests module re-exported via pub(crate) mod tests wrapper in mod.rs to preserve crate::serve::ssh::tests::MockSshClient import path"
patterns_established:
  - "Same flat-to-directory module conversion pattern from T01: move to mod.rs, extract child modules, re-export pub items"
  - "Test compatibility shim: pub(crate) mod tests { pub(crate) use super::mock::MockSshClient; } preserves existing import paths"
observability_surfaces:
  - none — pure refactoring
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Decomposed ssh.rs (976L) into 4-file directory module with mod.rs at 111 lines

**Converted `serve/ssh.rs` (976 lines) into a `serve/ssh/` directory module with `mod.rs` at 111 lines — well under the 400-line threshold.**

## What Happened

Decomposed the monolithic `ssh.rs` into four focused files:

- **`mod.rs` (111 lines)** — Module-level doc comment, `SshOutput` struct, `SshClient` trait, re-exports, and a `tests` compatibility shim.
- **`client.rs` (318 lines)** — `SubprocessSshClient` struct with `build_ssh_args`, `build_scp_args`, `ssh_binary`, `scp_binary`, and the full `impl SshClient for SubprocessSshClient`.
- **`operations.rs` (86 lines)** — Three generic free functions (`deliver_manifest`, `sync_state_back`, `run_remote_job`) that operate on any `C: SshClient`.
- **`mock.rs` (498 lines)** — `MockSshClient` test double and all 14 SSH tests (12 active + 2 gated `#[ignore]` integration tests), behind `#[cfg(test)]`.

A `pub(crate) mod tests` wrapper in `mod.rs` re-exports `MockSshClient` so the existing `crate::serve::ssh::tests::MockSshClient` import path used by `dispatch.rs` and `tests.rs` remains valid without any changes to those consumers.

## Deviations

None — followed the plan exactly.

## Verification

### Slice-level checks (intermediate — T02 of T04):
- `cargo test --workspace` — 286+ pass, 0 failures ✓
- `cargo doc --workspace --no-deps` — 0 warnings ✓
- `wc -l ssh/mod.rs` — 111 (< 400 threshold) ✓
- `wc -l run/mod.rs` — verified still < 300 from T01 ✓
- `wc -l serve/tests.rs` — not yet decomposed (T03)

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh/mod.rs` — NEW: trait + types + re-exports (111 lines)
- `crates/smelt-cli/src/serve/ssh/client.rs` — NEW: SubprocessSshClient implementation (318 lines)
- `crates/smelt-cli/src/serve/ssh/operations.rs` — NEW: free functions (86 lines)
- `crates/smelt-cli/src/serve/ssh/mock.rs` — NEW: MockSshClient + tests (498 lines)
- `crates/smelt-cli/src/serve/ssh.rs` — DELETED (replaced by ssh/ directory)
