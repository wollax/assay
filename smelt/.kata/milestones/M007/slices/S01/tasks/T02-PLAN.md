---
estimated_steps: 5
estimated_files: 3
---

# T02: Update elapsed callsites and verify all 19 serve tests pass

**Slice:** S01 — Serialize queue types + migrate Instant to SystemTime
**Milestone:** M007

## Description

Three files still reference `.elapsed()` on the old `Instant` fields and `Instant::now()` in the test: `http_api.rs`, `tui.rs`, and `tests.rs`. This task updates all three to use `elapsed_secs_since()` and `now_epoch()` from `types.rs`, then runs the full serve test suite to confirm zero regressions across all 19 tests.

## Steps

1. Open `crates/smelt-cli/src/serve/http_api.rs`. Add `use crate::serve::types::elapsed_secs_since;` to the imports. In `JobStateResponse::from()`:
   - Change `queued_age_secs: job.queued_at.elapsed().as_secs()` → `queued_age_secs: elapsed_secs_since(job.queued_at) as u64`
   - Change `elapsed_secs: job.started_at.map(|t| t.elapsed().as_secs_f64())` → `elapsed_secs: job.started_at.map(|t| elapsed_secs_since(t))`
   Remove any remaining `Instant`-related imports if present.

2. Open `crates/smelt-cli/src/serve/tui.rs`. Add `use crate::serve::types::elapsed_secs_since;` to the imports. In `render()`, change the elapsed computation inside the `.map(|j| ...)` closure:
   - Change `j.started_at.map(|t| format!("{}s", t.elapsed().as_secs()))` → `j.started_at.map(|t| format!("{}s", elapsed_secs_since(t) as u64))`

3. Open `crates/smelt-cli/src/serve/tests.rs`. Locate `test_tui_render_no_panic`. Inside the block that directly constructs a `QueuedJob`:
   - Remove the `use std::time::Instant;` import (either the block-level use or the file-level one if it was only used here)
   - Change `queued_at: Instant::now()` → `queued_at: crate::serve::types::now_epoch()`
   - Change `started_at: Some(Instant::now())` → `started_at: Some(crate::serve::types::now_epoch())`

4. Run `cargo check -p smelt-cli`. Must show zero errors and zero warnings about `Instant` imports.

5. Run `cargo test -p smelt-cli`. All 19 serve tests must pass. If any fail, inspect the error, apply the fix, and re-run.

## Must-Haves

- [ ] `http_api.rs` `queued_age_secs` computed via `elapsed_secs_since(job.queued_at) as u64`
- [ ] `http_api.rs` `elapsed_secs` computed via `job.started_at.map(|t| elapsed_secs_since(t))`
- [ ] `tui.rs` elapsed column computed via `elapsed_secs_since(t) as u64`
- [ ] `tests.rs` `test_tui_render_no_panic` constructs `QueuedJob` with `now_epoch()` values, not `Instant::now()`
- [ ] `cargo check -p smelt-cli` exits 0 with no warnings about `Instant`
- [ ] `cargo test -p smelt-cli` exits 0; all 19 serve tests pass; 0 failed

## Verification

- `cargo test -p smelt-cli 2>&1 | grep -E "^test result"` shows `test result: ok. N passed; 0 failed`
- `cargo check -p smelt-cli 2>&1 | grep -i "warning\|error"` shows nothing related to `Instant` or type mismatches
- `grep "Instant" crates/smelt-cli/src/serve/http_api.rs crates/smelt-cli/src/serve/tui.rs crates/smelt-cli/src/serve/tests.rs` returns empty (no stale `Instant` references)

## Observability Impact

- Signals added/changed: HTTP `GET /api/v1/jobs` and `GET /api/v1/jobs/:id` now return `queued_age_secs` and `elapsed_secs` derived from `u64` epoch arithmetic via `elapsed_secs_since()`; values are semantically identical to before but computed differently
- How a future agent inspects this: `curl http://localhost:<port>/api/v1/jobs` returns JSON with numeric `queued_age_secs` and `elapsed_secs` fields; TUI elapsed column shows `Ns` format
- Failure state exposed: if `elapsed_secs_since()` returns a negative value (impossible with `max(0.0)` guard), TUI would show `0s`; HTTP API would show `0.0` — both are safe defaults

## Inputs

- `crates/smelt-cli/src/serve/types.rs` — `now_epoch()` and `elapsed_secs_since()` exported (from T01)
- `crates/smelt-cli/src/serve/http_api.rs` — two `.elapsed()` callsites on `Instant` fields (pre-T02 state)
- `crates/smelt-cli/src/serve/tui.rs` — one `.elapsed()` callsite in render (pre-T02 state)
- `crates/smelt-cli/src/serve/tests.rs` — `test_tui_render_no_panic` constructs `QueuedJob` with `Instant::now()` (pre-T02 state)

## Expected Output

- `crates/smelt-cli/src/serve/http_api.rs` — uses `elapsed_secs_since()` for both timing fields; no `Instant` references
- `crates/smelt-cli/src/serve/tui.rs` — uses `elapsed_secs_since()` for elapsed column; no `Instant` references
- `crates/smelt-cli/src/serve/tests.rs` — `test_tui_render_no_panic` uses `now_epoch()` for `QueuedJob` construction
- `cargo test -p smelt-cli` exits 0 with all tests passing
