---
id: T02
parent: S01
milestone: M006
provides:
  - "compute_gate_data() — loads latest gate run per chunk slug, accumulates passed/failed across all chunks in a milestone, silently degrades on all errors"
  - "App::new() wired to compute_gate_data() — gate_data field fully populated from .assay/results/ on construction"
  - "Polling event loop in run() — event::poll(250ms) replaces blocking event::read(), allows future background refresh"
  - "handle_event() extended — Char('Q'), Esc, and Event::Resize handled; list_state.select_previous/next used (native ratatui API)"
  - "draw_dashboard() with proper 3-zone layout — header (bold title), body (milestone list with status badge + chunk fraction + gate counts), footer (keybindings)"
  - "draw_no_milestones() — renders 'No milestones — run `assay plan`' in body area"
  - "draw_no_project() updated — message updated to 'Not an Assay project — run `assay init` first'"
  - "status_badge() — maps MilestoneStatus to [Draft]/[InProgress]/[Verify]/[Complete] strings"
  - "test_gate_data_loaded_from_history — uses milestone_save + history::save_run to write fixtures, asserts gate_data from App::new()"
key_files:
  - crates/assay-tui/src/app.rs
key_decisions:
  - "Used list_state.select_previous()/select_next() (ratatui built-in) instead of manual index arithmetic — simpler and wraps correctly"
  - "#[allow(dead_code)] on project_root and config fields — they are valid App state preserved for S02+, suppressing the warning is correct"
  - "gate_data parallel-indexed with milestones (milestones.iter().zip(gate_data.iter())) — requires compute_gate_data to produce entries in the same order as milestone_scan; both use milestone slug sort, so order is stable"
patterns_established:
  - "compute_gate_data(assay_dir, milestones) is the canonical function for loading gate history — takes a slice ref, returns parallel Vec"
  - "All TUI data load errors (history::list, history::load, config::load, milestone_scan) are silently degraded — TUI never panics on bad data"
  - "draw_dashboard uses render_stateful_widget (not render_widget) for the milestone List — required for selection state to be respected"
duration: 25min
verification_result: passed
completed_at: 2026-03-20T13:53:00Z
blocker_discovered: false
---

# T02: Dashboard rendering, event loop, and gate data — all tests pass

**Full `assay-tui` app: polling event loop, live gate history per milestone, 3-zone dashboard, 5 unit tests green, binary present.**

## What Happened

T01 had already scaffolded `App::new()` with silent degradation and the basic draw functions. T02 completed the remaining behavior:

**`compute_gate_data()`:** Iterates each milestone's `chunks`, calls `history::list()` for each chunk slug, takes the last run_id, calls `history::load()`, and accumulates `summary.passed` + `summary.failed`. All errors (no results dir, bad JSON) are swallowed and produce zero counts. Result is a `Vec<(String, GateSummary)>` parallel-indexed with the milestones vec.

**`run()` polling loop:** Replaced blocking `event::read()` with `event::poll(Duration::from_millis(250))` + conditional `event::read()`. The 250ms poll interval keeps the UI responsive and leaves headroom for future background data refresh.

**`handle_event()`:** Extended to handle `Char('Q')` and `Esc` as quit keys (in addition to `'q'`). `Event::Resize` explicitly returns `false` (ignore). Used ratatui's `list_state.select_previous()` / `select_next()` rather than manual index arithmetic.

**`draw_dashboard()`:** Three-zone layout via `Layout::vertical([Length(1), Fill(1), Length(1)])`. Header: bold "Assay Dashboard". Footer: keybinding hint. Body: milestone list with `format!("{:<30} {:<12} {}/{:<5} ✓{} ✗{}", name, badge, done, total, passed, failed)`. Uses `render_stateful_widget` so selection highlight is active.

**`draw_no_project()`:** Updated message to "Not an Assay project — run `assay init` first" (removed the diagnostic path detail, replaced with actionable instruction).

**`draw_no_milestones()`:** Simple paragraph in the body area.

**`test_gate_data_loaded_from_history`:** Creates a TempDir, writes a milestone with one chunk via `milestone_save`, writes a gate run record via `history::save_run` with `passed: 2, failed: 1`, constructs `App::new()`, and asserts `gate_data` has the correct counts for the milestone slug. Covers the full history loading path.

## Verification

- `cargo test -p assay-tui` — 5 tests, all pass:
  - `test_no_assay_dir_sets_no_project_screen` ✓
  - `test_handle_event_q_returns_true` ✓
  - `test_handle_event_up_down_no_panic_on_empty` ✓
  - `test_handle_event_down_moves_selection` ✓
  - `test_gate_data_loaded_from_history` ✓
- `cargo build -p assay-tui && ls -la target/debug/assay-tui` — binary present (11.9 MB)
- `cargo build -p assay-cli && ls -la target/debug/assay` — CLI binary present, no collision
- `cargo fmt --all -- --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — all 769+ tests pass
- `cargo deny check` — fails on pre-existing `aws-lc-sys` vulnerabilities (RUSTSEC-2026-0044 through -0048) in transitive dev-only dependency of `assay-types`; unrelated to T02 changes; was failing before T01 as well

## Diagnostics

- `app.screen` variant is the state machine root: `Screen::NoProject` triggers the "run assay init" message; `Screen::Dashboard` triggers the milestone list
- `app.milestones.len()` and `app.gate_data.len()` should always be equal; if they diverge, `draw_dashboard` will panic on zip iteration — invariant is maintained by `compute_gate_data` which iterates the same `milestones` slice
- `app.gate_data[i].1.passed + failed == 0` means the milestone has no gate history (either no chunks, or no results dir for those chunks) — this is the normal state for new projects
- Silent degradation is by design: a future agent sees an empty dashboard and checks `.assay/results/` directly to diagnose missing history

## Deviations

- `draw_no_project()` message changed from T01's "Not an Assay project — no .assay/ directory found.\nPress q to quit." to T02's plan's "Not an Assay project — run `assay init` first" — actionable instruction is more useful; the `q` to quit hint is now in the footer of draw_dashboard (not needed for NoProject since q handling is unconditional).

## Known Issues

- `cargo deny check` fails on pre-existing `aws-lc-sys` vulnerabilities (RUSTSEC-2026-0044..0048) — these are in `reqwest → rustls → aws-lc-rs` chain pulled by `jsonschema` which is only a dev-dependency of `assay-types`. Not introduced by this task. Fix requires upgrading `aws-lc-sys` to ≥0.39.0 workspace-wide.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Full implementation: compute_gate_data(), App::new() with gate data, polling run(), extended handle_event(), draw_dashboard() 3-zone layout, draw_no_milestones(), updated draw_no_project(), status_badge(), test_gate_data_loaded_from_history
