---
phase: 10
plan: "10-02"
subsystem: plugin-hooks
tags: [claude-code, hooks, stop-hook, post-tool-use, cli, gate-run]
dependency-graph:
  requires: [phase-7, phase-9, 10-01]
  provides: [gate-run-all, post-tool-use-hook, stop-hook-enforcement]
  affects: []
tech-stack:
  added: []
  patterns: [stop-hook-guard-chain, reminder-only-post-tool-use, configurable-enforcement]
key-files:
  created:
    - plugins/claude-code/scripts/post-tool-use.sh
    - plugins/claude-code/scripts/stop-gate-check.sh
  modified:
    - crates/assay-cli/src/main.rs
    - plugins/claude-code/hooks/hooks.json
decisions:
  - "`--all` flag on gate run uses clap conflicts_with for mutual exclusivity with name"
  - "handle_gate_run_all reuses evaluate/resolve_timeout helpers from assay-core::gate"
  - "JSON output for --all is Vec<GateRunSummary> array (one per spec)"
  - "Streaming output for --all shows per-spec headers with separators"
  - "PostToolUse script drains stdin to avoid broken pipe (jq not needed)"
  - "Stop hook guard chain: stop_hook_active > mode=off > no .assay/ > no binary > run gates"
  - "ASSAY_STOP_HOOK_MODE env var controls enforcement (enforce/warn/off, default enforce)"
  - "Stop hook extracts failed count from JSON array with jq for block reason"
  - "bash prefix in hooks.json avoids execute permission issues"
metrics:
  duration: "5m 41s"
  completed: "2026-03-03"
---

# Phase 10 Plan 02: Hooks and CLI --all Flag Summary

CLI `--all` flag for running gates across all specs, PostToolUse reminder hook on Write/Edit, and Stop hook with configurable gate enforcement and five safety guards preventing infinite loops and graceful degradation.

## What Was Done

### Task 1: Add --all flag to gate run CLI command

Modified `GateCommand::Run` to accept optional `name` and boolean `--all` with `conflicts_with = "name"` for mutual exclusivity. Created `handle_gate_run_all` function that:

- Loads config and scans specs directory via `assay_core::spec::scan`
- JSON path: collects `Vec<GateRunSummary>` and serializes as JSON array
- Streaming path: iterates specs with per-spec headers, reuses `format_pass`/`format_fail`/`print_evidence` helpers
- Aggregate summary line shows spec count and totals
- Exits 1 if any spec has failures, 0 otherwise
- Bare `gate run` (neither name nor `--all`) prints error and exits 1

Updated help text at both command and top-level to include `--all` examples.

### Task 2: PostToolUse reminder hook

Created `plugins/claude-code/scripts/post-tool-use.sh`:
- Drains stdin to avoid broken pipe (no JSON parsing needed)
- Outputs `hookSpecificOutput` with `additionalContext` reminder
- Uses "when you're done" phrasing to avoid reminder fatigue (per Research pitfall 4)
- Always exits 0 (reminder should never block)

Updated `plugins/claude-code/hooks/hooks.json`:
- PostToolUse entry with `Write|Edit` matcher and 5-second timeout
- Stop entry with 120-second timeout
- Both use `bash ${CLAUDE_PLUGIN_ROOT}/scripts/...` prefix for permission safety

Removed `plugins/claude-code/hooks/.gitkeep` (now has real content).

### Task 3: Stop hook gate enforcement script

Created `plugins/claude-code/scripts/stop-gate-check.sh` with five safety guards:

1. **stop_hook_active guard**: If `true`, allows stop immediately (prevents infinite loops per Research pitfall 1)
2. **ASSAY_STOP_HOOK_MODE=off guard**: User-disabled enforcement
3. **No .assay/ directory guard**: Graceful degradation (per Research pitfall 5)
4. **Missing binary guard**: Graceful degradation (per Research pitfall 2)
5. **ASSAY_STOP_HOOK_MODE=warn guard**: Allows stop without blocking on gate failure

Default enforce mode runs `assay gate run --all --json`, parses failed count from JSON array with jq, and outputs `{"decision": "block", "reason": "..."}` with actionable instructions.

## Deviations from Plan

None -- plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
| --- | --- |
| clap `conflicts_with` for --all vs name | Declarative mutual exclusivity; clap generates clear error message automatically |
| `name: Option<String>` instead of required | Enables bare `gate run` error path and `--all` mode without name |
| Vec<GateRunSummary> for JSON output | One summary per spec; consistent with single-spec GateRunSummary structure |
| stdin drain without jq in PostToolUse | Script always reminds on Write/Edit; no need to parse tool input |
| Guard chain order in Stop hook | Most critical guards first: infinite loop prevention before any other logic |
| jq for JSON parsing in Stop hook | Reliable JSON field extraction per Research recommendation |

## Verification Results

- `just ready` passes (fmt-check, clippy, test, deny, plugin-version)
- `--all` flag appears in `gate run --help` output
- `--all` and spec name are mutually exclusive (clap error exit 2)
- Bare `gate run` produces error with guidance (exit 1)
- hooks.json validates as JSON with PostToolUse and Stop entries
- post-tool-use.sh outputs valid JSON with additionalContext
- stop-gate-check.sh: stop_hook_active=true allows stop (no output)
- stop-gate-check.sh: no .assay/ directory allows stop (no output)
- stop-gate-check.sh: ASSAY_STOP_HOOK_MODE=off allows stop (no output)

## Next Phase Readiness

Phase 10 is complete. All plugin deliverables are in place:
- PLG-01: plugin.json (Plan 01)
- PLG-02: .mcp.json (Plan 01)
- PLG-03: /gate-check skill (Plan 01)
- PLG-04: /spec-show skill (Plan 01)
- PLG-05: CLAUDE.md workflow snippet (Plan 01)
- PLG-06: PostToolUse reminder hook (Plan 02)
- PLG-07: Stop hook enforcement (Plan 02)
- CLI: `assay gate run --all` for multi-spec evaluation (Plan 02)
