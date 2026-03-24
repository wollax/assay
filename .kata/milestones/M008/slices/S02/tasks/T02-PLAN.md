---
estimated_steps: 7
estimated_files: 5
---

# T02: TuiEvent variant + polling thread + App state + dashboard badge rendering

**Slice:** S02 — TUI PR status panel with background polling
**Milestone:** M008

## Description

Wire `pr_status_poll` into the TUI event loop. Add `TuiEvent::PrStatusUpdate` for channel delivery, store results on `App`, spawn a background polling thread in `main.rs`, and render PR status badges in the dashboard milestone list.

## Steps

1. In `crates/assay-tui/src/event.rs`, add `PrStatusUpdate { slug: String, info: assay_core::pr::PrStatusInfo }` variant to the `TuiEvent` enum. This requires adding `assay-core` as a dependency of `assay-tui` — verify it's already in `Cargo.toml` (it should be, since `app.rs` already imports `assay_core`).
2. In `crates/assay-tui/src/app.rs`, add to `App`:
   - `pub pr_statuses: std::collections::HashMap<String, assay_core::pr::PrStatusInfo>`
   - `pub poll_targets: std::sync::Arc<std::sync::Mutex<Vec<(String, u64)>>>`
   Initialize `pr_statuses` as empty HashMap and `poll_targets` as Arc::new(Mutex::new(vec)) populated from `self.milestones` (filter where `pr_number.is_some()`, collect `(slug, pr_number.unwrap())`).
3. Add `App::handle_pr_status_update(&mut self, slug: String, info: PrStatusInfo)` — simply inserts into `self.pr_statuses`.
4. Add a helper method `App::refresh_poll_targets(&self)` that locks `self.poll_targets` and replaces contents with current `self.milestones` filtered by `pr_number.is_some()`. Call this everywhere `self.milestones` is refreshed: in `handle_agent_done`, after wizard submit success, and in `with_project_root` initialization.
5. In `crates/assay-tui/src/main.rs` `run()`:
   - After creating the App, check `gh` availability: `Command::new("gh").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()`.
   - If `gh` is available AND `poll_targets` is non-empty, spawn a polling thread: clone `tx` (the `mpsc::Sender<TuiEvent>`) and `app.poll_targets`. The thread loops: sleep 60s → lock targets → for each `(slug, pr_number)`: call `pr_status_poll(pr_number)`, on `Ok(info)` send `TuiEvent::PrStatusUpdate { slug, info }`, on `Err` skip silently. Wrap the loop body in `std::panic::catch_unwind` for defense.
   - Add `TuiEvent::PrStatusUpdate { slug, info }` match arm in the main event loop → calls `app.handle_pr_status_update(slug, info)`.
   - Do an initial poll immediately (sleep 0 on first iteration) so the badge appears without waiting 60s.
6. In `draw_dashboard` in `app.rs`, modify the `ListItem` construction: after the status label and milestone name, check if `pr_statuses` contains the milestone slug. If so, append a badge span:
   - `PrStatusState::Open` → `🟢 OPEN` (green)
   - `PrStatusState::Merged` → `🟣 MERGED` (magenta)
   - `PrStatusState::Closed` → `🔴 CLOSED` (red)
   - If ci_pass + ci_fail + ci_pending > 0, append ` ✓{ci_pass}/{total}` or `✗{ci_fail} fail` summary
   - If review_decision is non-empty, append abbreviated review status
   Update `draw_dashboard` signature to accept `&HashMap<String, PrStatusInfo>` as a new parameter (consistent with D097 — pass individual fields, not `&mut App`). Update the call site in `App::draw()`.
7. Run `cargo build -p assay-tui`, `cargo clippy -p assay-tui -- -D warnings`, `cargo fmt --check`. Fix any issues.

## Must-Haves

- [ ] `TuiEvent::PrStatusUpdate` variant in event.rs
- [ ] `App.pr_statuses` HashMap populated by `handle_pr_status_update`
- [ ] `App.poll_targets` Arc<Mutex> shared between App and polling thread
- [ ] `refresh_poll_targets` called on every milestone refresh path
- [ ] Background polling thread spawned in `run()` when `gh` is available
- [ ] Initial poll with no delay (badge appears within seconds of TUI launch)
- [ ] Dashboard badge rendered with state icon + CI summary for milestones with PR status
- [ ] `gh` not found → no thread spawned, no crash, no badge
- [ ] `draw_dashboard` accepts `pr_statuses` as parameter (D097 pattern)

## Verification

- `cargo build -p assay-tui` — compiles clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- Existing tests pass: `cargo test -p assay-tui` — no regressions

## Observability Impact

- Signals added/changed: `tracing::debug` in polling thread on each cycle; `tracing::warn` when `gh` not found at startup
- How a future agent inspects this: `App.pr_statuses` is pub — integration tests read it directly; `App.poll_targets` shows which milestones are being polled
- Failure state exposed: polling errors silently skipped (by design — no badge is the degradation signal)

## Inputs

- T01 output: `PrStatusInfo`, `PrStatusState`, `pr_status_poll()` in `assay-core::pr`
- `crates/assay-tui/src/event.rs` — existing `TuiEvent` enum (D107/D114)
- `crates/assay-tui/src/app.rs` — existing `App` struct, `draw_dashboard`, `handle_agent_done`
- `crates/assay-tui/src/main.rs` — existing `run()` event loop with crossterm thread pattern
- D097: screen render fns take individual fields, not `&mut App`
- D107: channel-based event loop, clone `tx` for background threads

## Expected Output

- `crates/assay-tui/src/event.rs` — `PrStatusUpdate` variant added
- `crates/assay-tui/src/app.rs` — `pr_statuses`, `poll_targets`, `handle_pr_status_update`, `refresh_poll_targets`, badge in `draw_dashboard`
- `crates/assay-tui/src/main.rs` — `gh` check, polling thread spawn, `PrStatusUpdate` dispatch
