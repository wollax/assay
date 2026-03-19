---
id: T02
parent: S06
milestone: M002
provides:
  - CLI multi-session detection routing single-session to run_manifest and multi-session to run_orchestrated + merge
  - --failure-policy and --merge-strategy CLI flags on assay run
  - OrchestrationResponse JSON output type for orchestrated runs
  - needs_orchestration() pure detection function
key_files:
  - crates/assay-cli/src/commands/run.rs
key_decisions:
  - Base branch auto-detected via git rev-parse --abbrev-ref HEAD before orchestration, consistent with T01 MCP approach
  - Session runner closure constructed inside execute_orchestrated using plain function calls (D035) — same pattern as MCP tool
patterns_established:
  - CLI orchestration path mirrors MCP tool pattern: session_runner closure → run_orchestrated → checkout base → extract_completed_sessions → merge_completed_sessions
  - Separate execute_sequential / execute_orchestrated functions keep single-session path unchanged
observability_surfaces:
  - CLI stderr output includes orchestration phase markers (Phase 1/2/3) with per-session status and merge report summary
  - --json flag returns OrchestrationResponse with run_id, per-session outcomes, merge_report, and aggregate summary
  - Exit codes: 0 = all succeed + clean merge, 1 = any error/skip, 2 = merge conflicts
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Wire CLI multi-session routing with orchestrator and merge

**Added multi-session detection and orchestrated execution + merge routing to `assay run` CLI with `--failure-policy` and `--merge-strategy` flags.**

## What Happened

Extended the `RunCommand` struct with two new CLI flags (`--failure-policy` and `--merge-strategy`) using custom value parsers that map kebab-case CLI strings to the existing `FailurePolicy` and `MergeStrategy` enums from assay-types.

Added a pure `needs_orchestration()` function that detects whether a manifest requires orchestrated execution (more than one session, or any session with `depends_on`). Single-session manifests without dependencies continue through the unchanged `run_manifest()` path via `execute_sequential()`.

Multi-session manifests route through `execute_orchestrated()` which follows the same three-phase pattern established by the MCP `orchestrate_run` tool in T01: (1) orchestrated parallel execution via `run_orchestrated()`, (2) base branch checkout, (3) sequential merge via `merge_completed_sessions()`. The session runner closure uses plain function calls (D035) to construct a `HarnessWriter` per-session.

Added `OrchestrationResponse`, `OrchestrationSessionResult`, and `OrchestrationSummary` types for structured JSON output of orchestrated runs.

## Verification

- `cargo test -p assay-cli -- run` — 12 tests pass (4 existing + 8 new)
- `cargo clippy -p assay-cli -- -D warnings` — clean
- Slice-level checks:
  - `cargo test -p assay-mcp` — 27 passed (T01 tests intact)
  - `cargo test -p assay-core --features orchestrate -- integration` — 26 passed
  - `just ready` — not run (T03 pending, will be final check)

## Diagnostics

- Run `assay run manifest.toml --json` to get structured `OrchestrationResponse` with run_id, per-session outcomes, and merge report
- Exit code 0 = all succeed + clean merge, 1 = any error/skip, 2 = merge conflicts
- Phase progress visible on stderr: Phase 1 (execution) → Phase 2 (checkout) → Phase 3 (merge)

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — Added --failure-policy/--merge-strategy flags, needs_orchestration() detection, execute_orchestrated() path, OrchestrationResponse types, and 8 new tests
