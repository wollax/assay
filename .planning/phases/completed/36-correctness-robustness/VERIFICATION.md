# Phase 36 Verification Report

**Verifier:** Claude Code (claude-sonnet-4-6)
**Date:** 2026-03-11
**Result:** PASSED

---

## Method

Each must_have was verified by reading the actual source files. No SUMMARY documents were used as evidence.

Key files read:
- `crates/assay-types/src/worktree.rs`
- `crates/assay-types/src/session.rs`
- `crates/assay-core/src/worktree.rs`
- `crates/assay-core/src/gate/mod.rs`
- `crates/assay-core/src/gate/session.rs`
- `crates/assay-mcp/src/server.rs`
- `crates/assay-mcp/tests/mcp_handlers.rs`

---

## Plan 01 — Worktree Base-Branch Status

| Must-Have | Verified | Evidence |
|-----------|----------|---------|
| `worktree_status` computes ahead/behind relative to base branch, not `@{upstream}` | PASS | `crates/assay-core/src/worktree.rs:319–370`: uses `rev-list --left-right --count HEAD...origin/<base>` (or local fallback), no `@{upstream}` reference anywhere |
| Base branch persisted at `<worktree_path>/.assay/worktree.json` at creation time | PASS | `write_metadata()` called in `create()` at line 250–256; `WorktreeMetadata { base_branch, spec_slug }` written to `.assay/worktree.json` |
| Ahead/behind uses remote-tracking ref `origin/<base>` with local fallback `refs/heads/<base>` | PASS | Lines 321–343: tries `refs/remotes/origin/<base>`, falls back to `refs/heads/<base>` |
| `WorktreeStatus.ahead` and `WorktreeStatus.behind` are `Option<usize>` | PASS | `crates/assay-types/src/worktree.rs:77–79`: `ahead: Option<usize>`, `behind: Option<usize>` |
| Missing base ref returns null counts + warning string, rest of status still returned | PASS | Lines 338–343: warning pushed, `(None, None)` returned; full `WorktreeStatus` still constructed at line 372 |
| `worktree_status` accepts optional `fetch` parameter (defaults to false) | PASS | `WorktreeStatusParams.fetch: Option<bool>` in `server.rs:217`; handler uses `params.0.fetch.unwrap_or(false)` at line 1168 |
| `list()` reads metadata to populate `base_branch` on `WorktreeInfo` | PASS | `list()` at line 282: `let base_branch = read_metadata(&wt.path).map(|m| m.base_branch)` |

All Plan 01 must-haves: **7/7 PASS**

---

## Plan 02 — Session Error Messages

| Must-Have | Verified | Evidence |
|-----------|----------|---------|
| Timeout errors include both elapsed time and configured timeout in the message | PASS | `session_not_found_error()` in `server.rs:1231–1238`: `"Session '{session_id}' timed out after {elapsed}s (configured timeout: {}s) for spec '{}'."` |
| Not-found errors include only the missing session ID, no listing of active sessions | PASS | Lines 1240–1245: message contains session ID and recovery hints only; test at `mcp_handlers.rs:419–422` asserts `!text.contains("active sessions")` |
| Recovery hints suggest specific MCP tool calls (`gate_run`, `gate_history`) | PASS | Both branches of `session_not_found_error()` include `"Use gate_run to start a new session, or gate_history to review past results."` |
| `gate_report` and `gate_finalize` use the same error format via a shared helper | PASS | Both call `self.session_not_found_error(&p.session_id).await` (lines 817, 875); test `gate_report_and_finalize_not_found_errors_are_consistent` verifies same pattern |
| Timed-out sessions tracked so not-found can distinguish timeout from never-existed | PASS | `timed_out_sessions: Arc<Mutex<HashMap<String, TimedOutInfo>>>` field; timeout task inserts `TimedOutInfo` at lines 735–757 |
| `timed_out_sessions` has a capacity cap to prevent unbounded growth | PASS | `const MAX_TIMED_OUT_ENTRIES: usize = 100` at line 450; enforced at line 738 with LRU-style eviction |

All Plan 02 must-haves: **6/6 PASS**

---

## Plan 03 — Diff Capture

| Must-Have | Verified | Evidence |
|-----------|----------|---------|
| `git diff HEAD` (staged + unstaged) captured at `gate_run` time | PASS | `server.rs:667–686`: `Command::new("git").args(["diff", "HEAD"])` run in `gate_run` handler before session creation |
| Diff truncated using assay's head-biased truncation engine with 32 KiB cap | PASS | `const DIFF_BUDGET_BYTES: usize = 32 * 1024` (line 447); `assay_core::gate::truncate_diff(&raw, DIFF_BUDGET_BYTES)` called at line 674 |
| `AgentSession` has `diff: Option<String>`, `diff_truncated: bool`, `diff_bytes_original: Option<usize>` | PASS | `crates/assay-types/src/session.rs:133–142`: all three fields present with correct types and serde attributes |
| Clean worktree (no diff) stores `None`, not `Some("")` | PASS | `truncate_diff()` in `gate/mod.rs:721–731`: if `raw.is_empty()` returns `(None, false, None)` |
| Git command failure during diff capture warns and continues without diff (non-blocking) | PASS | `server.rs:676–684`: non-success output warns via `tracing::warn!` and returns `(None, false, None)`; error path also warns and returns same |
| `truncate_head_tail` and `TruncationResult` are `pub(crate)` visible | PASS | `gate/mod.rs:661`: `pub(crate) struct TruncationResult`; line 682: `pub(crate) fn truncate_head_tail`; `pub fn truncate_diff` wraps it as public API |

All Plan 03 must-haves: **6/6 PASS**

---

## Phase Success Criteria (from ROADMAP.md)

1. **`worktree_status` computes ahead/behind relative to base branch tip, not upstream**: CONFIRMED. Uses `rev-list --left-right --count HEAD...origin/<base>` resolved from persisted metadata, never `@{upstream}`.

2. **Gate session errors distinguish timeout vs not-found with recovery hints**: CONFIRMED. `session_not_found_error()` shared helper distinguishes the two cases with different message content; both include recovery hint `gate_run`.

3. **`git diff HEAD` (32 KiB cap, head-biased truncation) stored on AgentSession with `diff_truncated` flag**: CONFIRMED. All three fields present on `AgentSession`, captured non-blockingly at `gate_run` time, truncated via `truncate_diff` with `DIFF_BUDGET_BYTES = 32 * 1024`.

---

## Summary

**Total must-haves checked: 19/19 PASS**

No gaps found. All three plans are correctly implemented in the codebase. `just ready` was confirmed passing by the orchestrator.
