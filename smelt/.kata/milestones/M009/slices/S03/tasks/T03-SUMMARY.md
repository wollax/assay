---
id: T03
parent: S03
milestone: M009
provides:
  - serve/tests/ directory module replacing monolithic 1370-line tests.rs
  - tests/mod.rs (88 lines) — shared helpers (VALID_MANIFEST_TOML, manifest(), start_test_server()) + TUI test + mod declarations
  - tests/queue.rs — 4 queue unit tests
  - tests/dispatch.rs — 2 dispatch + 2 watcher tests
  - tests/http.rs — 7 HTTP API tests
  - tests/ssh_dispatch.rs — 6 SSH dispatch/round-robin/failover tests
  - tests/config.rs — 9 config validation tests
key_files:
  - crates/smelt-cli/src/serve/tests/mod.rs
  - crates/smelt-cli/src/serve/tests/queue.rs
  - crates/smelt-cli/src/serve/tests/dispatch.rs
  - crates/smelt-cli/src/serve/tests/http.rs
  - crates/smelt-cli/src/serve/tests/ssh_dispatch.rs
  - crates/smelt-cli/src/serve/tests/config.rs
key_decisions:
  - "D131: test_manifest_delivery_and_remote_exec moved to ssh_dispatch.rs rather than staying in mod.rs — groups all SSH-related tests together for feature coherence"
patterns_established:
  - "Same flat-to-directory module conversion pattern from T01/T02: move to mod.rs, extract child modules, shared helpers stay in mod.rs"
observability_surfaces:
  - none — pure refactoring
duration: 10min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T03: Decomposed serve tests.rs (1370L) into 6-file directory module with mod.rs at 90 lines

**Converted `serve/tests.rs` (1370 lines, 31 tests) into a `serve/tests/` directory module with `mod.rs` at 90 lines — well under the 500-line threshold.**

## What Happened

Followed the same flat-to-directory module conversion pattern established in T01 and T02. Moved `tests.rs` to `tests/mod.rs`, then extracted tests by feature area into 5 child modules: `queue.rs` (4 tests), `dispatch.rs` (4 tests — dispatch loop + cancellation + 2 watcher tests), `http.rs` (7 tests), `ssh_dispatch.rs` (6 tests — round-robin, failover, requeue, state persistence, manifest delivery, state sync), and `config.rs` (9 tests). Shared helpers (`VALID_MANIFEST_TOML`, `manifest()`, `start_test_server()`) and the TUI render test stayed in `mod.rs`. All child modules use `use super::*` or `use super::VALID_MANIFEST_TOML` to access shared items.

## Verification

- `cargo build --workspace` — clean, no warnings
- `cargo test -p smelt-cli -- serve::tests` — 29 passed, 2 ignored (gated SSH tests), 0 failures
- `cargo test --workspace` — 286 passing, 0 failures
- `cargo doc --workspace --no-deps` — 0 warnings
- `wc -l crates/smelt-cli/src/serve/tests/mod.rs` — 90 lines (< 500 threshold)

### Slice-level verification (all pass — this is the final task):
- `cargo test --workspace` — all lines show 0 failures, 286 passing ✓
- `cargo doc --workspace --no-deps` — 0 warnings ✓
- `run/mod.rs` — 116 lines (< 300) ✓
- `ssh/mod.rs` — 111 lines (< 400) ✓
- `tests/mod.rs` — 90 lines (< 500) ✓

## Diagnostics

None — pure refactoring. Verify with `cargo test`, `cargo build`, `wc -l`.

## Deviations

Moved `test_manifest_delivery_and_remote_exec` from mod.rs to ssh_dispatch.rs for feature coherence (plan had it in mod.rs). This groups all SSH-related tests in one file.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/tests/mod.rs` — shared helpers + TUI test + mod declarations (90 lines)
- `crates/smelt-cli/src/serve/tests/queue.rs` — 4 queue unit tests
- `crates/smelt-cli/src/serve/tests/dispatch.rs` — 2 dispatch + 2 watcher tests
- `crates/smelt-cli/src/serve/tests/http.rs` — 7 HTTP API tests
- `crates/smelt-cli/src/serve/tests/ssh_dispatch.rs` — 6 SSH dispatch tests
- `crates/smelt-cli/src/serve/tests/config.rs` — 9 config validation tests
- `crates/smelt-cli/src/serve/tests.rs` — deleted (replaced by directory module)
