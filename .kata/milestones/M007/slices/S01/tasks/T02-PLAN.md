---
estimated_steps: 5
estimated_files: 3
---

# T02: Implement `handle_agent_line`, `handle_agent_done`, and `draw_agent_run`

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

This task replaces the T01 stubs with real implementations for `handle_agent_line`, `handle_agent_done`, and the `draw_agent_run` renderer. After this task, the 8 integration tests in `agent_run.rs` all pass. The main loop refactor (T03) is separate — this task only touches `app.rs` and makes the App state machine correct.

Key constraints:
- `handle_agent_line` caps lines at 10 000 to prevent unbounded memory growth (per research doc).
- `handle_agent_done` must refresh milestones and cycle_slug from disk using the same patterns already in `with_project_root` (guard on `project_root`; use `.ok()` to degrade gracefully on I/O error).
- `draw_agent_run` must handle `lines.is_empty()` gracefully — show "Starting…" rather than panic.
- The `Screen::AgentRun` arm in `handle_event()` needs real scroll + Esc wiring.
- The stub `draw_agent_run_stub` from T01 is replaced by the real `draw_agent_run` free function (consistent with D097/D105 pattern: free fn, explicit `area: Rect` parameter).

## Steps

1. **Implement `handle_agent_line`**
   - If `self.screen` is `Screen::AgentRun { ref mut lines, .. }`: push `line` to `lines`; if `lines.len() > 10_000`, remove the first element (`lines.remove(0)` or use `Vec::drain` for efficiency — a `VecDeque<String>` is more efficient but the research doc accepts `Vec` with a cap; use `Vec` for simplicity, trim from front)
   - All other screen variants: no-op (return early)

2. **Implement `handle_agent_done`**
   - If `self.screen` is `Screen::AgentRun { ref mut status, .. }`: set `*status = if exit_code == 0 { AgentRunStatus::Done { exit_code } } else { AgentRunStatus::Failed { exit_code } }`
   - Then refresh disk state: `if let Some(ref root) = self.project_root { let assay_dir = root.join(".assay"); if let Ok(ms) = milestone_scan(&assay_dir) { self.milestones = ms; } self.cycle_slug = cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug); }`
   - Keep it at this; `detail_run` refresh deferred (the user can press `Enter` to re-navigate)

3. **Implement `draw_agent_run` free function**
   - Signature: `fn draw_agent_run(frame: &mut Frame, area: Rect, chunk_slug: &str, lines: &[String], scroll_offset: usize, status: &AgentRunStatus)`
   - Split `area` into content rows (area.height - 2) and a 1-row status line at bottom, 1-row title at top
   - Render a bordered Block with title `"Agent Run: {chunk_slug}"`
   - If `lines.is_empty()`: display a single "Starting…" item styled dim
   - Otherwise: compute the visible window as `lines[safe_start..safe_end]` where `safe_start = scroll_offset.min(lines.len().saturating_sub(1))` and `safe_end = (safe_start + visible_rows).min(lines.len())`; render each as a `ListItem`
   - Status line: match `status` → `"● Running…"` (yellow), `"✓ Done (exit {n})"` (green, n==0), `"✗ Failed (exit {n})"` (red); append `"  Esc: back"` hint
   - Remove `draw_agent_run_stub` and update the `draw()` match arm to call the real function with field references

4. **Implement `Screen::AgentRun` arm in `handle_event()`**
   - Replace the stub `Screen::AgentRun { .. } => false` with:
     ```
     Screen::AgentRun { ref mut scroll_offset, .. } => {
         match key.code {
             KeyCode::Esc => { self.screen = Screen::Dashboard; }
             KeyCode::Down | KeyCode::Char('j') => { *scroll_offset = scroll_offset.saturating_add(1); }
             KeyCode::Up | KeyCode::Char('k') => { *scroll_offset = scroll_offset.saturating_sub(1); }
             _ => {}
         }
         false
     }
     ```
   - Use the clone-then-mutate pattern from D098 if the borrow checker requires it (only needed if accessing other `self` fields during the arm; scroll is self-contained so direct mutation should work)

5. **Update `draw()` match arm for `Screen::AgentRun`**
   - Destructure `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` from `&self.screen` (use `..` trick per D098 if borrow splitting is needed)
   - Call `draw_agent_run(frame, content_area, chunk_slug, lines, *scroll_offset, status)`

## Must-Haves

- [ ] `handle_agent_line` appends to `Screen::AgentRun.lines` and caps at 10 000
- [ ] `handle_agent_line` is a no-op on all non-`AgentRun` screens (no panic)
- [ ] `handle_agent_done` sets `AgentRunStatus::Done` for exit_code == 0, `Failed` for exit_code != 0
- [ ] `handle_agent_done` refreshes `self.milestones` and `self.cycle_slug` from disk (graceful `.ok()` degradation)
- [ ] `draw_agent_run` handles `lines.is_empty()` (shows "Starting…")
- [ ] `Screen::AgentRun` `handle_event` arm: `Esc` → `Screen::Dashboard`, `j`/`↓` → scroll down, `k`/`↑` → scroll up
- [ ] All 8 agent_run integration tests pass
- [ ] All 27 pre-existing TUI tests still pass

## Verification

```bash
# All 8 new agent_run tests pass
cargo test -p assay-tui --test agent_run

# All pre-existing tests unchanged
cargo test -p assay-tui

# No compile errors
cargo check -p assay-tui
```

## Observability Impact

- Signals added/changed: `Screen::AgentRun.lines` now accumulates real content; `Screen::AgentRun.status` reflects `AgentRunStatus::Done`/`Failed` — a future agent can inspect `app.screen` for post-run diagnostics.
- How a future agent inspects this: `match app.screen { Screen::AgentRun { ref lines, ref status, .. } => … }` to read the full stdout buffer and exit status.
- Failure state exposed: `AgentRunStatus::Failed { exit_code }` is visible in the TUI and in test assertions; non-zero exit codes are preserved and rendered.

## Inputs

- `crates/assay-tui/src/app.rs` (T01 output) — stub `handle_agent_line`, `handle_agent_done`, stub `draw_agent_run_stub` to replace
- `crates/assay-tui/tests/agent_run.rs` (T01 output) — failing tests to make pass
- D097/D105 — draw functions take individual fields, not `&mut App`; `area: Rect` passed explicitly
- D098 — `..` pattern in draw() match arms; clone-then-mutate in handle_event() for borrow splitting

## Expected Output

- `crates/assay-tui/src/app.rs` — real `handle_agent_line`, `handle_agent_done`, `draw_agent_run` (free fn), updated `draw()` and `handle_event()` arms
- All 8 `tests/agent_run.rs` tests green
