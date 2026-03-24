---
id: S02
parent: M008
milestone: M008
provides:
  - PrStatusState enum (Open/Merged/Closed) with serde UPPERCASE rename
  - PrStatusInfo struct with state, CI pass/fail/pending counts, review_decision
  - pr_status_poll(pr_number) -> Result<PrStatusInfo> shelling out to gh pr view --json
  - TuiEvent::PrStatusUpdate variant for channel-based polling delivery
  - Background polling thread with initial-poll-no-delay and 60s interval
  - Dashboard PR badge rendering (state icon + CI summary + review abbreviation)
  - Arc<Mutex<Vec<(String, u64)>>> shared poll targets refreshed on milestone reload
requires:
  - slice: S01
    provides: Milestone.pr_number field for filtering which milestones to poll
affects:
  - S05
key_files:
  - crates/assay-core/src/pr.rs
  - crates/assay-core/tests/pr_status.rs
  - crates/assay-tui/src/event.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/tests/pr_status_panel.rs
key_decisions:
  - "D122: PrStatusInfo lives in assay-core::pr, not assay-types (display-only view type)"
  - "D123: Poll interval hardcoded as const (60s), not configurable via Config"
  - "D124: Arc<Mutex<Vec>> shared state between App and polling thread"
  - "D125: eprintln for gh-not-found warning (tracing not a dep of assay-tui)"
patterns_established:
  - "Background thread + TuiEvent channel pattern for subprocess polling (reusable for future polling needs)"
  - "catch_unwind wrapping each poll iteration for defense against panics in background thread"
  - "write_milestone_toml raw-TOML helper in tests to avoid chrono dev-dep in assay-tui"
observability_surfaces:
  - "App.pr_statuses is pub — integration tests and future slash commands read it directly"
  - "App.poll_targets is pub — shows which milestones are being polled"
  - "eprintln warning when gh CLI not found at startup"
  - "Absent badge is the graceful degradation signal (polling errors silently skipped)"
drill_down_paths:
  - .kata/milestones/M008/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M008/slices/S02/tasks/T03-SUMMARY.md
duration: 40min
verification_result: passed
completed_at: 2026-03-23T12:30:00Z
---

# S02: TUI PR status panel with background polling

**Background-polled PR status badges on TUI dashboard with CI check counts, review status, and graceful gh-missing degradation**

## What Happened

Built a three-layer PR status feature for the TUI dashboard:

**T01 — Core polling function:** Added `PrStatusState` enum and `PrStatusInfo` struct to `assay-core::pr`. Implemented `pr_status_poll(pr_number)` which shells out to `gh pr view <n> --json state,statusCheckRollup,reviewDecision` and parses CI check conclusions (SUCCESS→pass, FAILURE/CANCELLED→fail, null/IN_PROGRESS→pending). Internal `RawPrStatus`/`RawStatusCheck` structs handle serde deserialization. 8 integration tests cover all state combinations including malformed JSON, non-zero exit, and CANCELLED-as-failure.

**T02 — TUI wiring:** Added `TuiEvent::PrStatusUpdate` to the event loop. Extended `App` with `pr_statuses: HashMap<String, PrStatusInfo>` and `poll_targets: Arc<Mutex<Vec<(String, u64)>>>`. The polling thread spawns only when `gh --version` succeeds, does an initial poll immediately (no delay on first cycle), then polls every 60s. Each iteration is wrapped in `catch_unwind` for defense. Dashboard rendering appends a PR badge (🟢/🟣/🔴 + CI counts + review abbreviation) when status data is available.

**T03 — Integration tests:** 3 tests in `pr_status_panel.rs` proving event→state storage, poll target initialization from milestones with `pr_number`, and target refresh after `handle_agent_done` milestone reload. Used raw TOML writing to avoid chrono dev-dep.

## Verification

- `cargo test -p assay-core --test pr_status` — 8/8 pass
- `cargo test -p assay-tui --test pr_status_panel` — 3/3 pass
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean

## Requirements Advanced

- R058 (Advanced PR workflow) — TUI PR status panel now shows live PR status badges. S01 delivered the creation side (labels, reviewers, templates); S02 completes the visibility side (status polling + dashboard rendering). R058 is now fully validated.

## Requirements Validated

- R058 — PR creation with labels/reviewers/templates (S01) + TUI PR status panel with background polling (S02). Both halves proven by integration tests with mock `gh` binary.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- Used `eprintln!` instead of `tracing::warn`/`tracing::debug` because `tracing` is not a dependency of `assay-tui` (D125). Consistent with existing pattern.
- Added 3 extra tests in T01 beyond the 5 planned (non-zero exit, malformed JSON, CANCELLED conclusion) — no plan conflict, just better coverage.
- Used raw TOML string writing in T03 instead of `milestone_save` to avoid pulling `chrono` into assay-tui dev-deps.

## Known Limitations

- Poll interval is hardcoded at 60s (D123) — not configurable via Config. Trivial to promote later.
- Polling errors are silently skipped — absent badge is the degradation signal. No per-error diagnostic exposed to the user.
- Review decision stored as raw String, not enum — `gh` may add new values in the future.

## Follow-ups

- None — S03 (OpenCode plugin), S04 (analytics), S05 (TUI analytics) are independent or downstream.

## Files Created/Modified

- `crates/assay-core/src/pr.rs` — Added PrStatusState, PrStatusInfo, pr_status_poll(), parse_pr_status_json(), RawPrStatus, RawStatusCheck
- `crates/assay-core/tests/pr_status.rs` — 8 integration tests for all status parsing scenarios
- `crates/assay-tui/src/event.rs` — Added PrStatusUpdate variant with PrStatusInfo import
- `crates/assay-tui/src/app.rs` — Added pr_statuses, poll_targets, handle_pr_status_update, refresh_poll_targets; updated draw_dashboard with PR badge rendering
- `crates/assay-tui/src/main.rs` — Added gh availability check, background polling thread, PrStatusUpdate dispatch
- `crates/assay-tui/tests/pr_status_panel.rs` — 3 integration tests for PR status panel mechanics

## Forward Intelligence

### What the next slice should know
- The TuiEvent channel pattern (D107) is well-established — any future background feature should use the same `tx.send(TuiEvent::Variant)` pattern.
- `draw_dashboard` now accepts `&HashMap<String, PrStatusInfo>` as an extra parameter (D097 pattern) — future dashboard additions should follow the same field-passing approach.

### What's fragile
- The `TuiEvent` enum is duplicated between `event.rs` and `app.rs` — adding a variant requires updating both files. This is a known tech debt from M007 (D114).

### Authoritative diagnostics
- `cargo test -p assay-core --test pr_status` — all 8 tests exercise the parsing layer; failures here mean the gh JSON contract changed.
- `cargo test -p assay-tui --test pr_status_panel` — all 3 tests exercise the TUI wiring; failures here mean App state management broke.

### What assumptions changed
- No assumptions changed — the polling thread + channel pattern worked as planned. The `gh` subprocess is fast enough (~200-500ms) that sequential polling of multiple milestones is acceptable.
