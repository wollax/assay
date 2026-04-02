---
id: S02
parent: M010
milestone: M010
provides:
  - warn_teardown() async helper replacing 6 duplicated silent teardown blocks in phases.rs
  - .context() error chain preservation on all 5 monitor.write()/set_phase() calls
  - eprintln! warnings on teardown failures (provider.teardown, monitor.set_phase, monitor.cleanup)
  - build_common_ssh_args() private helper consolidating SSH/SCP flag-building logic
  - build_ssh_args()/build_scp_args() reduced to single-line delegations
requires:
  - slice: none
    provides: independent slice — no upstream dependencies
affects:
  - S03
key_files:
  - crates/smelt-cli/src/commands/run/phases.rs
  - crates/smelt-cli/src/serve/ssh/client.rs
key_decisions:
  - "warn_teardown prints 'Container removed.' only on teardown success — previously printed unconditionally after silent discard"
  - "Kept let _ = on outcome match block (GatesFailed/Complete/Failed/Timeout/Cancelled) — best-effort phase transitions, not teardown"
  - "build_common_ssh_args is private — only the two public wrappers are API surface"
patterns_established:
  - "warn_teardown(monitor, provider, container) pattern for early-return teardown in phases.rs"
  - "build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args) pattern for SSH/SCP arg building"
observability_surfaces:
  - "eprintln!(Warning: ...) on teardown failures — previously silent let _ = discards"
  - ".context() on monitor.write()/set_phase() preserves SmeltError chain through anyhow"
drill_down_paths:
  - .kata/milestones/M010/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M010/slices/S02/tasks/T02-SUMMARY.md
duration: 18min
verification_result: passed
completed_at: 2026-03-24T12:30:00Z
---

# S02: Teardown error handling + SSH DRY cleanup

**Teardown failures now produce visible stderr warnings instead of silent `let _ =` discards; error chains preserved via `.context()`; SSH arg builders consolidated into a single shared helper**

## What Happened

Two independent code quality fixes from the PR #33 review backlog:

**T01 — Teardown error handling:** Extracted a `warn_teardown()` async helper in `phases.rs` that consolidates the three teardown steps (set TearingDown phase → provider.teardown → monitor.cleanup) with `eprintln!` warnings on each failure. Replaced all 6 duplicated early-return teardown blocks with calls to this helper. Replaced 5 `anyhow!("{e}")` calls on `monitor.write()` and `monitor.set_phase()` with `.context()`, preserving the original `SmeltError` chain instead of stringifying it. The final "always runs" teardown block was already properly logging warnings and was left unchanged.

**T02 — SSH DRY cleanup:** Extracted `build_common_ssh_args()` private helper in `client.rs` parameterized by port flag (`-p` vs `-P`) and tool name (`"SSH"` vs `"SCP"`). Both `build_ssh_args()` and `build_scp_args()` are now single-line delegations. ~40 fewer lines of duplicated flag-building code.

## Verification

| Check | Status | Evidence |
| --- | --- | --- |
| `cargo test --workspace` | ✓ PASS | 155 tests + 3 doc-tests, 0 failures |
| `cargo clippy --workspace` | ✓ PASS | Clean |
| `cargo doc --workspace --no-deps` | ✓ PASS | Zero warnings |
| `rg 'anyhow!.*\{e\}' phases.rs` | ✓ PASS | Zero hits |
| `rg 'let _ = provider\.teardown' phases.rs` | ✓ PASS | Zero hits |
| `build_ssh_args` body ≤5 lines | ✓ PASS | 1 line delegation |
| `build_scp_args` body ≤5 lines | ✓ PASS | 1 line delegation |
| SSH arg tests unchanged | ✓ PASS | 4/4 tests pass without modification |

## Requirements Advanced

- R052 — All 6 silent `let _ =` teardown discards replaced with `warn_teardown()` helper; all 5 `anyhow!("{e}")` replaced with `.context()`
- R053 — `build_ssh_args`/`build_scp_args` now delegate to `build_common_ssh_args()` — zero duplicated flag logic

## Requirements Validated

- R052 — Teardown errors now produce visible `eprintln!` warnings; error chains preserved via `.context()`; verified by code inspection + full test suite passing
- R053 — SSH arg builders share common helper; verified by 4 existing mock tests passing unchanged + body size ≤1 line each

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- `warn_teardown()` uses `eprintln!` directly rather than `tracing::warn!` — consistent with existing teardown warning pattern in the final cleanup block, but a future tracing migration could unify these
- `build_common_ssh_args()` doc links use backtick-only references (not `[`link`]`) to avoid cargo doc warnings when linking to private items (consistent with D070)

## Follow-ups

- none

## Files Created/Modified

- `crates/smelt-cli/src/commands/run/phases.rs` — Added `warn_teardown()` helper, replaced 6 teardown blocks and 5 lossy error conversions
- `crates/smelt-cli/src/serve/ssh/client.rs` — Extracted `build_common_ssh_args()`, rewrote `build_ssh_args`/`build_scp_args` as delegations

## Forward Intelligence

### What the next slice should know
- phases.rs and client.rs are clean — S03 only needs to focus on documentation (examples/server.toml and README.md)
- All code quality items from the PR review backlog are resolved

### What's fragile
- Nothing — pure refactoring with full test coverage

### Authoritative diagnostics
- `cargo test --workspace` is the single gate — all 155+ tests must pass
- `rg 'let _ =' phases.rs` should show only the outcome match block (GatesFailed/Complete/Failed/Timeout/Cancelled), not teardown paths

### What assumptions changed
- No assumptions changed — scope matched plan exactly
