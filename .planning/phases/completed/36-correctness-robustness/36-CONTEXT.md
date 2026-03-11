# Phase 36: Correctness & Robustness - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix three correctness issues in MCP tools: worktree_status computes ahead/behind relative to base branch (not upstream), gate session errors distinguish timeout vs not-found with recovery hints, and git diff is captured and attached to gate sessions with a 32 KiB cap and truncation flag.

</domain>

<decisions>
## Implementation Decisions

### Base branch resolution (FIX-01)
- Base branch is **stored at worktree creation time** in worktree metadata — deterministic, no guessing at query time
- Ahead/behind computed against **remote-tracking ref with local fallback** (try `origin/<base>` first, fall back to `refs/heads/<base>` if no remote configured)
- **Optional `fetch` parameter** on `worktree_status` (defaults to false) — user opts in when freshness matters; no auto-fetch by default
- If base branch ref doesn't exist: return **null ahead/behind counts + warning** in warnings array — rest of status still returned, non-blocking

### Error message design (FIX-02)
- Recovery hints **suggest specific MCP tool calls** — e.g., "Session timed out after 120s. Use gate_run to start a new session." Actionable for agent consumers
- Timeout errors include **both elapsed time and configured timeout** — e.g., "Session xyz timed out after 120s (timeout: 120s)"
- Not-found errors include **only the missing session ID** — no listing of active sessions. Agent can call session_list separately if needed
- gate_report and gate_finalize use the **same error format** — one consistent error shape for both tools

### Diff capture (FIX-03)
- Diff captured **at gate_run time** — represents the state being evaluated, single clear point in time
- Uses `git diff HEAD` — **staged + unstaged changes** relative to HEAD
- Truncation uses **assay's existing head-biased truncation engine** (from Phase 29) with 32 KiB cap — consistent behavior, already tested
- Clean worktree (no diff): field is **null/absent** (Option<String> = None) — distinguishes "no diff captured" from "captured but empty"
- `diff_truncated: bool` flag indicates whether truncation occurred

### Claude's Discretion
- Exact field names on AgentSession struct (diff_content vs diff_text vs diff)
- Whether to store diff_bytes_original alongside diff_truncated
- Error message exact wording (pattern is locked, phrasing is flexible)
- How to handle git command failures during diff capture (likely: warn + continue without diff)

</decisions>

<specifics>
## Specific Ideas

- Reuse the truncation engine from Phase 29 (crates/assay-core) rather than reimplementing head-biased truncation
- The `fetch` parameter on worktree_status should feel like a convenience, not a default — most calls should be fast/local
- Error messages are consumed by AI agents, so clarity and actionability matter more than brevity

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 36-correctness-robustness*
*Context gathered: 2026-03-11*
