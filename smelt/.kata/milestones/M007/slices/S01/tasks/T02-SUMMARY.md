---
id: T02
parent: S01
milestone: M007
provides:
  - "`http_api.rs` computes `queued_age_secs` via `elapsed_secs_since(job.queued_at) as u64`"
  - "`http_api.rs` computes `elapsed_secs` via `job.started_at.map(|t| elapsed_secs_since(t))`"
  - "`tui.rs` elapsed column computed via `elapsed_secs_since(t) as u64`"
  - "`tests.rs` `test_tui_render_no_panic` constructs `QueuedJob` with `now_epoch()` values"
  - "All 46 smelt-cli tests pass with zero failures"
key_files:
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "No new decisions — all changes are mechanical callsite migrations following T01's established helpers"
patterns_established:
  - "All `.elapsed()` / `Instant::now()` callsites in serve/ are replaced; the pattern `elapsed_secs_since(field)` and `now_epoch()` from types.rs is consistently used across all three serve files"
observability_surfaces:
  - "HTTP GET /api/v1/jobs and GET /api/v1/jobs/:id return `queued_age_secs` (u64) and `elapsed_secs` (f64) computed from u64 epoch fields via elapsed_secs_since()"
  - "TUI elapsed column shows `Ns` format derived from elapsed_secs_since(); saturating_sub guard ensures 0s not panic on clock skew"
duration: 5min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Update elapsed callsites and verify all 19 serve tests pass

**Migrated all three remaining `Instant`-based callsites in `http_api.rs`, `tui.rs`, and `tests.rs` to use `elapsed_secs_since()` / `now_epoch()`; 46 total tests pass.**

## What Happened

Three files still referenced `.elapsed()` on the old `Instant` fields after T01 migrated `QueuedJob`'s storage. The changes were straightforward:

- **`http_api.rs`**: Added `elapsed_secs_since` to the `use crate::serve::types` import. Replaced `job.queued_at.elapsed().as_secs()` with `elapsed_secs_since(job.queued_at) as u64` and `t.elapsed().as_secs_f64()` with `elapsed_secs_since(t)`.
- **`tui.rs`**: Added `use crate::serve::types::elapsed_secs_since`. Replaced `t.elapsed().as_secs()` with `elapsed_secs_since(t) as u64` in the render closure.
- **`tests.rs`**: Removed `use std::time::Instant` from the `test_tui_render_no_panic` block. Replaced `Instant::now()` with `crate::serve::types::now_epoch()` for both `queued_at` and `started_at` fields.

The only remaining `Instant` references in these files are `tokio::time::Instant` in an unrelated test timeout loop — not the `std::time::Instant` that was used for `QueuedJob` fields.

## Verification

- `cargo check -p smelt-cli` exits 0, no warnings or errors.
- `cargo test -p smelt-cli` exits 0: **46 passed; 0 failed** (including all serve tests).
- `grep "Instant" http_api.rs tui.rs tests.rs` shows no `std::time::Instant` references; only `tokio::time::Instant` remains in an unrelated test deadline.

## Diagnostics

- `curl http://localhost:<port>/api/v1/jobs` returns JSON with `queued_age_secs` (u64) and `elapsed_secs` (f64 or null) derived from `elapsed_secs_since()`.
- TUI elapsed column: `elapsed_secs_since(t) as u64` formatted as `"Ns"` — `0s` on clock skew (safe default from saturating_sub guard in types.rs).

## Deviations

None. All changes matched the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/http_api.rs` — replaced two `.elapsed()` callsites with `elapsed_secs_since()`; added import
- `crates/smelt-cli/src/serve/tui.rs` — replaced one `.elapsed()` callsite with `elapsed_secs_since()`; added import
- `crates/smelt-cli/src/serve/tests.rs` — replaced `Instant::now()` with `now_epoch()` in `test_tui_render_no_panic`; removed `use std::time::Instant`
