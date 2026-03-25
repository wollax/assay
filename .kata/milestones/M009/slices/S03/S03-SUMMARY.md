---
id: S03
parent: M009
milestone: M009
provides:
  - commands/run/ directory module — mod.rs (116L), phases.rs, dry_run.rs, helpers.rs
  - serve/ssh/ directory module — mod.rs (111L), client.rs, operations.rs, mock.rs
  - serve/tests/ directory module — mod.rs (88L), queue.rs, dispatch.rs, http.rs, ssh_dispatch.rs, config.rs
  - All 286 workspace tests passing with zero regressions
  - Clean cargo clippy/doc/build across workspace
requires:
  - slice: S01
    provides: deny(missing_docs) baseline — all new modules compile clean under the lint
affects: []
key_files:
  - crates/smelt-cli/src/commands/run/mod.rs
  - crates/smelt-cli/src/commands/run/phases.rs
  - crates/smelt-cli/src/commands/run/dry_run.rs
  - crates/smelt-cli/src/commands/run/helpers.rs
  - crates/smelt-cli/src/serve/ssh/mod.rs
  - crates/smelt-cli/src/serve/ssh/client.rs
  - crates/smelt-cli/src/serve/ssh/operations.rs
  - crates/smelt-cli/src/serve/ssh/mock.rs
  - crates/smelt-cli/src/serve/tests/mod.rs
  - crates/smelt-cli/src/serve/tests/queue.rs
  - crates/smelt-cli/src/serve/tests/dispatch.rs
  - crates/smelt-cli/src/serve/tests/http.rs
  - crates/smelt-cli/src/serve/tests/ssh_dispatch.rs
  - crates/smelt-cli/src/serve/tests/config.rs
key_decisions:
  - "D128: File-to-directory module conversion with re-exports preserves API compatibility"
  - "D129: Tests distributed to the module containing the code they test"
  - "D130: SSH tests module re-exported via pub(crate) mod tests wrapper to preserve MockSshClient import path"
  - "D131: test_manifest_delivery_and_remote_exec moved to ssh_dispatch.rs for feature coherence"
patterns_established:
  - "Flat file → directory module conversion: move to mod.rs, extract child modules, re-export pub items via pub use"
  - "Tests co-located with implementation in each child module"
  - "Compatibility shim pattern: pub(crate) mod wrapper re-exporting from child module preserves external import paths"
observability_surfaces:
  - none — pure refactoring, no runtime changes
drill_down_paths:
  - .kata/milestones/M009/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M009/slices/S03/tasks/T03-SUMMARY.md
  - .kata/milestones/M009/slices/S03/tasks/T04-SUMMARY.md
duration: 60min
verification_result: passed
completed_at: 2026-03-24T18:00:00Z
---

# S03: Large file decomposition

**Decomposed three oversized files (run.rs 791L, ssh.rs 976L, tests.rs 1370L) into focused directory modules — all under size thresholds, 286 tests green, zero warnings**

## What Happened

Applied a consistent flat-file-to-directory-module conversion pattern across three large files in smelt-cli:

**T01 — run.rs (791L → 116L mod.rs):** Extracted `phases.rs` (full Phase 1-9 container lifecycle), `dry_run.rs` (validation + execution plan printing + truncate tests), and `helpers.rs` (gitignore guard + PR creation guard + their tests). `mod.rs` retains `RunArgs`, `AnyProvider` enum, and `execute()` entry point.

**T02 — ssh.rs (976L → 111L mod.rs):** Extracted `client.rs` (SubprocessSshClient + SshClient impl), `operations.rs` (deliver_manifest, sync_state_back, run_remote_job), and `mock.rs` (MockSshClient + all 14 SSH tests behind `#[cfg(test)]`). Added a `pub(crate) mod tests` shim in mod.rs to preserve the `crate::serve::ssh::tests::MockSshClient` import path used by dispatch.rs and tests.rs.

**T03 — tests.rs (1370L → 88L mod.rs):** Extracted tests by feature area into `queue.rs` (4), `dispatch.rs` (4), `http.rs` (7), `ssh_dispatch.rs` (6), and `config.rs` (9). Shared helpers (`VALID_MANIFEST_TOML`, `manifest()`, `start_test_server()`) and the TUI render test stayed in mod.rs.

**T04 — Final verification:** Belt-and-suspenders check confirmed all thresholds met, 286 tests green, clippy/doc/build clean. The 16 pre-existing collapsible-if clippy warnings were already resolved before this task.

## Verification

| Check | Result | Evidence |
|-------|--------|----------|
| `wc -l run/mod.rs` | 116 < 300 | ✓ PASS |
| `wc -l ssh/mod.rs` | 111 < 400 | ✓ PASS |
| `wc -l tests/mod.rs` | 88 < 500 | ✓ PASS |
| `cargo test --workspace` | 286 pass, 0 fail | ✓ PASS |
| `cargo doc --workspace --no-deps` | 0 warnings | ✓ PASS |
| `cargo clippy --workspace -- -D warnings` | exit 0 | ✓ PASS |
| `cargo build --workspace` | clean | ✓ PASS |

## Requirements Advanced

- R044 — All three target files decomposed under thresholds; 286 tests confirm zero regressions

## Requirements Validated

- R044 (Large file decomposition) — All size thresholds met (116/111/88 vs 300/400/500), all 286 tests pass, all public API signatures preserved, deny(missing_docs) compiles clean

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01: Removed `should_create_pr` re-export from mod.rs — only consumed within the run module, not externally
- T03: Moved `test_manifest_delivery_and_remote_exec` to ssh_dispatch.rs instead of keeping in mod.rs — groups SSH tests together for coherence

## Known Limitations

- none — pure refactoring slice with no behavior changes

## Follow-ups

- none

## Files Created/Modified

- `crates/smelt-cli/src/commands/run/mod.rs` — Public API surface (116 lines)
- `crates/smelt-cli/src/commands/run/phases.rs` — Container lifecycle phases
- `crates/smelt-cli/src/commands/run/dry_run.rs` — Dry-run mode + tests
- `crates/smelt-cli/src/commands/run/helpers.rs` — Gitignore/PR guards + tests
- `crates/smelt-cli/src/serve/ssh/mod.rs` — SshClient trait + types (111 lines)
- `crates/smelt-cli/src/serve/ssh/client.rs` — SubprocessSshClient implementation
- `crates/smelt-cli/src/serve/ssh/operations.rs` — SSH free functions
- `crates/smelt-cli/src/serve/ssh/mock.rs` — MockSshClient + all SSH tests
- `crates/smelt-cli/src/serve/tests/mod.rs` — Shared test helpers + TUI test (88 lines)
- `crates/smelt-cli/src/serve/tests/queue.rs` — Queue unit tests
- `crates/smelt-cli/src/serve/tests/dispatch.rs` — Dispatch + watcher tests
- `crates/smelt-cli/src/serve/tests/http.rs` — HTTP API tests
- `crates/smelt-cli/src/serve/tests/ssh_dispatch.rs` — SSH dispatch tests
- `crates/smelt-cli/src/serve/tests/config.rs` — Config validation tests

## Forward Intelligence

### What the next slice should know
- This is the final slice in M009. No downstream slices.

### What's fragile
- The `pub(crate) mod tests` shim in `ssh/mod.rs` exists solely for backward compatibility — if `dispatch.rs` or other consumers change their import path for `MockSshClient`, the shim can be removed.

### Authoritative diagnostics
- `cargo test --workspace` is the single source of truth for refactoring safety — 286 tests, 0 failures
- `wc -l` on the three mod.rs files confirms size thresholds

### What assumptions changed
- Collapsible-if clippy warnings (planned for T04) were already fixed before S03 — T04 became pure verification with no code changes
