---
id: T02
parent: S05
milestone: M012
provides:
  - AnyTrackerSource enum dispatching to GitHub, Linear, Mock(test-only) variants
  - TrackerPoller struct with run() loop (ensure_labels → interval poll → transition → enqueue)
  - D157 double-dispatch prevention (Ready→Queued before enqueue)
  - D105 temp file pattern (NamedTempFile + std::mem::forget)
  - build_manifest_toml() for serializing JobManifest via toml::Value manipulation
key_files:
  - crates/smelt-cli/src/serve/tracker_poller.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "JobManifest lacks Serialize — used toml::Value manipulation to build TOML from raw template string + injected session fields"
patterns_established:
  - "D084 enum dispatch pattern: AnyTrackerSource wraps concrete generic types to avoid non-object-safe RPITIT trait"
  - "Template TOML stored as raw string alongside parsed JobManifest for serialization roundtrip"
observability_surfaces:
  - "tracing::info! at poller startup (provider, interval_secs)"
  - "tracing::debug! per poll cycle (issues_found count)"
  - "tracing::warn! on poll error, transition error, manifest generation error"
  - "tracing::info! on successful enqueue (issue_id, job_id)"
duration: 15min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T02: TrackerPoller struct and AnyTrackerSource enum

**Created TrackerPoller background task with AnyTrackerSource enum dispatcher, D157 transition-before-enqueue, D105 temp file pattern, and 6 unit tests**

## What Happened

Built `tracker_poller.rs` with two main components:

1. **AnyTrackerSource enum** — dispatches `poll_ready_issues()`, `transition_state()`, and `ensure_labels()` to concrete implementations (GitHub, Linear, Mock). The Mock variant is gated behind `#[cfg(test)]`. This solves the RPITIT non-object-safe trait problem (D084).

2. **TrackerPoller struct** — holds the source, template manifest, config, shared state, cancellation token, and poll interval. `run()` calls `ensure_labels()` once (fatal on error), then enters a `tokio::select!` loop with `tokio::time::interval` (MissedTickBehavior::Skip) vs cancellation. Each tick calls `poll_once()` which: polls for ready issues (warn+continue on error), transitions each issue Ready→Queued before enqueue (D157, warn+skip on error), generates a manifest via `issue_to_manifest()`, writes it to a temp file using `toml::Value` manipulation (since JobManifest lacks Serialize), leaks the TempPath via `std::mem::forget` (D105), and enqueues into ServerState.

The `template_toml` field stores the raw TOML string alongside the parsed `JobManifest` to enable serialization roundtrips without requiring `Serialize` on the manifest types.

## Verification

- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — 6 tests pass
- `cargo test --workspace` — 396 passed, 0 failed, 11 ignored, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings

### Test coverage:
| # | Test | Status |
|---|------|--------|
| 1 | test_poller_enqueues_issues (2 issues → 2 Queued jobs) | ✓ PASS |
| 2 | test_poller_skips_on_transition_error (1 issue + Err → 0 jobs) | ✓ PASS |
| 3 | test_poller_continues_on_poll_error (Err then Ok → 1 job) | ✓ PASS |
| 4 | test_poller_exits_on_cancellation (cancel before run → Ok) | ✓ PASS |
| 5 | test_build_manifest_toml_roundtrips (TOML → parse → verify) | ✓ PASS |
| 6 | test_write_manifest_temp_creates_file (D105 persist check) | ✓ PASS |

### Slice-level checks:
- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — ✓ PASS
- `cargo test --workspace` — ✓ PASS (396 passed)
- `cargo clippy --workspace -- -D warnings` — ✓ PASS (zero warnings)
- TUI tests, docs — not yet applicable (later tasks)

## Diagnostics

- `SMELT_LOG=debug` shows every poll cycle with issue count
- `SMELT_LOG=warn` shows only poll/transition/manifest errors
- `SMELT_LOG=info` shows poller startup (provider, interval) and each successful enqueue (issue_id, job_id)
- `ensure_labels()` failure at startup propagates as fatal error (poller doesn't start)

## Deviations

- Added `template_toml: String` field to TrackerPoller (not in original plan) — required because `JobManifest` doesn't implement `Serialize`, so the raw TOML template must be preserved for `toml::Value` manipulation during manifest serialization.
- Added `build_manifest_toml()` helper and `write_manifest_temp()` helper as standalone functions rather than methods on TrackerPoller — cleaner separation and easier to test independently.
- Used `#[allow(dead_code)]` and `#[allow(unused_imports)]` annotations since the types are not yet wired into `smelt serve` (T05). This is expected — the annotations reference T05 for traceability.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/tracker_poller.rs` — New file: AnyTrackerSource enum, TrackerPoller struct with run()/poll_once(), build_manifest_toml(), write_manifest_temp(), 6 unit tests
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub(crate) mod tracker_poller` and re-export of `AnyTrackerSource`, `TrackerPoller`
