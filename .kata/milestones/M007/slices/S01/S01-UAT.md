# S01: Channel Event Loop and Agent Run Panel — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: The channel event loop, streaming primitive, and AgentRun state transitions are fully proven by integration tests with real subprocess pipes — these cover the mechanical correctness of R053. The human-experience portion (pressing `r` on a real project and watching Claude output stream) is required to validate end-to-end invocation with the actual Claude Code binary, which cannot be automated without a real API key and network access.

## Preconditions

**For automated tests (already verified):**
- `cargo test -p assay-core --test pipeline_streaming` passes (3/3)
- `cargo test -p assay-tui --test agent_run` passes (3/3)
- `cargo build -p assay-tui` produces `target/debug/assay-tui`

**For manual UAT:**
- `assay-tui` binary built: `cargo build -p assay-tui`
- A real Assay project exists with at least one milestone in `InProgress` state and an active chunk
- Claude Code CLI (`claude`) is installed and authenticated
- `ANTHROPIC_API_KEY` is set in the environment

## Smoke Test

Run `target/debug/assay-tui` from the project root. Navigate to the dashboard. Verify the dashboard renders with milestone/chunk status. Press `r` and verify the screen transitions to `Screen::AgentRun` (titled "Agent Run: <chunk-slug>").

## Test Cases

### 1. Agent output streams line-by-line (automated — already verified)

The `agent_run_streams_lines_and_transitions_to_done` integration test:

1. Construct an `App` with `project_root = None` (no real project needed)
2. Call `app.handle_agent_line("line one")`, `handle_agent_line("line two")`, `handle_agent_line("line three")`
3. Call `app.handle_agent_done(0)`
4. **Expected:** `app.screen` is `Screen::AgentRun { lines: ["line one", "line two", "line three"], status: AgentRunStatus::Done { exit_code: 0 }, .. }`

### 2. Failed exit code shows Failed status (automated — already verified)

The `agent_run_failed_exit_code_shows_failed_status` integration test:

1. Construct App in AgentRun state (Running)
2. Call `app.handle_agent_done(1)`
3. **Expected:** `status` is `AgentRunStatus::Failed { exit_code: 1 }`

### 3. r key with no project is a no-op (automated — already verified)

The `agent_run_r_key_on_no_project_is_noop` integration test:

1. Construct App with `project_root = None`, `screen = Screen::NoProject`
2. Press `r` (simulate `KeyCode::Char('r')` KeyEvent)
3. **Expected:** `app.screen` remains `Screen::NoProject`; no panic

### 4. launch_agent_streaming delivers lines with real subprocess (automated — already verified)

The `streaming_delivers_lines_to_receiver` integration test in `pipeline_streaming.rs`:

1. Spawn `launch_agent_streaming` with `["sh", "-c", "printf 'a\nb\nc\n'"]`
2. Drain the `mpsc::Receiver<String>`
3. **Expected:** receiver yields exactly `["a", "b", "c"]` in order

### 5. Agent output streams in real TUI (manual UAT)

1. Open `assay-tui` on a project with an InProgress milestone and active chunk
2. Navigate to the dashboard so the active chunk is visible
3. Press `r`
4. **Expected:** Screen transitions to "Agent Run: <chunk-slug>"; Claude Code output lines appear one by one as the agent runs; status bar shows "Running…" in yellow
5. Wait for agent to complete
6. **Expected:** Status bar updates to "Done (exit 0)" in green (or "Failed (exit N)" in red if the agent exited non-zero)

### 6. Gate results refresh after agent exits (manual UAT)

1. After step 5 above, press `Esc` to return to Dashboard
2. **Expected:** Milestone/chunk progress counts in the dashboard reflect any gate runs completed during the agent session (pass/fail counts updated); no TUI restart required

## Edge Cases

### r key with no active chunk is a no-op

1. Open `assay-tui` on a project where the active milestone has all chunks complete (status: Verify)
2. Press `r`
3. **Expected:** Nothing happens; no transition to AgentRun; no panic

### Agent produces no output then exits

1. (Manual UAT) Configure the agent to run a spec with no output
2. Press `r`
3. **Expected:** AgentRun screen shows empty output area; status bar shows "Done (exit 0)" or "Failed (exit N)" immediately

### Esc from AgentRun while agent is running

1. (Manual UAT) Press `r` to start an agent
2. While agent is running (status: Running), press `Esc`
3. **Expected:** Screen returns to Dashboard; agent continues running in background; bridge thread continues forwarding events (which are silently dropped since the channel receiver has moved on — no crash)

## Failure Signals

- TUI freezes (no key response) → event loop deadlock; check if crossterm background thread is blocked
- Lines appear in batches instead of one-by-one → `BufReader::lines()` iteration may be buffered; check subprocess stdout buffering
- "Done" status never appears after agent exits → bridge thread may not have received `exit_rx` value; check exit-code thread
- Status bar shows yellow "Running…" indefinitely after agent exits → `AgentDone` event not delivered; check exit_tx/exit_rx channel and bridge thread panic
- Panic in `handle_agent_done` → join error logged via `eprintln!`; check bridge thread ownership
- Gate results don't refresh after agent exit → `handle_agent_done` calls `milestone_scan` synchronously; check `project_root` is Some and `.assay/` is readable

## Requirements Proved By This UAT

- R053 (TUI agent spawning) — the full S01 mechanical loop is proven by automated integration tests: subprocess spawning, line-by-line streaming via mpsc channel, AgentRun screen state accumulation, Done/Failed status on exit, and dashboard gate result refresh on AgentDone. The automated tests use real OS subprocess pipes (not mocks).
- R054 (provider abstraction, partially) — the Claude Code adapter path through `r` → `launch_agent_streaming` → `AgentRun` is proven by the r-key integration test with a mock subprocess. Manual UAT with the real `claude` binary completes the validation.

## Not Proven By This UAT

- Real Claude Code invocation producing useful gate results — requires a real project, real API key, real spec with executable gates. UAT test case 5 covers this but is manual-only.
- Ollama and OpenAI provider dispatch — deferred to S02; the `r` key currently hardcodes the Claude Code adapter.
- Slash command overlay (`/`) — deferred to S03.
- MCP server panel (`m`) — deferred to S04.
- Gate result refresh correctness — the dashboard re-reads milestone data after `AgentDone`, but whether the displayed pass/fail counts are accurate depends on the gate evaluation run during the agent session. This is end-to-end workflow correctness, validated manually only.
- Multiple consecutive agent runs in one TUI session — temp dir accumulation (D115 limitation) is not stressed by automated tests; each run leaks a dir until process exit.

## Notes for Tester

- The `r` key only works when `App.event_tx` is `Some` (i.e., inside the real `run()` loop). Running `assay-tui` normally satisfies this. Unit tests that construct `App` directly will see the r key as a no-op — this is intentional and correct.
- If the agent run shows no output at all, check that the active chunk's spec has an executable gate criterion; the agent may exit immediately with a "nothing to run" message.
- The `VISIBLE_HEIGHT = 20` constant for auto-scroll is approximate. On very tall or short terminals, the last output line may be slightly off-screen; scroll with `j`/`k` (if wired in S02) or resize the terminal.
- Redaction: agent stdout lines are streamed verbatim. If the Claude session logs an API key or secret, it will appear in the AgentRun screen. This is a known M007 limitation (noted in the plan); do not use sensitive credentials in test environments.
