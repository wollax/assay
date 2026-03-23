# S01: Serialize queue types + migrate Instant to SystemTime

**Goal:** Replace `Instant`-based timing fields in `QueuedJob` with `u64` Unix epoch seconds; add `Serialize + Deserialize` to all queue types; update all callsites; keep all 19 existing serve tests green.
**Demo:** `cargo test -p smelt-cli` passes all 19 serve tests; `QueuedJob` has `#[derive(Serialize, Deserialize)]`; HTTP `elapsed_secs` and TUI elapsed column compute correct values from `u64` epoch fields.

## Must-Haves

- `QueuedJob` has `#[derive(Serialize, Deserialize)]` with `queued_at: u64` and `started_at: Option<u64>`
- `JobId` has `#[derive(Serialize, Deserialize)]` with `#[serde(transparent)]`
- `JobSource` and `JobStatus` have `Deserialize` added (already had `Serialize`)
- `now_epoch() -> u64` helper in `types.rs` (used by queue.rs, dispatch.rs, tests.rs)
- `elapsed_secs_since(epoch: u64) -> f64` helper in `types.rs` (used by http_api.rs, tui.rs)
- `http_api.rs` `queued_age_secs` and `elapsed_secs` computed via `elapsed_secs_since()`
- `tui.rs` elapsed column computed via `elapsed_secs_since()`
- All 19 existing `serve::tests` pass — zero regressions
- `test_tui_render_no_panic` updated to construct `QueuedJob` with `u64` epoch fields

## Proof Level

- This slice proves: contract
- Real runtime required: no (all verification via `cargo test -p smelt-cli`)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli 2>&1 | tail -5` — all tests pass, zero failures
- `cargo check -p smelt-cli` — no warnings related to unused `Instant` imports
- Confirm `QueuedJob` compile-time: `cargo test -p smelt-cli -- --list 2>&1 | grep -c serve::tests` shows 19 tests

## Observability / Diagnostics

- Runtime signals: `elapsed_secs_since()` uses `saturating_sub` — clock-skew results in `0.0` instead of panic; no structured log change
- Inspection surfaces: HTTP `GET /api/v1/jobs/:id` response shows `queued_age_secs` and `elapsed_secs` derived from new `u64` epoch fields
- Failure visibility: compiler errors are the primary failure signal; `cargo check` shows exact file/line if a callsite is missed
- Redaction constraints: none (timing fields are not sensitive)

## Integration Closure

- Upstream surfaces consumed: none (first slice)
- New wiring introduced in this slice: `now_epoch()` and `elapsed_secs_since()` in `types.rs`; used by `queue.rs`, `dispatch.rs`, `http_api.rs`, `tui.rs`, `tests.rs`
- What remains before the milestone is truly usable end-to-end: S02 (atomic state file write on every transition), S03 (load-on-startup + restart-recovery integration test)

## Tasks

- [x] **T01: Migrate `QueuedJob` fields and add serde derives to all queue types** `est:45m`
  - Why: `Instant` is not serializable; `u64` epoch seconds is the TOML-native time representation (D110). This task establishes the new types and helpers that all other files depend on.
  - Files: `crates/smelt-cli/src/serve/types.rs`, `crates/smelt-cli/src/serve/queue.rs`, `crates/smelt-cli/src/serve/dispatch.rs`
  - Do:
    1. In `types.rs`: add `use std::time::{SystemTime, UNIX_EPOCH};`. Add `pub fn now_epoch() -> u64` that returns `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()`. Add `pub fn elapsed_secs_since(epoch: u64) -> f64` that returns `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() - epoch as f64` (this gives sub-second precision from `SystemTime`; the result is f64, negatives are impossible in practice but the consumer can clamp if desired).
    2. In `types.rs`: add `use serde::Deserialize;` alongside the existing `use serde::Serialize;`. Add `#[derive(Serialize, Deserialize)]` + `#[serde(transparent)]` to `JobId`. Add `Deserialize` to `JobSource` and `JobStatus` derives. Add `#[derive(Serialize, Deserialize)]` to `QueuedJob`. Change `queued_at: Instant` → `queued_at: u64` and `started_at: Option<Instant>` → `started_at: Option<u64>`. Remove `use std::time::Instant;`.
    3. In `queue.rs`: replace `use std::time::Instant;` with `use crate::serve::types::now_epoch;`. In `enqueue()`, change `queued_at: Instant::now()` → `queued_at: now_epoch()`. In `try_dispatch()`, change `started_at = Some(Instant::now())` → `started_at = Some(now_epoch())`.
    4. In `dispatch.rs`: change `use std::time::{Duration, Instant};` → `use std::time::Duration;`. In `run_job_task()`, change `job.started_at = Some(Instant::now())` → `job.started_at = Some(crate::serve::types::now_epoch())`.
    5. Run `cargo check -p smelt-cli` — expect errors only in `http_api.rs`, `tui.rs`, and `tests.rs` (callsites not yet updated). Verify the three edited files are clean.
  - Verify: `cargo check -p smelt-cli 2>&1 | grep "^error" | grep -v "http_api\|tui\|tests"` returns empty (no errors outside the not-yet-updated callsites)
  - Done when: `types.rs`, `queue.rs`, and `dispatch.rs` compile without error; `http_api.rs`/`tui.rs`/`tests.rs` errors are exclusively about changed field types.

- [x] **T02: Update callsites in `http_api.rs`, `tui.rs`, `tests.rs` and verify all 19 tests pass** `est:30m`
  - Why: Three files still call `.elapsed()` on `Instant` fields; they must be updated to use `elapsed_secs_since()`. The test that directly constructs `QueuedJob` with `Instant::now()` must be updated to use `now_epoch()`.
  - Files: `crates/smelt-cli/src/serve/http_api.rs`, `crates/smelt-cli/src/serve/tui.rs`, `crates/smelt-cli/src/serve/tests.rs`
  - Do:
    1. In `http_api.rs`: add `use crate::serve::types::elapsed_secs_since;` (alongside existing imports). In `JobStateResponse::from()`, change `queued_age_secs: job.queued_at.elapsed().as_secs()` → `queued_age_secs: elapsed_secs_since(job.queued_at) as u64`. Change `elapsed_secs: job.started_at.map(|t| t.elapsed().as_secs_f64())` → `elapsed_secs: job.started_at.map(|t| elapsed_secs_since(t))`. Remove any `Instant`-related imports if present.
    2. In `tui.rs`: add `use crate::serve::types::elapsed_secs_since;`. In `render()`, change `j.started_at.map(|t| format!("{}s", t.elapsed().as_secs()))` → `j.started_at.map(|t| format!("{}s", elapsed_secs_since(t) as u64))`.
    3. In `tests.rs`: In `test_tui_render_no_panic`, change `use std::time::Instant;` (if present, otherwise remove the `Instant::now()` references). Change `queued_at: Instant::now()` → `queued_at: crate::serve::types::now_epoch()`. Change `started_at: Some(Instant::now())` → `started_at: Some(crate::serve::types::now_epoch())`.
    4. Run `cargo test -p smelt-cli 2>&1 | tail -20`. All 19 serve tests must pass.
    5. Run `cargo check -p smelt-cli`. Must show zero warnings about unused `Instant` imports.
  - Verify: `cargo test -p smelt-cli 2>&1 | grep -E "^test result"` shows `0 failed`; `cargo check -p smelt-cli` exits 0
  - Done when: `cargo test -p smelt-cli` exits 0 with all 19 serve tests passing; no compiler warnings about removed `Instant` imports.

## Files Likely Touched

- `crates/smelt-cli/src/serve/types.rs`
- `crates/smelt-cli/src/serve/queue.rs`
- `crates/smelt-cli/src/serve/dispatch.rs`
- `crates/smelt-cli/src/serve/http_api.rs`
- `crates/smelt-cli/src/serve/tui.rs`
- `crates/smelt-cli/src/serve/tests.rs`
