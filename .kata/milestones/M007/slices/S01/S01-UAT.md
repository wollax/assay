# S01: Channel Event Loop and Agent Run Panel — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven for streaming/event-loop; live-runtime for real `r` key press)
- Why this mode is sufficient: The channel event loop and AgentRun panel are fully proven by integration tests driving synthetic `TuiEvent` events through real `App` state — no terminal required. `launch_agent_streaming` is proven by a real subprocess (echo-based). Real Claude Code invocation is UAT-only and requires manual execution with the installed `claude` CLI.

## Preconditions

**For automated integration tests (artifact-driven):**
- `cargo test -p assay-tui --test agent_run` — runs without any external tools
- `cargo test -p assay-core -- launch_agent_streaming` — requires `sh` (always available on macOS/Linux)

**For live-runtime UAT (r key press):**
- `claude` CLI installed and authenticated (`claude --version` returns without error)
- A project with an InProgress milestone and an active chunk (`assay milestone list` shows a milestone in InProgress state with `active_chunk_slug` set)
- `assay-tui` binary built: `cargo build -p assay-tui`
- Run from the project root: `./target/debug/assay-tui`

## Smoke Test

```
cargo test -p assay-tui --test agent_run
# Expected: 4 tests, 0 failed
```

## Test Cases

### 1. Agent line accumulation (automated)

```rust
// From tests/agent_run.rs
let mut app = App::with_project_root(tmpdir.path().to_path_buf());
app.screen = Screen::AgentRun { chunk_slug: "test-chunk".into(), lines: vec![], scroll_offset: 0, status: AgentStatus::Running };
app.handle_tui_event(TuiEvent::AgentLine("hello".into()));
app.handle_tui_event(TuiEvent::AgentLine("world".into()));
// assert lines == ["hello", "world"]
```

**Expected:** `Screen::AgentRun.lines` contains both lines in insertion order.

### 2. AgentDone transitions to Done status (automated)

```rust
app.handle_tui_event(TuiEvent::AgentDone { exit_code: 0 });
// assert matches!(app.screen, Screen::AgentRun { status: AgentStatus::Done { exit_code: 0 }, .. })
```

**Expected:** Status transitions to `Done { exit_code: 0 }`.

### 3. Non-zero exit transitions to Failed (automated)

```rust
app.handle_tui_event(TuiEvent::AgentDone { exit_code: 1 });
// assert matches!(app.screen, Screen::AgentRun { status: AgentStatus::Failed { exit_code: 1 }, .. })
```

**Expected:** Status transitions to `Failed { exit_code: 1 }`.

### 4. `r` key no-op when no InProgress milestone (automated)

```rust
// App with no milestone files → cycle_status returns None
app.handle_event(KeyCode::Char('r').into());
// assert screen is still Dashboard
```

**Expected:** Screen remains `Screen::Dashboard`; no panic.

### 5. launch_agent_streaming delivers all lines (automated, real subprocess)

```rust
let (tx, rx) = std::sync::mpsc::channel();
let handle = launch_agent_streaming(&["sh", "-c", "printf 'line1\nline2\n'"], &cwd, tx);
let line1 = rx.recv_timeout(Duration::from_secs(2)).unwrap();
let line2 = rx.recv_timeout(Duration::from_secs(2)).unwrap();
assert_eq!(line1, "line1");
assert_eq!(line2, "line2");
assert_eq!(handle.join().unwrap(), 0);
```

**Expected:** Both lines received in order, exit code 0.

### 6. `r` key streams live output (live-runtime UAT)

1. Open `assay-tui` in a project with an InProgress milestone.
2. Confirm dashboard shows the InProgress milestone with an active chunk.
3. Press `r`.
4. **Expected:** Screen transitions to `AgentRun` panel showing "Running…" in the status line.
5. Agent stdout lines appear line-by-line as they are emitted.
6. When agent exits, status line changes to "Done (exit 0)" or "Failed (exit N)".
7. Press `Esc`.
8. **Expected:** Returns to Dashboard.

## Edge Cases

### No active chunk (dashboard with no InProgress milestone)

1. Open `assay-tui` in a project with all milestones Complete or no milestones at all.
2. Press `r`.
3. **Expected:** No transition — screen stays as Dashboard, no error.

### Agent exits non-zero

1. Configure `r` to run an agent binary that exits with a non-zero code.
2. Press `r`.
3. **Expected:** Status line shows "Failed (exit N)" where N is the exit code. Esc returns to Dashboard.

### Empty stdout agent

1. Agent emits no stdout lines before exiting.
2. Press `r`.
3. **Expected:** `Screen::AgentRun` panel shows empty list and "Done (exit 0)" (or "Failed") — no panic.

## Failure Signals

- `cargo test -p assay-tui --test agent_run` reporting any FAILED test
- TUI freezing when `r` is pressed (indicates blocking loop not replaced)
- Status line staying "Running…" after agent exits (indicates AgentDone event not received)
- Pressing `Esc` not returning to Dashboard from AgentRun
- `cargo build -p assay-tui` producing warnings or errors

## Requirements Proved By This UAT

- R053 (TUI agent spawning) — Integration tests prove the channel event loop, streaming accumulation, and Done/Failed status transitions. Live-runtime test proves real Claude Code output streams into the panel.
- R054 (Provider abstraction, foundation) — `launch_agent_streaming` is provider-agnostic; the channel loop is the shared infrastructure that S02's provider dispatch will use. S01 proves the plumbing; S02 proves the dispatch.

## Not Proven By This UAT

- Accurate exit codes from the agent — S01 forwarder always sends `exit_code: 0` as sentinel; real exit codes require S02 wiring.
- Provider routing (Anthropic vs Ollama vs OpenAI) — S02.
- Gate result refresh after agent exits — `AgentDone` calls `milestone_scan` and `cycle_status` from disk, but gate history refresh (`detail_run` update) is only partially wired and depends on S02 harness running real gate evaluation.
- Slash command overlay — S03.
- MCP server panel — S04.
- Real Claude Code invocation in CI — manual UAT only; CI has no `claude` CLI.

## Notes for Tester

- The live-runtime UAT (Test Case 6) requires `claude` CLI authenticated. If Claude isn't available, the automated tests (Cases 1–5) are the primary proof surface.
- S01 hardcodes `["claude", "--print"]` as CLI args. If Claude is not installed, the agent spawn in the TUI will fail immediately (exit code -1 → `AgentStatus::Failed`). This is expected behavior for S01 — not a bug.
- The `r` key can be pressed while an agent is already running (no guard exists). This is a known rough edge — S02 should add a guard.
- Status bar shows "Running…" while the agent is active. If the TUI appears frozen during agent execution, it indicates the channel loop refactor may have regressed — check that `event::read()` is no longer in the main loop (`grep "event::read()" crates/assay-tui/src/main.rs` should only find it inside the background thread).
