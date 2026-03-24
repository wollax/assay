---
id: T02
parent: S02
milestone: M008
provides:
  - TuiEvent::PrStatusUpdate variant in event.rs and app.rs
  - App.pr_statuses HashMap<String, PrStatusInfo> for cached PR badge data
  - App.poll_targets Arc<Mutex<Vec<(String, u64)>>> shared with polling thread
  - refresh_poll_targets() method called on every milestone reload path
  - Background polling thread in main.rs with initial-poll-no-delay and 60s interval
  - PR status badge rendering in draw_dashboard (state icon + CI summary + review status)
  - Graceful degradation when gh is missing (eprintln warning, no thread spawn, no crash)
key_files:
  - crates/assay-tui/src/event.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - "Used eprintln for gh-not-found warning instead of tracing (tracing not a dep of assay-tui)"
  - "Polling thread does initial poll immediately (first=true flag skips sleep) so badge appears within seconds"
  - "draw_dashboard accepts &HashMap<String, PrStatusInfo> as parameter per D097 pattern"
  - "Review decision abbreviated: APPROVED→✓rvw, CHANGES_REQUESTED→△rvw, REVIEW_REQUIRED→?rvw"
patterns_established:
  - "Arc<Mutex<Vec>> shared state between App and background thread, updated via refresh_poll_targets"
  - "std::panic::catch_unwind wrapping each poll iteration for defense against panics in background thread"
observability_surfaces:
  - "App.pr_statuses is pub — integration tests and future slash commands can read it directly"
  - "App.poll_targets is pub — shows which milestones are being polled"
  - "eprintln warning when gh CLI not found at startup"
  - "Polling errors silently skipped — absent badge is the degradation signal"
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: TuiEvent variant + polling thread + App state + dashboard badge rendering

**Wired pr_status_poll into TUI event loop with background polling thread, shared poll targets, and dashboard PR badge rendering with state/CI/review indicators**

## What Happened

Added `TuiEvent::PrStatusUpdate { slug, info }` variant to both `event.rs` and the duplicate `TuiEvent` in `app.rs`. Extended `App` with `pr_statuses: HashMap<String, PrStatusInfo>` and `poll_targets: Arc<Mutex<Vec<(String, u64)>>>`. The poll targets are initialized from milestones with `pr_number` in `with_project_root` and refreshed via `refresh_poll_targets()` in `handle_agent_done` and wizard submit success paths.

In `main.rs`, the polling thread is spawned only when `gh --version` succeeds and poll targets are non-empty. The thread does an initial poll immediately (no sleep on first iteration), then sleeps 60s between subsequent cycles. Each poll iteration locks the shared targets, calls `pr_status_poll` per target, and sends `PrStatusUpdate` events on success. Errors are silently skipped. Each iteration body is wrapped in `catch_unwind` for defense.

The `draw_dashboard` function now accepts `&HashMap<String, PrStatusInfo>` (D097 pattern) and appends a PR badge after the milestone name when status data is available: state icon (🟢/🟣/🔴), CI summary (✓pass/total or ✗fail), and abbreviated review decision.

## Verification

- `cargo build -p assay-tui` — compiles clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `cargo test -p assay-tui --lib` — 3/3 pass (no regressions)
- `cargo test -p assay-tui --test agent_run --test help_status --test mcp_panel` — 18/18 pass (no regressions)

### Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `TuiEvent::PrStatusUpdate` variant in event.rs | ✓ PASS | Added with slug + info fields |
| 2 | `App.pr_statuses` HashMap populated by `handle_pr_status_update` | ✓ PASS | HashMap<String, PrStatusInfo> with insert method |
| 3 | `App.poll_targets` Arc<Mutex> shared between App and polling thread | ✓ PASS | Initialized in with_project_root, cloned in main.rs |
| 4 | `refresh_poll_targets` called on every milestone refresh path | ✓ PASS | Called in handle_agent_done and wizard submit |
| 5 | Background polling thread spawned in `run()` when `gh` is available | ✓ PASS | gh --version check + thread::spawn in main.rs |
| 6 | Initial poll with no delay | ✓ PASS | first=true flag skips sleep on first iteration |
| 7 | Dashboard badge rendered with state icon + CI summary | ✓ PASS | draw_dashboard renders spans with color-coded icons |
| 8 | `gh` not found → no thread, no crash, no badge | ✓ PASS | eprintln warning, thread not spawned |
| 9 | `draw_dashboard` accepts `pr_statuses` as parameter (D097) | ✓ PASS | Signature updated, call site passes &self.pr_statuses |

### Slice-Level Verification (partial — T03 remaining)

| Check | Status | Notes |
|-------|--------|-------|
| `cargo test -p assay-core --test pr_status` | ✓ PASS | (T01 — pre-existing) |
| `cargo test -p assay-tui --test pr_status_panel` | ⏳ PENDING | T03 creates this test file |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✓ PASS | Clean |
| `cargo fmt --check` | ✓ PASS | Clean |

## Diagnostics

- `App.pr_statuses` is pub — read directly in integration tests or future slash commands
- `App.poll_targets` is pub — inspect which milestones are being polled
- Absent badge = polling error or gh not found (by design)
- eprintln at startup when gh CLI is missing

## Deviations

- Used `eprintln!` instead of `tracing::warn`/`tracing::debug` because `tracing` is not a dependency of `assay-tui`. Consistent with existing pattern in app.rs (e.g., config load warnings).
- Polling errors use silent skip (no eprintln per error) to avoid spamming terminal restore output. Absent badge is the degradation signal per slice plan.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/event.rs` — Added `PrStatusUpdate` variant with `assay_core::pr::PrStatusInfo` import
- `crates/assay-tui/src/app.rs` — Added `pr_statuses`, `poll_targets`, `handle_pr_status_update`, `refresh_poll_targets`; updated `draw_dashboard` signature and rendering with PR badge spans
- `crates/assay-tui/src/main.rs` — Added gh availability check, background polling thread spawn, `PrStatusUpdate` dispatch in main event loop
