---
estimated_steps: 5
estimated_files: 3
---

# T01: Migrate `QueuedJob` fields and add serde derives to all queue types

**Slice:** S01 — Serialize queue types + migrate Instant to SystemTime
**Milestone:** M007

## Description

Replace `Instant`-based timing fields in `QueuedJob` with `u64` Unix epoch seconds, add `Serialize + Deserialize` to all four queue types (`JobId`, `JobSource`, `JobStatus`, `QueuedJob`), and add the `now_epoch()` and `elapsed_secs_since()` helper functions. Update the two construction sites in `queue.rs` and the one in `dispatch.rs`. This task establishes the new type contract that `http_api.rs`, `tui.rs`, and `tests.rs` will consume in T02.

## Steps

1. Open `crates/smelt-cli/src/serve/types.rs`. Add `use std::time::{SystemTime, UNIX_EPOCH};` alongside the existing `use std::time::Instant;` (then remove `Instant` from the import after the struct changes). Add `use serde::Deserialize;` alongside `use serde::Serialize;`.

2. Add two public helper functions after the `use` block:
   ```rust
   /// Returns the current time as seconds since the Unix epoch.
   /// `unwrap_or_default()` handles the impossible pre-1970 case.
   pub fn now_epoch() -> u64 {
       SystemTime::now()
           .duration_since(UNIX_EPOCH)
           .unwrap_or_default()
           .as_secs()
   }

   /// Returns elapsed seconds since a stored Unix epoch value as `f64`.
   /// Uses `as_secs_f64()` to preserve sub-second precision from `SystemTime`.
   /// Returns 0.0 if the stored epoch is in the future (clock skew guard).
   pub fn elapsed_secs_since(epoch: u64) -> f64 {
       let now = SystemTime::now()
           .duration_since(UNIX_EPOCH)
           .unwrap_or_default()
           .as_secs_f64();
       (now - epoch as f64).max(0.0)
   }
   ```

3. Update `JobId`: add `#[derive(Serialize, Deserialize)]` and `#[serde(transparent)]` to make it serialize as a plain string.

4. Add `Deserialize` to the derive lists of `JobSource` and `JobStatus` (they already have `Serialize` and `#[serde(rename_all = "snake_case")]`).

5. Update `QueuedJob`: add `#[derive(Serialize, Deserialize)]`; change `queued_at: Instant` → `queued_at: u64`; change `started_at: Option<Instant>` → `started_at: Option<u64>`. Remove `use std::time::Instant;` from the file (only `SystemTime` and `UNIX_EPOCH` remain needed).

6. Open `crates/smelt-cli/src/serve/queue.rs`. Remove `use std::time::Instant;`. Add `use crate::serve::types::now_epoch;`. In `enqueue()`, change `queued_at: Instant::now()` → `queued_at: now_epoch()`. In `try_dispatch()`, change `started_at = Some(Instant::now())` → `started_at = Some(now_epoch())`.

7. Open `crates/smelt-cli/src/serve/dispatch.rs`. Change `use std::time::{Duration, Instant};` → `use std::time::Duration;`. In `run_job_task()`, change `job.started_at = Some(Instant::now())` → `job.started_at = Some(crate::serve::types::now_epoch())`.

8. Run `cargo check -p smelt-cli`. Expect errors only in `http_api.rs`, `tui.rs`, and `tests.rs` — not in the three files just edited.

## Must-Haves

- [ ] `QueuedJob` has `#[derive(Serialize, Deserialize)]`
- [ ] `queued_at` field is `u64`, `started_at` field is `Option<u64>`
- [ ] `JobId` has `#[derive(Serialize, Deserialize)]` with `#[serde(transparent)]`
- [ ] `JobSource` and `JobStatus` both have `Deserialize` in their derive list
- [ ] `now_epoch() -> u64` is a `pub` function in `types.rs`
- [ ] `elapsed_secs_since(epoch: u64) -> f64` is a `pub` function in `types.rs`
- [ ] `queue.rs` uses `now_epoch()` at both construction sites; no `Instant` import
- [ ] `dispatch.rs` uses `now_epoch()` at the one construction site; `Instant` removed from import
- [ ] `cargo check -p smelt-cli` shows errors only in `http_api.rs`, `tui.rs`, `tests.rs` — not in the three edited files

## Verification

- `cargo check -p smelt-cli 2>&1 | grep "^error" | grep -v "http_api\|tui\|tests"` returns no lines (zero errors outside the not-yet-updated callsites)
- `grep "Instant" crates/smelt-cli/src/serve/types.rs crates/smelt-cli/src/serve/queue.rs crates/smelt-cli/src/serve/dispatch.rs` returns empty (no `Instant` references remain in these three files)
- `grep "now_epoch\|elapsed_secs_since" crates/smelt-cli/src/serve/types.rs` shows both function definitions

## Observability Impact

- Signals added/changed: `now_epoch()` and `elapsed_secs_since()` are the new runtime timing primitives; downstream callsites in T02 use them to compute all display-facing elapsed values
- How a future agent inspects this: `grep "now_epoch\|elapsed_secs_since" crates/smelt-cli/src/serve/` to confirm all callsites are migrated
- Failure state exposed: compiler errors in `http_api.rs`/`tui.rs`/`tests.rs` are the expected (desired) failure state after this task; they are resolved in T02

## Inputs

- `crates/smelt-cli/src/serve/types.rs` — defines `JobId`, `JobSource`, `JobStatus`, `QueuedJob` with `Instant` fields (current state before this task)
- `crates/smelt-cli/src/serve/queue.rs` — two `Instant::now()` construction sites
- `crates/smelt-cli/src/serve/dispatch.rs` — one `Instant::now()` construction site
- Research: D110 mandates `u64` epoch seconds; `elapsed_secs_since` must use `saturating`/`max(0.0)` for clock-skew safety; `#[serde(transparent)]` on `JobId` required for TOML round-trip compatibility (from S01-RESEARCH.md)

## Expected Output

- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob` with `u64` timing fields and full serde derives; `now_epoch()` and `elapsed_secs_since()` exported
- `crates/smelt-cli/src/serve/queue.rs` — `now_epoch()` used at both enqueue and dispatch sites; no `Instant` import
- `crates/smelt-cli/src/serve/dispatch.rs` — `now_epoch()` used at the Running transition; no `Instant` import
