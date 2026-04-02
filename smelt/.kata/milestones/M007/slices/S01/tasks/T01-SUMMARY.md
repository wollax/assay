---
id: T01
parent: S01
milestone: M007
provides:
  - "`QueuedJob` with `#[derive(Serialize, Deserialize)]` and `u64` timing fields (`queued_at`, `started_at`)"
  - "`JobId` with `#[derive(Serialize, Deserialize)]` and `#[serde(transparent)]`"
  - "`JobSource` and `JobStatus` with `Deserialize` added to existing derive lists"
  - "`now_epoch() -> u64` public helper in `types.rs`"
  - "`elapsed_secs_since(epoch: u64) -> f64` public helper in `types.rs` with clock-skew guard"
  - "`queue.rs` uses `now_epoch()` at both enqueue and try_dispatch sites; no `Instant` import"
  - "`dispatch.rs` uses `now_epoch()` at the Running transition; `Instant` removed from import"
key_files:
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/dispatch.rs
key_decisions:
  - "u64 Unix epoch seconds (not SystemTime directly) for QueuedJob timing fields — required for serde compatibility (D110)"
  - "`#[serde(transparent)]` on JobId serializes as plain string, required for TOML round-trip compatibility"
  - "`elapsed_secs_since` uses `.max(0.0)` guard — returns 0.0 instead of negative on clock skew"
patterns_established:
  - "All timing operations go through `now_epoch()` / `elapsed_secs_since()` in `types.rs` — callsites never import SystemTime directly"
observability_surfaces:
  - "`elapsed_secs_since(epoch: u64) -> f64` is the runtime primitive for all display-facing elapsed computations in T02"
  - "`grep 'now_epoch\\|elapsed_secs_since' crates/smelt-cli/src/serve/` confirms all callsites are migrated"
duration: 5min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Migrate `QueuedJob` fields and add serde derives to all queue types

**Replaced `Instant`-based timing in `QueuedJob` with `u64` Unix epoch seconds; added full `Serialize + Deserialize` to all four queue types; added `now_epoch()` and `elapsed_secs_since()` helpers.**

## What Happened

Updated `types.rs` to import `SystemTime` and `UNIX_EPOCH` (removing `Instant`), added two public timing helpers (`now_epoch()` and `elapsed_secs_since()`), added `#[derive(Serialize, Deserialize)]` with `#[serde(transparent)]` to `JobId`, added `Deserialize` to `JobSource` and `JobStatus`, and changed `QueuedJob`'s timing fields from `Instant`/`Option<Instant>` to `u64`/`Option<u64>` with full serde derives.

Updated `queue.rs` to import `now_epoch` from `crate::serve::types` and use it at both construction sites (`enqueue()` and `try_dispatch()`), removing the `Instant` import.

Updated `dispatch.rs` to remove `Instant` from its time import and use `crate::serve::types::now_epoch()` at the one remaining `started_at` assignment in `run_job_task()`.

## Verification

- `cargo check -p smelt-cli 2>&1 | grep "^error" | grep -v "http_api\|tui\|tests"` → no output (zero errors outside expected callsites)
- Errors present only in `http_api.rs:53`, `http_api.rs:54`, `tui.rs:74` — all calling `.elapsed()` on the now-`u64` fields (expected; resolved in T02)
- `grep "Instant" types.rs queue.rs dispatch.rs` → no output (all `Instant` references removed)
- `grep "now_epoch\|elapsed_secs_since" types.rs` → both function definitions present

## Diagnostics

- `grep "now_epoch\|elapsed_secs_since" crates/smelt-cli/src/serve/` confirms all callsites migrated
- Compiler errors in `http_api.rs` and `tui.rs` are the desired failure state — they identify the remaining T02 callsites

## Deviations

None.

## Known Issues

`http_api.rs` and `tui.rs` have 3 compile errors calling `.elapsed()` on `u64` fields — this is the expected state after T01 and is resolved in T02.

## Files Created/Modified

- `crates/smelt-cli/src/serve/types.rs` — Added `now_epoch()`/`elapsed_secs_since()` helpers; full serde derives on all four types; `u64` timing fields on `QueuedJob`
- `crates/smelt-cli/src/serve/queue.rs` — Removed `Instant` import; uses `now_epoch()` at both construction sites
- `crates/smelt-cli/src/serve/dispatch.rs` — Removed `Instant` from import; uses `crate::serve::types::now_epoch()` at Running transition
