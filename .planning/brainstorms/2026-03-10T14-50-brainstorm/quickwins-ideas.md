# Quick-Win Ideas for Assay v0.4.0 — Explorer Proposals

_Explorer: explorer-quickwins | Date: 2026-03-10_

---

## Idea 1: `gate_evaluate` Diff-Context Injection

**Name:** Diff-Context Gate Evaluation

**What:** Before running AI-evaluated criteria via `gate_run`, automatically capture `git diff HEAD` from the associated worktree and inject it as context into the returned `pending_criteria` payload. Agents receive the diff alongside their criterion prompts, eliminating the manual "fetch diff before evaluating" step.

**Why:** The v0.4.0 capstone is `gate_evaluate` with diff context and context budgeting. This directly scaffolds that by making diff context first-class in the existing `gate_run`/`gate_report` flow — no new MCP tool needed yet, just a richer payload. It also unblocks headless Claude Code agents who need context to evaluate without extra tool calls.

**Scope:** ~4–6 hours. `gate_run` already knows the worktree path. Add diff capture (same subprocess pattern: `git diff HEAD`, head+tail truncation at existing budget), inject into `AgentSessionPayload`.

**Risks:**
- Diffs can be large — need to apply existing 32KiB truncation budget carefully
- Worktree may not be specified if gate is run from main tree (needs fallback)
- Adds latency to `gate_run` (git subprocess); should be async

---

## Idea 2: Session Timeout Notification via `gate_report` Response

**Name:** Graceful Session Expiry Signals

**What:** When an agent calls `gate_report` or `gate_finalize` with an expired/timed-out session ID, instead of returning an opaque "session not found" error, return a structured `SessionExpired` error with: the session's original spec, which criteria were already reported, and a hint to re-run `gate_run`. Currently, sessions silently vanish after 30 minutes with no recovery path.

**Why:** Headless agents running in long worktree sessions (a v0.4.0 core scenario) will hit this. A dead session with no signal causes agents to retry blindly or fail silently. This is a 1-day fix that makes the system dramatically more robust for the agentic use case.

**Scope:** ~3–4 hours. `AgentSession` already stores spec_name and agent_evaluations. Preserve a "tombstone" record in a separate `expired_sessions: HashMap<SessionId, SessionTombstone>` for a short window (e.g., 5 minutes) after expiry. `gate_report` checks tombstones before returning "not found."

**Risks:**
- Tombstone window adds slight memory overhead (bounded, trivially small)
- Doesn't solve the root cause (long-running tasks) — need context about why sessions expire

---

## Idea 3: `worktree_status` Ahead/Behind Without Upstream

**Name:** Offline Worktree Status

**What:** `worktree_status` currently returns `ahead: 0, behind: 0` when there's no upstream tracking branch. For assay-managed worktrees (created from local base branches), this is always the case. Instead, compute `ahead` relative to the worktree's base branch (stored at creation time) via `git rev-list --count <base>..HEAD`.

**Why:** Agents and the TUI call `worktree_status` to decide whether to push/merge. Silently returning 0/0 when there's no upstream gives false confidence. Showing commits-ahead-of-base-branch is actionable. This is a pure correctness fix with no API change — just better data.

**Scope:** ~2–3 hours. `WorktreeInfo` already has `base_branch`. Persist it alongside the worktree (or re-derive from branch name since convention is `assay/<spec-slug>`). Adjust `worktree_status` to fall back to base-branch comparison.

**Risks:**
- Base branch may have advanced since worktree creation (ahead/behind is still relative to current base HEAD — acceptable)
- Branch name convention parsing is fragile if convention changes

---

## Idea 4: `gate_history` Filtering by Outcome

**Name:** Outcome-Filtered Gate History

**What:** Add optional `outcome` filter parameter to `gate_history` (`passed | failed | any`, default `any`). Also add a `limit` parameter (default 10, max 50). Currently, agents receive all runs up to the cap with no way to ask "show me the last failed runs for this spec."

**Why:** For context-budgeted headless evaluation (v0.4.0), agents need recent failure context to understand regressions — not all history. This is the cheapest way to surface useful historical signal without building a full query system. Touches only the MCP layer and history loading.

**Scope:** ~2–3 hours. History files are already per-spec JSON. Post-filter in memory after loading. MCP tool input schema update, filter loop, done.

**Risks:**
- History can grow large if pruning is misconfigured — still loads all, then filters (acceptable for now)
- `outcome` semantics need clear definition: "all required passed" vs "no failures"

---

## Idea 5: Structured `gate_run` Dry-Run Mode

**Name:** Gate Dry-Run

**What:** Add a `dry_run: bool` parameter to `gate_run`. When true, validate the spec, resolve criteria, and return the command list and agent criteria names — without executing anything. No subprocess spawned, no session created, no history written.

**Why:** Headless agents orchestrating complex gate workflows (v0.4.0 scenario) need to pre-flight: "what will this gate do?" before committing to a worktree operation. Also valuable for debugging specs from the CLI and for the TUI "preview" UX. Zero risk to existing behavior.

**Scope:** ~3–4 hours. Early-return path after spec loading and criteria resolution. Return a new `DryRunResult` variant in the MCP response instead of `GateRunResult`.

**Risks:**
- Dry-run results may diverge from real run if environment differs (acceptable — it's a preview)
- Adds complexity to the response type union; MCP schema must handle the new variant cleanly

---

## Idea 6: History Save Failure Surfacing

**Name:** Surface History Save Failures to Agents

**What:** When `gate_run` (command-only) or `gate_finalize` fail to persist the history record, include a `warnings: Vec<String>` field in the MCP response. Currently, persistence failures are logged with `tracing::warn` and silently dropped. This is a known open issue (#history-save-failure-not-surfaced).

**Why:** Agents operating in CI or headless mode have no way to know their gate results weren't persisted. This means `gate_history` will silently miss runs. For a system built on audit trails, this is a correctness gap. The fix is purely additive — add `warnings` to the response types.

**Scope:** ~2 hours. Propagate the `Result<()>` from `save_run` through the existing response builders. Map errors to warning strings. No behavior change, pure observability improvement.

**Risks:**
- Agents may not check `warnings` — but at least the signal is available
- Warning verbosity could confuse simple pipelines that only look at success/failure

---

## Idea 7: `context_diagnose` Baseline Comparison

**Name:** Diagnose Against Saved Baseline

**What:** Add a `compare_to_baseline: bool` option to `context_diagnose`. When enabled, load the previous diagnostic snapshot from `.assay/context-baseline.json`, compute deltas (token growth, new bloat sources, size changes), and include them in the response. On first run, save the current state as the baseline.

**Why:** The guard daemon watches for threshold violations, but agents can't easily see *rate of growth* without context. A baseline comparison turns `context_diagnose` from a point-in-time snapshot into a trend detector — "since last checkpoint, your context grew 8K tokens, primarily in tool results." This is high-value for headless agents managing long sessions.

**Scope:** ~4–5 hours. Snapshot format is straightforward (serialize current DiagnosticResult). Delta computation is simple arithmetic. File persistence uses the same atomic write pattern already in history.

**Risks:**
- Baseline staleness: if session is cleared, baseline is meaningless — need session-ID correlation
- Storage location (`.assay/context-baseline.json`) is global, not per-worktree — could cause confusion in parallel worktree sessions

