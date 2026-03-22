# S01: Channel Event Loop and Agent Run Panel — Research

**Date:** 2026-03-21

## Summary

S01 refactors the TUI event loop from a blocking `event::read()` call to an
`mpsc::channel`-based dispatch loop, and adds `Screen::AgentRun` for streaming
agent output. The slice owns requirements R053 (TUI agent spawning) and
partially R054 (provider abstraction — S02 completes).

The core challenge is threefold:

1. **Event loop refactor**: The current `run()` in `main.rs` blocks on
   `event::read()`. To interleave terminal key events with streaming subprocess
   output, a background thread must feed crossterm events into a channel while
   a second thread feeds agent lines — both to the same `Receiver<TuiEvent>` in
   the main loop.

2. **Streaming vs batch**: `pipeline::launch_agent()` collects stdout after
   completion. A new `launch_agent_streaming(cli_args, working_dir, line_tx)`
   in `assay-core::pipeline` uses `BufReader::lines()` to send each line as it
   arrives, returning a `JoinHandle<i32>` for exit-code delivery. The existing
   batch function is untouched.

3. **App/event wiring**: `App::handle_event` currently returns `bool` (27
   existing tests depend on this signature). The `r` key handler needs access
   to the TuiEvent sender without changing the return type. Solution: store
   `Option<Sender<TuiEvent>>` on `App`; tests leave it `None` (no-op for `r`);
   `run()` sets it before the event loop starts.

The design is fully validated by the existing gossip executor pattern in
`assay-core::orchestrate::gossip` which uses `std::sync::mpsc` and
`std::thread::scope` — exact same pattern, no new deps needed.

## Recommendation

Use `std::sync::mpsc::channel()` (unbounded) for the TuiEvent channel.
Bounded channels (`sync_channel`) risk deadlock: if the channel is full, the
agent output relay thread blocks, which blocks the `BufReader::lines()` read,
which backs up the subprocess pipe, causing the agent to hang. Unbounded
prevents this at the cost of unbounded memory growth — mitigate by capping
`Screen::AgentRun.lines` at a max (e.g., 10_000 entries).

Define `TuiEvent` in `app.rs` (part of the library crate), not in `main.rs`
(binary). The `Sender<TuiEvent>` stored on `App` must reference the same type
that `main.rs` reads from, and `main.rs` imports from the lib crate
(`use assay_tui::app::TuiEvent`).

Add `assay-harness` as a dep to `assay-tui/Cargo.toml` for S01 so the `r` key
handler can call the Claude adapter directly. S02 replaces this with
`provider_harness_writer(config)`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Channel-based thread coordination | `std::sync::mpsc` — already used in `pipeline.rs` and `gossip.rs` | Proven pattern in this codebase; no new deps |
| Line-by-line stdout reading | `std::io::BufReader::lines()` — used in `context/mod.rs` and `context/parser.rs` | Blocks cleanly per line; drop of reader signals EOF |
| Atomic config writes | `NamedTempFile` pattern in `milestone_save`, `config_save` | Don't re-invent for any new persistence in S01 |
| Claude harness config | `assay_harness::claude::{generate_config, write_config, build_cli_args}` | Already fully tested (12 insta snapshots); S01 reuses directly |
| Crossterm event forwarding | Background thread calling `crossterm::event::read()` | Ratatui 0.30 / crossterm 0.28 have no async EventStream in workspace deps — pure thread is correct |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — Current `run()` with blocking `event::read()`. Refactor target: replace with `rx.recv()` over `Receiver<TuiEvent>`.
- `crates/assay-tui/src/app.rs` — `App` struct, `Screen` enum, `handle_event` (returns `bool`). Add `event_tx: Option<mpsc::Sender<TuiEvent>>`, `agent_thread: Option<JoinHandle<i32>>`. Add `Screen::AgentRun` variant.
- `crates/assay-core/src/pipeline.rs` — `launch_agent()` (batch, hardcodes `claude`). New `launch_agent_streaming()` mirrors this structure but reads stdout line-by-line and returns `JoinHandle<i32>`. Leave `launch_agent()` untouched.
- `crates/assay-core/src/orchestrate/gossip.rs` — `mpsc::channel::<GossipCompletion>()`, `std::thread::scope`, `recv_timeout` drain loop. **Exact reference pattern** for the channel-based coordinator; reuse the termination idiom (channel disconnect = workers done).
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir)` returns `Result<Option<CycleStatus>>`. `CycleStatus.active_chunk_slug: Option<String>`. The `r` key handler calls this to get the active chunk slug.
- `crates/assay-harness/src/claude.rs` — `generate_config(profile)`, `write_config(config, dir)`, `build_cli_args(config)`. S01 harness wiring: call these three in sequence, pass resulting args to `launch_agent_streaming`.
- `crates/assay-tui/tests/settings.rs` — Pattern for integration tests: `App::with_project_root(Some(root))`, drive key events via `app.handle_event(key(...))`, assert on `app.screen`. S01 tests follow this exact pattern.

## Constraints

- **`handle_event` returns `bool`** — 27 existing assay-tui tests call this directly. Must not change the return type to avoid breaking all of them.
- **`assay-core` is sync** (D007) — `launch_agent_streaming` must use `std::thread::spawn`, not tokio. No async in `assay-core::pipeline`.
- **Zero-trait convention** (D001) — No `EventHandler` trait, no `AgentBackend` trait. All dispatch is via free functions and closures.
- **`assay-tui` → `assay-harness` dependency** — Currently absent from `Cargo.toml`. Must be added for the `r` key handler to call the Claude adapter. `assay-harness` already depends on `assay-core` and `assay-types`; adding it to `assay-tui` creates no cycle.
- **`AgentStatus` name collision** — `assay-core::checkpoint` already exports an `AgentStatus` type (for checkpoint extraction). The TUI-local enum (`Running`, `Done`, `Failed`) must live in `assay_tui::app` and must NOT be named `AgentStatus` at the crate-public level without qualification, or it will confuse readers. Consider a distinct name like `AgentRunStatus` if collision is a concern.
- **`TuiEvent` visibility** — Define in `app.rs` (`pub enum TuiEvent`) so both `lib.rs` and `main.rs` can use it. If defined only in `main.rs`, `App`'s `Sender<TuiEvent>` field becomes impossible to type in `app.rs`.
- **`ratatui 0.30` / `crossterm 0.28`** — No async event stream is enabled via workspace features. Crossterm's `EventStream` (async) is not available without `event-stream` feature. Pure `std::thread` is the only option.

## Common Pitfalls

- **`r` key no-op when `event_tx` is None** — In tests, `App.event_tx` is `None`. The `r` key handler must check `if let Some(tx) = &self.event_tx` before attempting to spawn. If `None`, either silently do nothing or set a transient error message on screen. Do NOT panic.
- **Crossterm event thread never exits** — The background thread calling `event::read()` will block forever after the main loop exits (because `ratatui::restore()` disables raw mode but the thread may still be blocked in `read()`). Acceptable: the process exits and the OS reclaims the thread. Do NOT try to signal the crossterm thread via a flag — it won't be checked while blocked in `event::read()`. This is the standard Ratatui pattern.
- **`launch_agent_streaming` channel relay deadlock** — Don't use a bounded `sync_channel` between the streaming thread and the relay. Use `mpsc::channel()` (unbounded) for `line_tx`. The relay thread forwards `String` → `TuiEvent::AgentLine`. The TuiEvent channel itself should also be unbounded.
- **Exit code lost if relay uses separate `JoinHandle`** — The clean pattern is: one outer wrapper thread that (1) drains the `line_rx` channel (blocking until `str_tx` is dropped when the streaming thread finishes), then (2) joins the streaming `JoinHandle<i32>`, then (3) sends `TuiEvent::AgentDone`. This avoids a separate "done-notifier" thread and guarantees no line is lost before `AgentDone` is sent.
- **draw() called on `Screen::AgentRun` before it has lines** — The first render happens before any `AgentLine` event arrives. `draw_agent_run` must handle `lines.is_empty()` gracefully (show "Starting…" or empty list, not panic).
- **Gate refresh on `AgentDone`** — The handler for `TuiEvent::AgentDone` must call `milestone_scan`, update `self.milestones`, and refresh `self.cycle_slug`. It must also refresh `self.detail_run` if `self.screen` is `Screen::AgentRun` (so the caller can navigate to ChunkDetail and see updated results). Use the existing `cycle_status(&assay_dir).ok().flatten()` pattern already in `settings.rs` and `app.rs`.
- **`assay-core::pipeline::launch_agent_streaming` naming** — The function signature in `assay-core` takes `Sender<String>` (not `Sender<TuiEvent>`) to keep `assay-core` free of TUI-specific types. The wrapping from `String → TuiEvent::AgentLine` happens in `app.rs` via the relay-wrapper thread pattern.

## Open Risks

- **Agent produces no newlines** — If the agent writes large JSON blobs without `\n`, `BufReader::lines()` will block until `\n` or EOF. This means the TUI won't update during long stretches of output. Mitigation: document this limitation; it's inherent to line-by-line streaming. Fix in a future slice if needed.
- **`ratatui::restore()` on agent-running drop** — If the user presses `q` while `Screen::AgentRun` is active, the agent subprocess continues running in the background (streaming thread still alive). The `App.agent_thread` JoinHandle will be dropped without joining, orphaning the subprocess. For S01: acceptable (document as known limitation). Future: add a kill mechanism via `app.agent_pid`.
- **27 existing tests must all pass after refactor** — The `run()` loop in `main.rs` is refactored but `App::with_project_root` and `App::handle_event` remain unchanged. Tests bypass `run()` entirely. Risk is LOW — the refactor only touches `main.rs::run()` and adds fields to `App`. Verify with `cargo test -p assay-tui` after the refactor.
- **Harness write_config needs a writable worktree** — The `r` key handler calls `write_config()` which creates `.claude/` directory. Without a real git worktree, this will fail. For S01, the integration test uses an echo subprocess (not real Claude), so `write_config` won't be called. UAT only for real invocation.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui (TUI) | checked `npx skills find "ratatui"` | none found |
| crossterm | checked `npx skills find "crossterm"` | none found |

No relevant skills found. The Ratatui documentation and the existing gossip executor pattern in the codebase are sufficient reference.

## Sources

- Gossip executor mpsc pattern (source: `crates/assay-core/src/orchestrate/gossip.rs:213`) — the `mpsc::channel::<T>()`, relay-thread, `recv_timeout` drain idiom is battle-tested in this codebase; reuse verbatim for TuiEvent
- BufReader::lines() usage (source: `crates/assay-core/src/context/mod.rs:93`, `context/parser.rs:44`) — established pattern for line-by-line reading of file/pipe output
- Existing 27-test count (source: `cargo test -p assay-tui` output showing 1+6+5+6+9 = 27 tests across 5 test files) — milestone constraint: all must still pass post-refactor
- `launch_agent` batch pattern (source: `pipeline.rs:227`) — reference for `launch_agent_streaming` structure; mirrors spawn + timeout thread, remove batch collection, replace with BufReader relay
- `cycle_status` + `CycleStatus.active_chunk_slug` (source: `milestone/cycle.rs:39,72`) — the `r` key handler entry point for finding the active chunk
- `handle_event` returning `bool` pattern and test convention (source: `tests/settings.rs`, `tests/spec_browser.rs`) — must not change signature; `event_tx = None` guard is required for test compat
