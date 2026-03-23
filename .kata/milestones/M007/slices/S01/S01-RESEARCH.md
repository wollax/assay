# M007/S01 — Channel Event Loop and Agent Run Panel: Research

**Date:** 2026-03-23

## Summary

S01 is the highest-risk slice in M007. The current TUI `run()` loop in `main.rs` uses blocking `event::read()`, which cannot interleave terminal key events with streaming subprocess stdout. The refactor must convert to a channel-based `mpsc::Receiver<TuiEvent>` loop where one background thread pumps crossterm events and a second (when an agent is active) pumps `AgentLine`/`AgentDone` events — all to the same sender. A new `launch_agent_streaming(cli_args, working_dir, line_tx)` free function goes in `assay-core::pipeline` alongside the existing batch `launch_agent()` (which must not change). The `Screen::AgentRun` variant and `draw_agent_run()` render function are entirely new additions to `app.rs`.

The existing codebase is in excellent shape for this work. `main.rs` is only 30 lines; the entire event loop is one `match event::read()?` block that can be replaced wholesale. The `App` struct and `Screen` enum are in `app.rs`, which is a pub lib module — the new `Screen::AgentRun` variant slots in cleanly. The 27 existing TUI tests (all in `tests/`) exercise `App` state directly with synthetic key events, independent of the actual event loop — so the `run()` refactor does not touch them.

The proof strategy from the roadmap is sound and efficient: an integration test drives a mock subprocess (e.g., `echo` on each line then exits), pumps the resulting `TuiEvent::AgentLine` / `TuiEvent::AgentDone` events through the real channel loop, and asserts `Screen::AgentRun` accumulates all lines and transitions to Done — no real terminal required.

## Recommendation

Implement in this order:
1. Add `launch_agent_streaming` to `assay-core::pipeline` with tests (pure Rust, no TUI dependency)
2. Define `TuiEvent` enum in `main.rs`; refactor `run()` to channel-based dispatch
3. Add `Screen::AgentRun`, `AgentStatus`, `draw_agent_run()`, and `r` key handler to `app.rs`
4. Wire `on AgentDone` refresh of `milestones`, `cycle_slug`, `detail_run`
5. Write integration test in `tests/agent_run.rs` with a mock subprocess

Keep `launch_agent()` (batch, blocking) completely untouched — it is used by CLI/MCP callers.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic writes for any new state files | `NamedTempFile::new_in + sync_all + persist` pattern (milestone_save, config_save) | Established D093 pattern; prevents partial writes |
| HarnessProfile construction from config | `build_harness_profile()` in `assay-core::pipeline` | Already handles ManifestSession → HarnessProfile; reuse at TUI agent spawn |
| Cycle status for active chunk | `cycle_status(&assay_dir)` in `assay-core::milestone` | Returns `Option<CycleStatus>` with `active_chunk_slug`; use this to get chunk slug for `Screen::AgentRun` |
| Claude CLI args | `assay_harness::claude::build_cli_args()` | Already builds `["--print", "--output-format", "json", ...]` correctly |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — Current 30-line blocking `run()`. The entire `match event::read()?` block is replaced by `while let Ok(event) = rx.recv()` in the refactor. `Event::Resize` still calls `terminal.clear()`. The background thread for crossterm: `std::thread::spawn(move || loop { if let Ok(e) = event::read() { tx.send(TuiEvent::from(e)); } })`.
- `crates/assay-tui/src/app.rs` — `App` struct and `Screen` enum. New `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentStatus }`. `App` gains `agent_thread: Option<std::thread::JoinHandle<i32>>`. Draw dispatch already uses `match &self.screen` — add `Screen::AgentRun { .. }` arm. D097 pattern: `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` with explicit fields, not `&mut App`.
- `crates/assay-core/src/pipeline.rs` — `launch_agent()` batch impl. The new `launch_agent_streaming` follows the same `std::process::Command::new` + `Stdio::piped()` pattern, but instead of waiting for completion, spawns a thread that reads stdout line-by-line (`BufReader::lines()`) and sends each to `line_tx: mpsc::Sender<String>`, then returns `JoinHandle<i32>` (exit code from the join).
- `crates/assay-tui/tests/` — All 27 existing tests construct `App::with_project_root()` and drive `handle_event()` directly. The `run()` function is not tested at unit level — tests only care about `App` state transitions. This means the `run()` refactor is safely decoupled from existing test coverage.
- `crates/assay-harness/src/claude.rs` — `build_cli_args(config)` returns `Vec<String>` starting with `["--print", "--output-format", "json"]`. At agent spawn from the TUI, we construct a `HarnessWriter` closure, call it to write harness config and get CLI args, then pass args to `launch_agent_streaming`.

## Constraints

- **D001 (zero traits):** `TuiEvent` is an enum, not a trait. `launch_agent_streaming` is a free function with `Sender<String>` parameter. No new traits.
- **D007 (sync core):** `launch_agent_streaming` uses `std::thread::spawn` + `BufReader::lines()` — no tokio, no async. The background thread is `Send`-compatible because `Sender<String>` is `Send`.
- **D108:** `launch_agent()` batch function is left completely untouched. New function is purely additive.
- **D097 (borrow checker pattern):** `draw_agent_run` must accept individual fields, not `&mut App`, to avoid borrow-split on `self.screen` during render. Same pattern as all other draw functions.
- **D105 (layout authority):** `draw_agent_run(frame, area, ...)` receives `content_area` from `App::draw()`'s single layout split; does not call `frame.area()` internally.
- **Bounded vs unbounded channel:** Use `mpsc::channel()` (unbounded). The agent output rate is human-readable text (KB/s); backpressure from a bounded channel would deadlock the TUI render loop. If the channel fills (pathological), the streaming thread blocks — acceptable since the TUI is still rendering from the receiver end.
- **`r` key pre-condition:** Agent spawn only fires when `Screen::Dashboard` is active AND `cycle_status()` returns a non-None active chunk. If either condition fails, the `r` key is a no-op (or shows inline error message in the status bar).
- **`assay-tui` cannot depend on `assay-harness` directly** (would create a dep-graph cycle). The TUI constructs `HarnessWriter` closures that capture harness adapter functions — the closure is `Box<HarnessWriter>` passed to streaming functions. The `assay-tui` crate must add `assay-harness` as a dependency (it currently does NOT have it). This is safe: harness depends on core, not vice versa; TUI can depend on harness without a cycle.

## Common Pitfalls

- **Borrow conflict on `self.screen` when handling `AgentDone`** — When `TuiEvent::AgentDone` arrives, the handler must: (1) read `exit_code`, (2) call `self.milestones = milestone_scan(...)`, (3) mutate `Screen::AgentRun { status, .. }`. The mutation in step 3 borrows `self.screen` mutably while step 2 borrows `self`. Solution: use `if let Screen::AgentRun { ref mut status, .. } = self.screen` only after the scan completes, matching the clone-then-mutate pattern from D098.
- **`event::read()` blocking in background thread** — The background thread for crossterm events must loop with `event::read()` (blocking). This is fine in a background thread. But: the thread must handle the case where crossterm is no longer initialized (after `ratatui::restore()`). The simplest fix: the main loop sends a quit sentinel when the app exits, and the crossterm thread checks a `Arc<AtomicBool>` stop flag. Alternatively, just let the thread die when the main process exits — Rust drops threads on process exit.
- **`JoinHandle<i32>` from `launch_agent_streaming` is not polled** — The join handle must be polled on `AgentDone` to get the exit code, but the exit code is ALSO sent in the `TuiEvent::AgentDone { exit_code }`. The join handle on `App.agent_thread` can be joined for cleanup (to avoid zombie threads), but the exit code comes from the event, not the join. Use `app.agent_thread.take().map(|h| h.join())` on AgentDone for cleanup.
- **Scroll offset for `Screen::AgentRun`** — The `lines` vec grows unboundedly during a long agent run. The scroll offset must not exceed `lines.len().saturating_sub(visible_height)`. Auto-scroll (scroll to bottom on each new `AgentLine`) is the simplest UX: `scroll_offset = lines.len().saturating_sub(visible_height)`. User override with `Up`/`Down` keys can be a future enhancement.
- **`assay-harness` dependency not currently in `assay-tui/Cargo.toml`** — Must be added before the `r` key handler can call `assay_harness::claude::*` functions. The workspace `Cargo.toml` already has `assay-harness` as a workspace crate; just add `assay-harness.workspace = true` to `[dependencies]` in `crates/assay-tui/Cargo.toml`.
- **`draw()` match arm for `Screen::AgentRun`** — Currently `draw()` has explicit arms for all Screen variants. Adding `Screen::AgentRun` requires a new arm; the compiler will error if it's missing (exhaustive match), which is a safety net.

## Open Risks

- **`cargo test -p assay-tui` count must remain 27 after refactor** — The roadmap requires all 27 existing tests to pass after the event loop refactor. Since tests bypass `run()` entirely (they call `handle_event()` directly), this is low-risk. But adding `Screen::AgentRun` to the `Screen` enum may require updating any exhaustive match arms in test helpers. Verify with `cargo test -p assay-tui` after each step.
- **`AgentStatus::Done` vs `Failed` naming collision with `assay-core::checkpoint::AgentStatus`** — `assay-core` already exports an `AgentStatus` type (in `checkpoint/extractor.rs`). The new `AgentStatus` for the TUI must live in `assay-tui` (either in `app.rs` or a new `agent.rs` module) — not in `assay-core`. Name conflict across crates is not a Rust error (different namespaces), but may confuse readers; consider naming it `AgentRunStatus` or `RunStatus` to disambiguate.
- **Integration test for streaming** — The proof strategy requires a real subprocess (e.g., `echo` loop). On macOS, `echo` behaves slightly differently than GNU `echo`. A more reliable mock: spawn `sh -c 'printf "line1\nline2\nline3\n"; exit 0'`. This works portably across macOS and Linux and produces deterministic output.
- **Gate refresh on `AgentDone`** — After the agent exits, `handle_event` for `AgentDone` calls `milestone_scan()` and `cycle_status()` synchronously. If the `.assay/` directory has many milestones, this could cause a brief TUI stutter. Acceptable for M007 per D091 (sync loading, only at lifecycle transitions).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | none found | none — use ratatui docs directly |
| Crossterm | none found | none — straightforward channel pattern |

## Sources

- Roadmap boundary map for S01 (M007-ROADMAP.md) — authoritative API contract for `TuiEvent`, `launch_agent_streaming`, `Screen::AgentRun`, `AgentStatus`
- `crates/assay-tui/src/main.rs` — current 30-line blocking loop; refactor target
- `crates/assay-tui/src/app.rs` — `App`, `Screen`, all draw functions; D097 field-passing pattern for render functions
- `crates/assay-core/src/pipeline.rs` — `launch_agent()` batch impl; `HarnessWriter` type alias; template for `launch_agent_streaming`
- `crates/assay-harness/src/claude.rs` — `build_cli_args()`, `write_config()` signatures
- Decisions D001, D007, D097, D098, D105, D107, D108 in DECISIONS.md
