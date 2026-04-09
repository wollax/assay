# Close the Loop — Mid-Session Checkpoints + Auto-Promote

This example demonstrates Assay's M024 feature set: **mid-session checkpoint
evaluation** with **early abort** on failure, and **automatic spec promotion**
on a clean run. It exercises the full gate/spec loop against a real `claude`
CLI subprocess.

## Overview

Two execution paths are provided:

| Path | Script | Outcome |
|------|--------|---------|
| **Abort** | `run-abort.sh` | Agent exceeds tool budget → checkpoint fails → early kill |
| **Promote** | `run-promote.sh` | Agent stays within budget → session-end passes → auto-promote to `verified` |

### How It Works

The spec defines two gate criteria:

1. **`tool-budget`** (checkpoint) — An `EventCount` criterion with `max = 1`
   and `when = { type = "after_tool_calls", n = 2 }`. If the agent makes 2+
   tool calls, this checkpoint fires and counts `tool_called` events. Since
   count (2) exceeds `max` (1), the criterion fails and the pipeline kills the
   agent subprocess.

2. **`no-tool-errors`** (session-end) — A `NoToolErrors` criterion evaluated
   at session end. Passes trivially when the agent completes without tool
   errors.

The feature spec (`spec.toml`) sets `auto_promote = true` and
`status = "in-progress"`, enabling the pipeline to automatically advance to
`verified` when all gates pass.

## Prerequisites

- Assay CLI installed: `cargo install --path crates/assay-cli`
- `claude` CLI installed and authenticated
  (`claude --version` should print a version string)

## Quick Start

```bash
# 1. Install spec into .assay/specs/
./examples/close-the-loop/setup.sh

# 2. Run the promote path (auto-promotion demo)
./examples/close-the-loop/run-promote.sh

# 3. Inspect the result
assay spec review close-the-loop
# → Shows "Auto-promotion: in-progress → verified"
```

### About the Abort Path

The abort path (`run-abort.sh`) exercises the checkpoint failure scenario.
In `--print` mode, the `claude` CLI runs single-pass inference without tool
calls, so the `after_tool_calls` checkpoint (n = 2) does not fire.

To trigger the checkpoint abort, run the agent in **interactive/agentic mode**
(e.g., via MCP or the TUI), where tool calls produce `tool_called` streaming
events. The checkpoint driver monitors these events and fires `evaluate_checkpoint`
when the tool call count reaches the threshold.

Or use the justfile recipe to run the promote path:

```bash
just demo-close-the-loop
```

## Expected Output

### Abort Path

After running `run-abort.sh`, `assay spec review close-the-loop` shows:

```
Review: close-the-loop
  ...

Gate Diagnostics (most recent run):
  Run: <run-id> (YYYY-MM-DD HH:MM:SS UTC)
  Passed: 0  Failed: 1
  ✗ tool-budget
    stderr: EventCount: count 2 exceeds max 1

Checkpoints:
  [0] phase: at_tool_call(2)  passed: 0  failed: 1
      ✗ tool-budget
```

The spec remains at `in-progress` status.

### Promote Path

After resetting and running `run-promote.sh`:

```
Review: close-the-loop
  ...

Gate Diagnostics (most recent run):
  Run: <run-id> (YYYY-MM-DD HH:MM:SS UTC)
  Passed: 1  Failed: 0

Auto-promotion: in-progress → verified
```

## `when` Schema Reference

The `when` field on a criterion controls when it is evaluated:

| Variant | TOML | Behavior |
|---------|------|----------|
| `SessionEnd` | *(default, omit `when`)* | Evaluated after the agent session completes |
| `AfterToolCalls` | `[criteria.when]`<br>`type = "after_tool_calls"`<br>`n = 5` | Evaluated when the agent has emitted exactly `n` tool calls |
| `OnEvent` | `[criteria.when]`<br>`type = "on_event"`<br>`event_type = "tool_called"` | Evaluated when the most recent event matches `event_type` |

## Checkpoint Cost Guidance

**Event criteria are cheap.** `EventCount` and `NoToolErrors` evaluate
in-memory against the buffered event stream. They add negligible overhead and
are safe to use at any checkpoint frequency.

**Command/file gates are dangerous at checkpoints.** A `cmd`-based criterion
with `when = { type = "after_tool_calls", ... }` runs a shell command against a **partial working
directory** — the agent is still writing files. This can produce flaky results
(partial writes, missing files) and adds real wall-clock latency to every
checkpoint. **Prefer event criteria for mid-session checkpoints.**

Rule of thumb:
- **Checkpoint criteria:** `EventCount`, `NoToolErrors`
- **Session-end criteria:** `cmd`, `path`, `AgentReport`

## Prompt Selection

The two prompt files (`prompt-abort.md` and `prompt-clean.md`) are **reference
documents** describing what kind of agent behavior triggers each path. The
pipeline's agent invocation is controlled by the harness profile — the operator
configures the agent's system prompt or task description through harness
settings, not through the manifest. These files document the intent so the
operator can craft an appropriate prompt for each run.

## Resetting State

To re-run the scenario from scratch:

```bash
./examples/close-the-loop/reset.sh
```

This restores `spec.toml` status to `in-progress` and clears
`.assay/sessions/` and `.assay/reviews/close-the-loop/`.

Alternatively, reset manually:

```bash
rm -rf .assay/sessions/ .assay/reviews/close-the-loop/
cp examples/close-the-loop/spec.toml .assay/specs/close-the-loop/spec.toml
```

## File Inventory

| File | Purpose |
|------|---------|
| `spec.toml` | Feature spec with `auto_promote = true`, `status = "in-progress"` |
| `gates.toml` | Gate criteria: checkpoint (`tool-budget`) + session-end (`no-tool-errors`) |
| `manifest.toml` | Run manifest referencing the `close-the-loop` spec |
| `prompt-abort.md` | Reference prompt for the abort path (encourages 3+ tool calls) |
| `prompt-clean.md` | Reference prompt for the promote path (completes in ≤1 tool call) |
| `setup.sh` | One-time setup: copies specs into `.assay/specs/`, clears state |
| `reset.sh` | Re-run setup: restores status, clears sessions/reviews |
| `run-abort.sh` | Runs the abort path and shows review output |
| `run-promote.sh` | Runs the promote path and shows review output |
