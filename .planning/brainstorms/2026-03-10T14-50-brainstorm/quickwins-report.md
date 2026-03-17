# Quick-Win Proposals for Assay v0.4.0 — Consolidated Report

_Explorer: explorer-quickwins | Challenger: challenger-quickwins | Date: 2026-03-10_
_Debate rounds: 3 | Final status: 6 accepted proposals (1 merged, 1 downscoped, 1 reframed)_

---

## Executive Summary

Seven proposals entered debate. After three rounds of challenge and revision, six survive in final form. One proposal (Dry-Run Mode) was merged into an existing tool rather than added as new API surface. The six final proposals address four themes: **agent observability**, **headless robustness**, **correctness fixes**, and **v0.4.0 scaffolding**.

Total estimated scope: ~18–22 hours across 6 proposals. All are independently deliverable.

---

## Final Proposals

### QW-1: Diff Context Attached to Gate Sessions

**Status:** Accepted (reshaped from original)

**What:** When `gate_run` is called in a git worktree, capture `git diff HEAD` and attach it to the `AgentSession` as `diff_context: Option<String>`. Include it in the session metadata returned alongside `pending_criteria`. The diff is stored server-side on the session — it is not injected into criterion prompts automatically, but is available to agents that include it in their `gate_report` reasoning.

**Why:** v0.4.0's `gate_evaluate` capstone requires diff context for quality evaluation. This establishes the data capture seam now, using the existing subprocess pattern. Full server-side criterion prompt injection is a later step.

**Implementation Notes:**
- Run `git diff HEAD` via existing subprocess pattern (not a new dep)
- Apply 32 KiB cap with head-biased truncation (first changed files first)
- If path is not a git repo or `git diff` fails: `diff_context: None`, log warning — no hard failure
- Add `diff_truncated: bool` alongside `diff_context` so consumers know if it was cut
- `git diff HEAD` runs in the worktree root; fallback is `working_dir` passed to `gate_run`

**Scope:** 6–8h

**Risks:**
- Large monorepo diffs are still 32 KiB even truncated — agents should budget accordingly
- If gate is run outside a git repo, diff is silently absent (acceptable, documented)

---

### QW-2: Better Session-Not-Found Error Messages

**Status:** Accepted (downscoped from tombstone proposal)

**What:** Improve `gate_report` and `gate_finalize` error responses for missing sessions to distinguish two cases:
1. **Session timed out** (in-process): include spec name, criteria already reported, and hint to re-run `gate_run`
2. **Session not found** (server restart or bad ID): include hint to re-run `gate_run`

Currently both return an opaque "session not found" string with no recovery guidance.

**Why:** Headless agents in long-running worktree sessions (core v0.4.0 scenario) hit the 30-minute in-process timeout (SESSION_TIMEOUT_SECS = 1800). Without a structured error, agents retry blindly or give up. A structured error with a re-run hint enables automatic recovery.

**Implementation Notes:**
- Existing session HashMap already stores `AgentSession` with spec_name and evaluations
- Timeout task already fires at 1800s — add a brief tombstone (5-min window, in-memory only) before removal, carrying spec_name and evaluation count
- No disk persistence required; tombstone lives only as long as the server process

**Scope:** 1–2h

**Risks:** Tombstone is in-process only — doesn't survive server restart (documented limitation)

---

### QW-3: Worktree Status Relative to Base Branch

**Status:** Accepted (as-is with clarification)

**What:** Fix `worktree_status` returning `ahead: 0, behind: 0` when the worktree branch has no remote upstream. Assay-managed worktrees (`assay/<spec-slug>` branches) never have upstreams. Instead, compute:
- `ahead`: `git rev-list --count <base>..HEAD`
- `behind`: `git rev-list --count HEAD..<base>`

where `<base>` is the **current tip** of the base branch (not the tip at creation time).

**Why:** `ahead: 0, behind: 0` gives false confidence to agents deciding whether to merge or rebase. Headless agents need to know "main has 5 new commits since I branched" to decide on rebase before gate evaluation.

**Implementation Notes:**
- `WorktreeInfo` already records `base_branch` at creation time
- "Current branch tip" semantics: `git rev-parse <base_branch>` resolves to current HEAD of that branch
- Fallback to `0/0` only if base branch no longer exists (deleted branch edge case)

**Scope:** 2–3h

**Risks:** Base branch tip changes between worktree creation and status check — this is intentional (users need current divergence, not historical divergence)

---

### QW-4: Outcome-Filtered Gate History

**Status:** Accepted (with explicit semantics)

**What:** Add two optional parameters to `gate_history`:
- `outcome: "passed" | "failed" | "any"` (default: `"any"`)
- `limit: u32` (default: 10, max: 50)

**Outcome semantics:** `"failed"` = `required_failed > 0` (equivalent to `blocked: true`). Advisory-only failures count as `"passed"`. This matches `gate_finalize` enforcement semantics.

**Why:** Headless agents evaluating regressions need "show me the last 5 failed runs" not all history. Token-budgeted context (v0.4.0) means every unnecessary record is waste.

**Implementation Notes:**
- History files are per-spec JSON in `.assay/results/<spec>/`
- Load all records (O(n)), filter, return top-N by timestamp descending
- `max_history` defaults to 50 — worst case is 50 file reads (acceptable)
- Document O(n) behavior in API; no index required at this scale

**Scope:** 2–3h

**Risks:** O(n) scan scales with history depth — safe under default `max_history: 50`; users disabling pruning accept the tradeoff

---

### QW-5: `spec_get` Resolved Configuration

**Status:** Accepted (merged from standalone Dry-Run proposal)

**What:** Add an optional `resolve` parameter to `spec_get`:
```
resolve?: {
  timeout_override?: u64,   // seconds; mirrors gate_run's override
  working_dir?: string      // validate existence and return resolved path
}
```

When provided, `spec_get` returns a `resolved_config` block alongside the raw spec showing:
- Per-criterion effective timeouts (applying 3-tier precedence: override > per-criterion > config > 300s default)
- Working dir exists: `true/false`

**Why:** Headless agents pre-flighting a gate run need "what will criterion X time out at?" The 3-tier timeout precedence is not visible through `spec_get` today. This enables agents to check feasibility before committing to a long gate run. No new tool surface.

**Implementation Notes:**
- Timeout resolution logic already exists in gate evaluation; extract as a pure function
- `resolve` parameter is optional — `spec_get` without it is unchanged
- Return `resolved_config` only when `resolve` is provided

**Scope:** 3–4h

**Risks:** None significant; additive to existing tool

---

### QW-6: Consistent `warnings` Field Across Mutating MCP Tools

**Status:** Accepted (expanded from single-tool fix)

**What:** Add `#[serde(default, skip_serializing_if = "Vec::is_empty")] warnings: Vec<String>` to the response types for all mutating MCP tools: `gate_run`, `gate_report`, `gate_finalize`, `worktree_create`. When history persistence fails, when diff capture fails, when branch deletion warns during cleanup — surface it here instead of silently logging.

This closes the known open issue: `.planning/issues/open/2026-03-10-history-save-failure-not-surfaced.md`.

**Why:** Agents in CI and headless mode can't observe `tracing::warn` logs. History save failures silently produce gaps in `gate_history`. `warnings` is the MCP-native way to surface soft failures without breaking callers. Also establishes the response envelope pattern for v0.4.0 tools.

**Implementation Notes:**
- `skip_serializing_if = "Vec::is_empty"` means `warnings` only appears in JSON when non-empty — zero breaking change
- Propagate `Result<()>` from `save_run` through response builders; map to warning strings
- Convention: warnings are informational (call succeeded); errors use `isError: true`

**Scope:** 2–3h

**Risks:**
- Sets a precedent that every failure mode should have a warning — that's fine, intentional
- Agents that don't check `warnings` miss the signal — unavoidable, but documented

---

### QW-7: Session Growth Rate Metrics in `estimate_tokens`

**Status:** Accepted (reframed from baseline comparison)

**What:** Add growth rate metrics to the `estimate_tokens` response:
```
growth_rate?: {
  current_turn: u32,          // count of AssistantEntry entries so far
  avg_tokens_per_turn: u32,   // rolling average over last 10 assistant turns
  estimated_turns_remaining: u32  // at current rate before hitting context budget
}
```

Only populated when there are at least 5 assistant turns in the session (too few turns = noisy average).

**"Turn" definition:** One `SessionEntry::Assistant` entry. The parser already distinguishes `SessionEntry::User` and `SessionEntry::Assistant` as distinct enum variants — no new parsing infrastructure required.

**Why:** Point-in-time token counts are less actionable than "at this rate you have ~23 turns left." Headless agents managing long sessions can decide to checkpoint, prune, or wrap up based on projected remaining budget — not just current usage.

**Implementation Notes:**
- `estimate_tokens` already parses the JSONL session via the existing parser
- Add rolling window: collect `raw_bytes` of last 10 `SessionEntry::Assistant` entries
- `estimated_turns_remaining = (budget - current_tokens) / avg_tokens_per_turn`
- Budget constant from existing context budget config (already used by guard daemon)

**Scope:** 3h

**Risks:**
- Token estimates are approximate (raw bytes ≠ exact token count) — acceptable for planning heuristics
- Very short sessions (<5 turns) don't get `growth_rate` — document this threshold

---

## Summary Table

| ID | Proposal | Theme | Scope | Debate Outcome |
|----|----------|-------|-------|----------------|
| QW-1 | Diff Context on Gate Sessions | v0.4.0 scaffolding | 6–8h | Reshaped: session-attached, not payload-returned |
| QW-2 | Better Session Error Messages | Headless robustness | 1–2h | Downscoped: better errors only, no tombstone disk persistence |
| QW-3 | Worktree Status vs. Base Branch | Correctness | 2–3h | Accepted as-is; base = current branch tip |
| QW-4 | Outcome-Filtered Gate History | Agent observability | 2–3h | Accepted; outcome=failed means blocked=true |
| QW-5 | `spec_get` Resolved Config | Agent observability | 3–4h | Merged from dry-run; no new tool surface |
| QW-6 | `warnings` on Mutating Tools | Observability / tech debt | 2–3h | Expanded to all mutating tools; closes open issue |
| QW-7 | Growth Rate in `estimate_tokens` | Agent observability | 3h | Reframed from baseline; turn = AssistantEntry |

**Total estimated scope: 19–26h across 7 proposals (6 new + 1 merged)**

---

## Proposals Dropped

- **Standalone Gate Dry-Run Mode:** Value exists but is entirely subsumed by `spec_get` resolved config (QW-5). Adding a separate `dry_run` parameter to `gate_run` would bloat API surface for no additional value.
- **Context Diagnose Baseline Comparison:** Under-specified. "Session got bigger" is expected. Replaced by growth rate metrics in `estimate_tokens` (QW-7), which are actionable.

---

## Implementation Priority Recommendation

High-confidence, low-risk first:
1. **QW-6** (warnings pattern) — enables all other observability improvements; 2–3h; pure additive
2. **QW-3** (worktree status) — pure bug fix; 2–3h; no design decisions
3. **QW-2** (session errors) — ~1h; headless agent UX immediately better
4. **QW-4** (outcome-filtered history) — 2–3h; useful standalone
5. **QW-5** (spec_get resolved config) — 3–4h; reduces pre-flight uncertainty
6. **QW-7** (growth rate metrics) — 3h; supports long-session headless work
7. **QW-1** (diff on sessions) — 6–8h; highest leverage but needs v0.4.0 context for full value
