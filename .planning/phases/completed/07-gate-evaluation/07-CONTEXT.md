# Phase 7: Gate Evaluation - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Command gates execute, capture evidence, enforce timeouts, and produce structured results that both CLI and MCP can consume. This phase covers `gate::evaluate()` core logic, the `assay gate run <spec>` CLI command, the `GateKind::FileExists` variant, and aggregate result summaries. MCP tool wrappers are Phase 8.

</domain>

<decisions>
## Implementation Decisions

### CLI result display
- Stream progress during execution: show criterion name + spinner while running, replace with pass/fail on completion
- After all criteria finish, show a summary table at the end
- Default display: pass/fail per criterion, with stdout/stderr evidence shown automatically for failing criteria only
- `--verbose` / `-v` flag shows evidence for all criteria (including passing ones)
- `--json` flag for structured JSON output, consistent with `assay spec show --json` and `assay spec list --json`

### Failure and continuation
- Always run all criteria regardless of failures — user sees the full picture, no early exit
- Descriptive-only criteria (no `cmd`) are skipped — only executable criteria count toward pass/fail; descriptive ones listed separately (e.g., "2 skipped")

### Timeout configuration
- Three-tier precedence: CLI `--timeout` flag > per-criterion `timeout` field in spec > global `timeout_seconds` in config.toml `[gates]`
- Default timeout: 300 seconds (from GATE-03 requirement)

### Evidence capture limits
- Truncate stdout/stderr capture at a sensible default limit (tail-biased since errors appear at end)

### Claude's Discretion
- Exit code strategy when criteria fail (exit 1 vs failure count)
- AlwaysPass gate handling in summary (include in count vs list separately)
- Timeout behavior: whether timed-out criteria report as "failed" or a distinct "timed_out" state
- CLI `--timeout` override scope: whether it caps everything or only replaces the global default
- Minimum timeout floor for validation
- Exact truncation limit (bytes/lines) and whether it's configurable or hardcoded for v0.1.0
- Binary/non-UTF8 output handling strategy
- Truncation indicator in CLI display

</decisions>

<specifics>
## Specific Ideas

- Streaming display should feel like `cargo test` — criterion name visible while running, then result appears
- Evidence on failure mirrors how test runners show assertion details only when tests fail
- The three-tier timeout precedence follows standard CLI convention: flag > file-level > global config

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-gate-evaluation*
*Context gathered: 2026-03-02*
