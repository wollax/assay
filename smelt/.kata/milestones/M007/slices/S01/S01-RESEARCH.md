# S01: Serialize queue types + migrate Instant to SystemTime — Research

**Date:** 2026-03-23

## Summary

S01 is a pure type-migration and derive-annotation slice with no new logic and no new dependencies. All five files that need changing are small and well-understood: `types.rs` (the type definitions), `queue.rs` (construction sites), `dispatch.rs` (one `Instant::now()` site), `http_api.rs` (two `.elapsed()` callsites), and `tui.rs` (one `.elapsed()` callsite). The `serde` crate is already a workspace dep in smelt-cli; all other types (`JobSource`, `JobStatus`) already have `#[derive(Serialize)]` — they just need `Deserialize` added. The `toml` crate is already a regular dep (promoted in D062).

The only non-trivial design decision is how to compute "elapsed" after the type change. `Instant::now().duration_since(t)` becomes `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - stored_epoch_secs`. The subtraction can underflow if the clock is set backwards; this should be handled with a saturating subtraction or checked subtraction returning 0 on underflow.

There are **19 serve tests** currently all green (16 inline in `serve::tests`, plus 3 watcher/integration) that serve as the regression guard. One test (`test_tui_render_no_panic`) directly constructs a `QueuedJob` with `Instant::now()` fields — this test must be updated to use `u64` epoch values. No test constructs `QueuedJob` with specific timing assertions that would break on `u64` epoch arithmetic.

## Recommendation

Use `u64` Unix epoch seconds (not `SystemTime` directly) for `queued_at` and `started_at`. Store the epoch value via a helper `now_epoch() -> u64` that calls `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()`. This is the most compact, TOML-friendly representation (no custom serializer needed — `u64` serializes natively), and it matches D110.

Add a `elapsed_secs_since(epoch: u64) -> f64` free function (or method on `QueuedJob`) that computes `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().saturating_sub(epoch) as f64` — used by both `http_api.rs` and `tui.rs`.

**Do NOT** use `SystemTime` as the field type — it has no stable serde impl without a custom serializer. `u64` epoch seconds is the correct choice.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| `SystemTime` serde serialization | Use `u64` epoch seconds instead | `SystemTime` has no built-in serde impl; `u64` serializes natively to TOML integer; no crate needed |
| Atomic file write pattern | `smelt-core/src/monitor.rs` `JobMonitor` write pattern | Precedent already in codebase (D100): write to `.tmp`, `fs::rename`. S01 doesn't write state files but S02 will follow this exact pattern |
| TOML derive | `toml` crate already in `smelt-cli` deps | Already a regular dep (D062); `#[derive(Serialize, Deserialize)]` just works |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/types.rs` — Defines `JobId`, `JobSource`, `JobStatus`, `QueuedJob`. `JobSource` and `JobStatus` already have `#[derive(Serialize)]` with `#[serde(rename_all = "snake_case")]`. `JobId` has no serde derives at all (wraps `String`). `QueuedJob` has no `#[derive]` at all. All four need `Deserialize` added. `queued_at: Instant` and `started_at: Option<Instant>` become `queued_at: u64` and `started_at: Option<u64>`.
- `crates/smelt-cli/src/serve/queue.rs` — Two construction sites for `Instant::now()`: `enqueue()` sets `queued_at: Instant::now()` (line 39), `try_dispatch()` sets `started_at = Some(Instant::now())` (line 56). Both become `now_epoch()` calls.
- `crates/smelt-cli/src/serve/dispatch.rs` — One `Instant::now()` in `run_job_task()` sets `job.started_at = Some(Instant::now())` (line 42). Becomes `now_epoch()`. Note `use std::time::{Duration, Instant}` import needs updating.
- `crates/smelt-cli/src/serve/http_api.rs` — `JobStateResponse::from(&QueuedJob)` computes `queued_age_secs: job.queued_at.elapsed().as_secs()` (line 53) and `elapsed_secs: job.started_at.map(|t| t.elapsed().as_secs_f64())` (line 54). Both become `elapsed_secs_since(epoch)` calls. The `queued_age_secs` field semantics stay the same (age from enqueue time), `elapsed_secs` stays the same (age from start time).
- `crates/smelt-cli/src/serve/tui.rs` — `render()` computes `let elapsed = j.started_at.map(|t| format!("{}s", t.elapsed().as_secs()))` (lines 73-74). Becomes `j.started_at.map(|t| format!("{}s", elapsed_secs_since(t) as u64))`.
- `crates/smelt-cli/src/serve/tests.rs` — `test_tui_render_no_panic` directly constructs `QueuedJob` with `queued_at: Instant::now(), started_at: Some(Instant::now())` (lines 675-676). Must be updated to `queued_at: now_epoch(), started_at: Some(now_epoch())`. All other tests enqueue via `ServerState::enqueue()` and never touch timing fields directly — they are unaffected.

## Constraints

- `std::time::Instant` is not `Serialize` or `Deserialize` and cannot be made so without a wrapper. Must be removed.
- `u64` epoch seconds is the only TOML-native time representation — `f64` would lose precision for timestamps past 2038, `String` would need a custom parser. `u64` is the correct choice.
- `UNIX_EPOCH` is `1970-01-01T00:00:00Z`; `SystemTime::now().duration_since(UNIX_EPOCH)` returns `Ok(Duration)` on all modern systems. The `unwrap_or_default()` guard is sufficient for the epoch-before-1970 edge case (which cannot happen on any real system running this code).
- `JobId` currently has `#[derive(Clone, Debug, PartialEq, Eq, Hash)]` but no serde. It wraps a `String`. Adding `#[derive(Serialize, Deserialize)]` with `#[serde(transparent)]` makes it serialize as a plain string (most TOML-friendly). Without `#[serde(transparent)]`, serde would emit `{ "0": "job-1" }` for newtype wrappers in some formats — use `transparent` for TOML compatibility.
- `QueuedJob.manifest_path: PathBuf` — `PathBuf` has native serde support (serializes as a string). No special handling needed.
- The `deny_unknown_fields` attribute must NOT be added to `QueuedJob` or any on-disk representation (D017 only applies to manifest structs; the context doc explicitly says no `deny_unknown_fields` on the persisted job format for forward compatibility).

## Common Pitfalls

- **`SystemTime` as field type** — `SystemTime` has no serde impl in `std`; requires the `serde` crate's `derive` feature or the `time` crate. Don't use it as the field type. Store `u64` epoch seconds.
- **Integer overflow in elapsed computation** — `now_epoch_secs - stored_epoch_secs` panics on underflow if clock is adjusted backwards. Use `saturating_sub()`.
- **`Instant` still imported in dispatch.rs after the change** — `dispatch.rs` uses `use std::time::{Duration, Instant}`. After removing `Instant::now()`, `Instant` is no longer used; the compiler will warn. Remove the import.
- **`test_tui_render_no_panic` constructs `QueuedJob` directly** — This is the only test that directly constructs `QueuedJob` with `Instant` fields. It must be updated or the build breaks. No other tests are affected.
- **`queued_age_secs` field in `JobStateResponse`** — This field is documented as "seconds since enqueued" (an *age*, not a timestamp). The computation changes from `job.queued_at.elapsed().as_secs()` to `now_epoch().saturating_sub(job.queued_at)`. The semantics are identical; only the implementation changes.
- **`f64` vs `u64` for `elapsed_secs`** — `http_api.rs` currently exposes `elapsed_secs: Option<f64>` (sub-second precision). After the migration, the stored value is a `u64` epoch. `elapsed_secs_since(epoch) as f64` gives sub-second precision from `SystemTime::now()` if computed as `Duration::as_secs_f64()` — preserve this. Compute: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() - epoch as f64`.

## Open Risks

- **Clock skew / backwards clock**: If the system clock is set backwards between `queued_at` capture and elapsed computation, `saturating_sub` will return 0 instead of a negative value. This is the correct behaviour for a display field (shows "0s elapsed" instead of panicking). Not a correctness risk.
- **`JobId` serde representation**: If `#[serde(transparent)]` is not added to `JobId`, toml serialization will produce a struct form `{ "0": "job-1" }` which is valid but unexpected. Adding `transparent` is the correct move. Verify after implementation by running a round-trip test.
- **No serde round-trip test for `QueuedJob` in S01**: The slice's verification criterion is that all 19 existing tests still pass + HTTP elapsed and TUI elapsed show correct values. A TOML round-trip test for `QueuedJob` would be a bonus but is not required until S02 uses the type in a state file.
- **`manifest_path` in TOML**: `PathBuf` serializes as a string on all platforms. On Windows it uses backslashes. Not a risk for this codebase (Linux/macOS target) but worth noting in case CI ever runs on Windows.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust serde | (standard library knowledge — no skill needed) | none needed |

## Sources

- Codebase inspection: `crates/smelt-cli/src/serve/` — all 9 files read; 19 serve tests confirmed green before changes
- D110 (decisions register): u64 Unix epoch seconds is the decided representation; `elapsed_secs()` helper mandated
- D109: re-queue semantics for Dispatching/Running at restart (S03 concern, not S01)
- M007-CONTEXT.md: identified files, constraints, and integration points
