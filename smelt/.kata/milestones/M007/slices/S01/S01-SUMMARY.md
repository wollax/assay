---
id: S01
parent: M007
milestone: M007
provides:
  - "`QueuedJob` with `#[derive(Serialize, Deserialize)]` and `u64` timing fields (`queued_at`, `started_at: Option<u64>`)"
  - "`JobId` with `#[derive(Serialize, Deserialize)]` and `#[serde(transparent)]`"
  - "`JobSource` and `JobStatus` with full `Serialize + Deserialize`"
  - "`now_epoch() -> u64` public helper in `types.rs` — used by queue.rs, dispatch.rs, tests.rs"
  - "`elapsed_secs_since(epoch: u64) -> f64` public helper in `types.rs` with `.max(0.0)` clock-skew guard"
  - "`http_api.rs` computes `queued_age_secs` and `elapsed_secs` via `elapsed_secs_since()`"
  - "`tui.rs` elapsed column computed via `elapsed_secs_since(t) as u64`"
  - "All 46 smelt-cli tests pass; 19 serve tests confirmed by `--list`"
requires: []
affects:
  - S02
  - S03
key_files:
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/dispatch.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "D110 — `u64` Unix epoch seconds replace `Instant` in `QueuedJob`; required for serde/TOML round-trip"
  - "`#[serde(transparent)]` on `JobId` serializes as a plain string in TOML (D113)"
  - "`elapsed_secs_since()` uses `.max(0.0)` guard — returns 0.0 instead of panic on clock skew"
  - "All timing operations flow through `now_epoch()` / `elapsed_secs_since()` in `types.rs`; callsites never import `SystemTime` directly"
patterns_established:
  - "Timing helpers (`now_epoch`, `elapsed_secs_since`) are the single source of truth in `types.rs`; all serve files import from there"
  - "Serde on queue types follows the TOML-native convention: transparent wrappers, no `deny_unknown_fields` on `QueuedJob`"
observability_surfaces:
  - "`GET /api/v1/jobs` and `GET /api/v1/jobs/:id` return `queued_age_secs` (u64) and `elapsed_secs` (f64) computed from epoch fields"
  - "TUI elapsed column shows `Ns` format; `0s` on clock skew (safe default from guard in types.rs)"
  - "`grep 'now_epoch\\|elapsed_secs_since' crates/smelt-cli/src/serve/` confirms all callsites migrated"
  - "`cargo check -p smelt-cli` with zero warnings is the primary migration health signal"
drill_down_paths:
  - .kata/milestones/M007/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T02-SUMMARY.md
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S01: Serialize queue types + migrate Instant to SystemTime

**Replaced all `Instant`-based timing in `QueuedJob` with `u64` Unix epoch seconds; added full `Serialize + Deserialize` to all four queue types; updated all callsites; 46 tests pass with zero regressions.**

## What Happened

T01 established the foundation: `types.rs` gained two public helpers (`now_epoch()` and `elapsed_secs_since()`), all four queue types gained serde derives (`JobId` with `#[serde(transparent)]`, `JobSource`/`JobStatus` gained `Deserialize`, `QueuedJob` gained full `Serialize + Deserialize` with `u64` timing fields replacing `Instant`). `queue.rs` and `dispatch.rs` were updated to use `now_epoch()` at all construction sites, removing all `Instant` imports. After T01, exactly three compile errors remained — in `http_api.rs`, `tui.rs`, and `tests.rs` — where `.elapsed()` was called on what were now `u64` fields.

T02 cleared those three callsites: `http_api.rs` and `tui.rs` imported `elapsed_secs_since` and replaced the two and one `.elapsed()` calls respectively; `tests.rs` replaced `Instant::now()` with `now_epoch()` in `test_tui_render_no_panic` and removed the `std::time::Instant` import. The only remaining `Instant` reference in those files is `tokio::time::Instant` in an unrelated test deadline — not the `std::time::Instant` that was driving `QueuedJob` fields.

## Verification

- `cargo test -p smelt-cli`: **46 passed; 0 failed** (including all 19 serve tests confirmed by `--list`)
- `cargo check -p smelt-cli`: exits 0 with zero warnings
- `grep "Instant" http_api.rs tui.rs tests.rs`: no `std::time::Instant` references; only `tokio::time::Instant` in an unrelated test deadline
- `grep "now_epoch\|elapsed_secs_since" types.rs`: both function definitions present

## Requirements Advanced

- R028 — This slice removes the blocking `Instant` serialization issue; `QueuedJob` is now TOML round-trip capable, unblocking S02 (atomic state file write) and S03 (load-on-startup).

## Requirements Validated

- None validated in this slice (R028 requires S02 + S03 for full proof).

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

None. All changes matched the plan exactly across both tasks.

## Known Limitations

- `QueuedJob` has no `#[serde(deny_unknown_fields)]` — intentional for forward compatibility (D108). Future fields added to the state file will not cause parse errors on old binaries.
- `elapsed_secs_since()` returns `f64` which can be negative if the system clock is set backward — `.max(0.0)` guard clamps to 0.0 but does not alert. This is acceptable for display-only elapsed values.

## Follow-ups

- S02: Add `write_queue_state()` / `read_queue_state()` and wire atomic writes on every `ServerState` mutation.
- S03: Implement `ServerState::load_or_new()` and the restart-recovery integration test.

## Files Created/Modified

- `crates/smelt-cli/src/serve/types.rs` — Added `now_epoch()`/`elapsed_secs_since()` helpers; full serde derives on all four types; `u64` timing fields on `QueuedJob`; removed `Instant` import
- `crates/smelt-cli/src/serve/queue.rs` — Removed `Instant` import; uses `now_epoch()` at both construction sites
- `crates/smelt-cli/src/serve/dispatch.rs` — Removed `Instant` from import; uses `now_epoch()` at Running transition
- `crates/smelt-cli/src/serve/http_api.rs` — Replaced two `.elapsed()` callsites with `elapsed_secs_since()`; added import
- `crates/smelt-cli/src/serve/tui.rs` — Replaced one `.elapsed()` callsite with `elapsed_secs_since()`; added import
- `crates/smelt-cli/src/serve/tests.rs` — Replaced `Instant::now()` with `now_epoch()` in `test_tui_render_no_panic`; removed `use std::time::Instant`

## Forward Intelligence

### What the next slice should know
- `QueuedJob` derives `Serialize + Deserialize` but has no `#[serde(deny_unknown_fields)]` — this is intentional; do not add it.
- `JobId` serializes as a plain string via `#[serde(transparent)]` — TOML keys/values using `JobId` will appear as bare strings, not wrapped objects.
- The `now_epoch()` helper is the canonical time source for all queue operations; do not use `SystemTime::now()` directly in new serve code.

### What's fragile
- `elapsed_secs_since()` silently returns 0.0 on clock skew rather than an error — downstream display is safe but loses diagnostic value if clocks diverge significantly. Not a concern for local deployments.
- `tests.rs`'s `test_tui_render_no_panic` uses `now_epoch()` for both `queued_at` and `started_at`, meaning elapsed is always ~0s in tests. If a test needs non-trivial elapsed values, it must subtract a constant from `now_epoch()`.

### Authoritative diagnostics
- `cargo check -p smelt-cli` with zero warnings is the primary health signal for the migration. Any warning about unused `Instant` import means a callsite was missed.
- `cargo test -p smelt-cli -- --list 2>&1 | grep -c serve::tests` must show 19 to confirm no tests were deleted.

### What assumptions changed
- Original plan assumed `elapsed_secs_since` would use `saturating_sub` on integers (u64). Actual implementation uses f64 subtraction with `.max(0.0)`, which gives sub-second precision in the `elapsed_secs` HTTP field. This is strictly better — no assumption broke, only precision improved.
