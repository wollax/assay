# S01: Channel Event Loop and Agent Run Panel — UAT

**Milestone:** M007
**Written:** 2026-03-21

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: The channel event loop, `launch_agent_streaming`, and App state machine are fully proven by automated integration tests using a real echo subprocess. The human experience test covers the one gap automation cannot reach: a real Claude Code agent streaming output to the TUI from a real project's active chunk.

## Preconditions

1. `cargo build -p assay-tui` succeeds and `target/debug/assay-tui` exists.
2. A project with `.assay/milestones/` exists and has at least one InProgress milestone with an active chunk (cycle_status returns Some).
3. `claude` CLI is installed and authenticated (Anthropic API key set).
4. Provider in `.assay/config.toml` is set to Anthropic (or default — Anthropic is the only S01 provider).
5. Run from the project root.

## Smoke Test

Run `cargo test -p assay-tui --test agent_run` — all 8 tests must pass before proceeding to human UAT.

## Test Cases

### 1. TUI launches and shows dashboard

1. Run `target/debug/assay-tui` from a project root with at least one milestone.
2. **Expected:** Dashboard renders with milestone list, chunk progress, and status bar showing project name.

### 2. `r` key opens AgentRun screen and streams output

1. From the Dashboard, navigate to an InProgress chunk and press `r`.
2. **Expected:** Screen transitions immediately to the Agent Run panel titled "Agent Run: {chunk-slug}". "Starting…" placeholder appears before first output.
3. Watch as Claude output lines appear one-by-one in the scrollable area.
4. **Expected:** Lines accumulate in real time; screen remains responsive (can scroll with j/k).
5. When Claude exits, the status bar shows "✓ Done (exit 0)" in green (or "✗ Failed (exit N)" in red for non-zero exit).

### 3. Scrolling through agent output

1. While in the Agent Run panel with multiple lines visible, press `j` or `↓`.
2. **Expected:** Output list scrolls down one line.
3. Press `k` or `↑`.
4. **Expected:** Output list scrolls back up one line.

### 4. Esc returns to Dashboard

1. While in the Agent Run panel (Running or Done state), press `Esc`.
2. **Expected:** Screen transitions back to Dashboard. Milestone list refreshes (gate counts may have updated if agent completed gate evaluation).

### 5. `r` key with no active chunk is a no-op

1. Navigate to a Dashboard with no InProgress milestone (all milestones in Draft or Complete).
2. Press `r`.
3. **Expected:** Screen does not change; no crash; no error message.

## Edge Cases

### Agent outputs many lines (cap-at-10k)

1. Use a harness config that causes the agent to produce > 10 000 lines of output (e.g., a spec that loops).
2. **Expected:** Buffer stays at exactly 10 000 lines; TUI does not OOM or slow significantly.

### Agent exits non-zero

1. Configure the spec such that the agent will exit with a non-zero code (e.g., point to a non-existent command).
2. **Expected:** Status bar shows "✗ Failed (exit N)" in red. `Esc` returns to Dashboard normally.

### TUI responsiveness during streaming

1. While agent is streaming output, press `j` multiple times.
2. **Expected:** Scroll responds within one frame; TUI does not freeze or drop key events.

## Failure Signals

- Screen stays on Dashboard after `r` press → cycle_status returned None (no active chunk) or write_config failed; check `app.event_tx.is_some()` is true at runtime.
- Status bar shows "● Running…" indefinitely after agent should have finished → relay-wrapper thread may be blocked; check process exited and `str_rx` was drained.
- TUI freezes on key press during streaming → channel send is blocking (should not happen with unbounded channel); check for panic in background thread.
- "Starting…" placeholder never replaced by output → `launch_agent_streaming` failed to spawn subprocess or harness config is invalid; check `temp_dir/assay-agent-{slug}/` exists with valid `.claude/` config.

## Requirements Proved By This UAT

- R053 (TUI agent spawning) — human observer confirms `r` key spawns Claude Code, output streams live to Screen::AgentRun, Done/Failed status shown on exit, Dashboard gate counts refresh after completion.
- R054 (Provider abstraction, Anthropic path) — human confirms that with Anthropic configured, `r` invokes `claude --print` (not ollama or another binary).

## Not Proven By This UAT

- Ollama and OpenAI provider dispatch — proven in S02 UAT.
- Gate results auto-refreshing in the dashboard after agent exits — requires a real spec with runnable gates and a Claude run that produces gate-evaluatable output; this is a complex setup that belongs in S02's UAT when provider dispatch is stable.
- Performance with long-running agents (hours of output) — not tested; the 10 000-line cap is the only guard.
- Concurrent `r` presses — not tested; S02 should add a guard against double-spawning.
- The automated tests prove channel mechanics with a real echo subprocess, but they do not exercise the Claude CLI authentication path.

## Notes for Tester

- The `r` key handler writes harness config to `$TMPDIR/assay-agent-{chunk-slug}/`. If this directory already exists from a previous run, it will be overwritten — this is expected S01 behavior and will be fixed in S02.
- Claude Code will use `--print` mode (non-interactive), so there is no TTY interaction needed. The agent reads the spec from the harness config and runs to completion.
- The TUI renders in the terminal you launch it from. Use a terminal with at least 80×24 to see the full Agent Run panel with status bar.
- If `claude` is not on PATH, the `r` key silently no-ops (write_config may succeed but spawn will fail). This is a known S01 limitation — S02 adds proper error display.
