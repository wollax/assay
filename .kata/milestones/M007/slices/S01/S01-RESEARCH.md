# S01: Channel Event Loop and Agent Run Panel — Research

**Date:** 2026-03-23

## Summary

S01 refactors the TUI's blocking `event::read()` loop into a channel-based `mpsc::Receiver<TuiEvent>` loop, adds `launch_agent_streaming` to `assay-core::pipeline`, and introduces `Screen::AgentRun` with `draw_agent_run`. The current `run()` in `main.rs` is 15 lines — the entire event loop is one `match event::read()?` block. Ratatui 0.30 + crossterm 0.28 are already in the workspace. There are 27 integration tests in `crates/assay-tui/tests/` (across 5 test files) that must all continue to pass after the refactor.

The key architectural insight: the channel-based event loop is **not** a new architecture — it's the standard Ratatui pattern for background events. The `App` struct and all screen logic in `app.rs` are completely untouched by the event loop refactor. Only `main.rs:run()` changes. The `App::handle_event(key)` method stays `fn handle_event(&mut self, key: KeyEvent) -> bool` — the same signature as today. This isolation means none of the 27 existing integration tests need to change: they construct `App` directly and call `handle_event` with synthetic events, bypassing the event loop entirely.

`launch_agent_streaming` in `assay-core::pipeline` is a new free function alongside the existing batch `launch_agent()`. It spawns a `std::thread` that reads child stdout line-by-line via `BufReader::lines()` and sends each line to a `mpsc::Sender<String>`. The join handle's return value (`i32`) carries the exit code. This is the same pattern `launch_agent()` already uses for thread-based timeout — just with per-line delivery instead of batch read.

## Recommendation

Implement in two parallel tracks:

1. **`launch_agent_streaming` in `assay-core::pipeline`** — new free function, existing batch function untouched. Test with a real subprocess (`echo`/`sh -c 'echo line1; echo line2; exit 0'`).

2. **Channel event loop in `main.rs`** — replace the 15-line `run()` with the mpsc pattern. Wire `Screen::AgentRun`, `AgentStatus`, and `draw_agent_run` into `app.rs`. Test via integration tests that drive synthetic `TuiEvent` values through `App`.

Order tasks so T01 delivers `launch_agent_streaming` + tests, T02 delivers `TuiEvent` + refactored `run()` + `Screen::AgentRun` wired to `r` key, T03 delivers `draw_agent_run` + post-run gate refresh + integration tests.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Line-by-line subprocess stdout | `std::io::BufReader::lines()` on `child.stdout.take()` | Already used in `evaluator.rs` (line 482-484 shows `Stdio::piped()`); same `std::process::Command` spawn pattern as `launch_agent()` |
| Thread-based async from background thread | `std::sync::mpsc::channel()` | Already in `pipeline.rs` for timeout; just change from batch read to per-line send |
| Crossterm event polling without blocking | `crossterm::event::poll(Duration::ZERO)` + `event::read()` in a background thread | This is the canonical pattern for channel-based Ratatui event loops — push events into the channel from a dedicated thread |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — Current 15-line blocking `run()`. Replace with channel loop. `App::handle_event(key: KeyEvent) -> bool` signature unchanged — tests bypass `run()` entirely.
- `crates/assay-tui/src/app.rs` — `App` struct, `Screen` enum, all renderers. Add `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }`, `AgentStatus` enum, `App.agent_thread: Option<JoinHandle<i32>>`. Add `r` key handler in Dashboard arm.
- `crates/assay-core/src/pipeline.rs:launch_agent()` — Batch launcher. Pattern to follow for `launch_agent_streaming`: same `Command::new` + `Stdio::piped()` spawn, but thread sends lines instead of buffering all stdout. `HarnessWriter` type alias already here.
- `crates/assay-tui/tests/settings.rs`, `spec_browser.rs`, etc. — Integration test pattern: construct `App::with_project_root(Some(root))`, drive `app.handle_event(key(KeyCode::...))`, assert on `app.screen`. S01 tests follow this exact pattern.
- `crates/assay-harness/src/claude.rs:build_cli_args()` — Returns `Vec<String>` with `--print --output-format json`. This is the `cli_args` passed to `launch_agent_streaming`.

## Constraints

- **Zero tokio in assay-tui** — `assay-tui/Cargo.toml` has no async deps. All threading must use `std::thread`. The `evaluator.rs` uses tokio but that's in `assay-core` and not called from TUI directly.
- **D001: no traits** — `TuiEvent`, `AgentStatus`, `Screen::AgentRun` are plain enums/structs. No `Widget` impls on app state.
- **D007: sync core** — `launch_agent_streaming` uses `std::thread::spawn`, `std::io::BufReader`, `std::sync::mpsc::Sender`.
- **All 27 existing TUI tests must pass** — The integration tests call `App::handle_event` directly; they do not go through `run()`. Refactoring `run()` does not break them. Adding fields to `App` requires updating `App::with_project_root` to initialize them.
- **`App.agent_thread` field** — Adding this field to `App` will require initializing it to `None` in `with_project_root`. No existing test constructs `App` with agent state — safe to add.
- **Bounded vs unbounded channel** — Use unbounded `mpsc::channel()` for the TUI event channel. A bounded channel that fills blocks the crossterm event thread, which causes the TUI to freeze (D107). The existing timeout pattern in `pipeline.rs` uses unbounded channels.
- **`r` key requires InProgress chunk** — `cycle_status(&assay_dir)` returns `Option<CycleStatus>`. If `active_chunk_slug` is `None`, the `r` key is a no-op (no agent to spawn). The `App` already loads and caches `cycle_slug`; S01 needs `active_chunk_slug` too (or reloads `CycleStatus` on demand).

## Common Pitfalls

- **Holding `self.screen` borrow while mutating App** — The `r` key handler in the Dashboard arm needs to: (1) get the active chunk slug from `cycle_status`, (2) construct `HarnessProfile`, (3) spawn `launch_agent_streaming`, (4) set `self.screen = Screen::AgentRun`. The `HarnessWriter` closure (from `provider_harness_writer` in S02) needs to be available at `r`-key time. For S01, hardcode Anthropic (claude adapter) — S02 wires the provider dispatch. Avoid holding borrows across the `self.screen` assignment.
- **`JoinHandle<i32>` is not `Clone`** — `App.agent_thread: Option<JoinHandle<i32>>` cannot be cloned. On `AgentDone` event, call `self.agent_thread.take().map(|h| h.join())` to consume the handle. Never store the exit code twice.
- **`AgentDone` event polling** — The agent thread sends `TuiEvent::AgentLine(String)` for each line and `TuiEvent::AgentDone { exit_code }` when the child exits. The channel loop receives these naturally. No polling needed — the channel delivers them when they arrive.
- **draw_agent_run scroll** — Use `List` with `ListState` for scrollable output. Do NOT use `ListState` on `App` for agent lines (it would conflict with dashboard `list_state`). Store scroll as `usize` inside `Screen::AgentRun`. Use `List::new(items).render_stateful_widget` with a local `ListState`.
- **Integration test for channel loop** — The proof strategy says "no separate real-terminal test." Tests drive `App` directly with synthetic events. For `launch_agent_streaming`, the integration test uses a real subprocess (`echo line1; echo line2; exit 0`) and asserts all lines are received via the `Sender` before the thread joins.

## Open Risks

- **`r` key HarnessProfile construction without worktree** — S01's `r` key handler needs to call `HarnessWriter` to get CLI args, but the worktree setup (stages 1-2 of the pipeline) is slow. For S01, the agent is spawned directly in the working dir (project root or active chunk worktree if it exists). The full pipeline integration (setup_session + execute_session from the TUI) is a follow-on concern; S01 proves streaming delivery with a simplified spawn.
- **Exit code delivery via `JoinHandle`** — `JoinHandle<i32>::join()` returns `Result<i32, Box<dyn Any>>`. The thread panicking would yield `Err`. Handle gracefully: on `Err`, treat as exit code -1 (Failed).
- **Screen borrow in `draw()` for AgentRun** — The `draw()` match arm for `Screen::AgentRun { lines, scroll_offset, status, chunk_slug }` needs to pass fields individually (D097). `lines` is a `Vec<String>` inside the enum variant — the borrow checker will require `..` or cloning for the list state. Best pattern: store a separate `agent_list_state: ListState` on `App` (like `detail_list_state`), not inside the Screen variant.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | none found | none found |
| crossterm | none found | none found |

## Sources

- Blocking event loop pattern: `crates/assay-tui/src/main.rs` (direct read)
- Streaming subprocess pattern: `crates/assay-core/src/pipeline.rs:launch_agent()` (thread + mpsc)
- Subprocess piping: `crates/assay-core/src/evaluator.rs` line 482-484 (`Stdio::piped()`)
- Test pattern: `crates/assay-tui/tests/settings.rs`, `spec_browser.rs` (App::with_project_root + handle_event)
- Harness CLI args: `crates/assay-harness/src/claude.rs:build_cli_args()` (Vec<String> with --print)
- Provider enum: `crates/assay-types/src/lib.rs:ProviderKind` (Anthropic, OpenAi, Ollama)
- App/Screen structure: `crates/assay-tui/src/app.rs` (Screen enum, App fields, draw/handle_event)
