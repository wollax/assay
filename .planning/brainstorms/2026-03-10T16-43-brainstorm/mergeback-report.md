# Merge-Back Workflow & Branch Strategy: Final Report

**Explorer:** explorer-mergeback | **Challenger:** challenger-mergeback
**Date:** 2026-03-10
**Rounds of debate:** 3
**Source proposals:** [mergeback-ideas.md](mergeback-ideas.md)

---

## Executive Summary

7 proposals entered debate. After 3 rounds, 2 survived for near-term implementation, 3 were deferred to later milestones, and 2 were killed. The core insight: **assay's merge-back mission (per PROJECT.md) is best served in v0.4.x by read-only conflict detection and PR-based workflows, not by building a git merge engine.**

---

## Surviving Proposals

### 1. `merge_check` — Read-Only Conflict Detection (v0.4.0 or v0.4.1)

**What:** A standalone read-only MCP tool that uses `git merge-tree --write-tree` (Git 2.38+) to detect merge conflicts without side effects. Returns structured `MergeCheck { clean: bool, conflicts: Vec<ConflictInfo>, base_sha, head_sha }`.

**Why this survived:**
- Zero side effects — safe for any agent to call at any time
- No working tree pollution (unlike `git merge --no-commit` which stages changes)
- Immediately useful: agents can check mergeability before proposing PRs
- Teaches the team about merge mechanics before building merge execution

**Scope:** 1-2 days. New MCP tool, types, core function wrapping `git merge-tree`.

**Key decisions from debate:**
- Must be standalone, NOT bundled with a resolution pipeline (challenger critique)
- Conflict resolution strategies (agent-resolve, rebase-retry, human-escalate) deferred to v0.5.0+
- Requires Git 2.38+ (released Oct 2022, acceptable minimum)

### 2. `merge_propose` — PR Creation with Gate Evidence (v0.4.0 or v0.4.1)

**What:** An MCP tool that creates a pull request with gate results attached, closing the orchestration loop for `autonomous: false` workflows.

Flow:
1. Push worktree branch to remote (if not already pushed)
2. Create PR via `gh pr create` (GitHub-first default path)
3. Attach gate results as formatted markdown in PR body (truncated with link to full results)
4. Return `MergeProposal { pr_url, pr_number, gate_summary }`

**Why this survived:**
- PR is the universal merge pattern — every team uses PRs
- Maps naturally to `autonomous: false` (human reviews and merges)
- Gate results in PR body gives reviewers structured quality evidence
- Sidesteps all direct-merge complexity (locking, main worktree mutation, rollback)

**Scope:** 3-4 days. MCP tool, gate report formatting, `gh` integration, env-var extensibility.

**Key decisions from debate:**
- **GitHub-first via `gh` CLI.** Only tested path in v0.4.x. Clear error if `gh` not available.
- **Forge-agnostic via environment variables, NOT template interpolation.** Assay sets `$ASSAY_BRANCH`, `$ASSAY_SPEC`, `$ASSAY_GATE_REPORT_PATH` and the user provides their own command string. No parsing, no injection risk.
- **`dry_run: bool` parameter.** Shows what PR would be created without actually creating it. Consistent with read-before-write philosophy. (Challenger addition, Round 3.)
- **PR body truncation.** GitHub has a 65536 char limit. Gate results must be truncated with a link to full results file for large gate runs.
- **Push-to-remote is a documented side effect.** This is assay's first remote-modifying operation. Must require explicit invocation, never auto-trigger.

---

## Deferred Proposals

### 3. WorkSession Merge State Machine (v0.5.0, after WorkSession stabilizes)

**What:** Extend WorkSession with merge-related states: `merge_ready → merging → merged | conflict_detected | merge_failed`, with `conflict_resolving` re-entering `gate_evaluated`.

**Why deferred, not killed:**
- Natural extension of WorkSession, but WorkSession itself is v0.4.0 planned-not-built
- State machine has cycles (conflict resolution loop) requiring `max_merge_attempts` termination
- Crash recovery for `merging` state is non-trivial: must reconcile git state vs session state (e.g., process dies after `git merge` succeeds but before session file writes — naive "abort" would undo a completed merge)
- Build AFTER WorkSession proves stable in production

**Prerequisite:** WorkSession base implementation shipped and battle-tested.

### 4. `worktree_merge` — Direct Merge Execution (v0.5.0+, if needed)

**What:** Direct merge execution for `autonomous: true` workflows where PR creation is unnecessary overhead.

**Why deferred, not killed:**
- Original proposal had a fundamental flaw: merging in the main worktree mutates shared state, creates locking issues, and the "dry run" via `git merge --no-commit` is NOT side-effect-free
- If built, must use `git merge-tree --write-tree` for detection + temporary worktree for execution, never touching the main working tree
- May not be needed: if `merge_propose` (PR workflow) serves all real use cases, direct merge is unnecessary complexity
- Gated on: evidence that `autonomous: true` users need direct merge AND that PR workflow is insufficient

### 5. Conflict Resolution Strategies (v0.5.0+)

**What:** Graduated conflict resolution: auto (clean-only) → rebase-retry → agent-resolve → human-escalate.

**Why deferred, not killed:**
- Agent conflict resolution is viable (return structured conflict info, agent edits files, re-gates serve as quality signal) but unvalidated
- Rebase-retry loop needs bounded retries (agreed: tractable but untested)
- Human-escalate is "creating a PR with extra steps" — may be unnecessary if `merge_propose` exists
- Needs real conflict scenarios to validate before shipping

---

## Killed Proposals

### 6. Auto-Revert on Post-Merge Gate Failure — KILLED

**Original idea:** Auto-revert merge commits on main when post-merge gates fail.

**Why killed:**
- Terrifying in multi-developer repos (developers who pulled post-merge now have diverged history)
- Gate flakiness × auto-revert = data loss
- Contradicts `autonomous: false` philosophy — a human should decide what to do when post-merge gates fail
- `git revert` of merge commits requires `-m 1` parent selection and reverting a revert later creates confusing history

**What survives:** Post-merge verification as a **read-only warning** (run gates on merged main, report results, but never auto-revert) is reasonable future work.

### 7. `MergeStrategy` Config Schema — KILLED (absorbed)

**Original idea:** Standalone `[merge]` config section with strategy, boolean knobs, spec-level overrides.

**Why killed:**
- Classic YAGNI: config schema for a feature that doesn't exist
- 48+ configuration combinations (2^4 × 3 strategies) with unclear interaction semantics
- Spec-level override precedence is a footgun (can specs bypass `require_gate_pass`?)
- When merge tools ship, hardcode sensible defaults (`require_gate_pass = true`, strategy = squash). Extract config only when users demonstrate need for different settings.

### 8. Multi-Worktree Merge Ordering — KILLED (externalized)

**Original idea:** File-overlap dependency detection, topological merge order, merge queue.

**Why killed:**
- File overlap is necessary-but-insufficient (semantic conflicts undetected)
- Merge queue is stateful daemon infrastructure, not MCP tool territory
- **Key insight from challenger:** If agents create PRs via `merge_propose`, GitHub's merge queue feature already handles topological ordering, conflict detection, and CI re-runs. This may never need to be built.
- Investigate GitHub merge queue coverage before building any custom solution.

---

## Milestone Placement: v0.4.0 vs v0.4.1

**Open question:** v0.4.0 is already scoped at 11 phases of headless orchestration work (0% complete). Adding 5-6 days of merge tooling is a scope risk.

**Recommendation:** Add as **Phase 44.5 (decimal phase)** in v0.4.0 if it can run in parallel with existing phases, OR defer to **v0.4.1 fast-follow** if v0.4.0 scope is at risk. The headless orchestration work (gate_evaluate, WorkSession) does NOT depend on merge tooling, so merge tools can ship independently.

**If v0.4.0:** `merge_check` and `merge_propose` as a single phase with 2 plans, inserted after Phase 40 (WorkSession) since `merge_propose` benefits from WorkSession context.

**If v0.4.1:** Ship as the sole milestone feature — small, focused, immediately valuable.

---

## Architectural Decisions

| Decision | Rationale | Status |
|----------|-----------|--------|
| PR creation over direct merge for v0.4.x | Maps to `autonomous: false`, avoids main worktree mutation | **Agreed** |
| `git merge-tree --write-tree` for conflict detection | Zero side effects, no working tree pollution | **Agreed** |
| Environment variables for forge-agnostic extensibility | No template parsing, no injection risk, user writes own command | **Agreed** |
| GitHub-first via `gh` CLI | Standard tooling, first-class support, clear error if missing | **Agreed** |
| Hardcode defaults, extract config from usage | Avoid premature config abstraction (YAGNI) | **Agreed** |
| Kill auto-revert permanently | Contradicts `autonomous: false`, flakiness × auto-revert = data loss | **Agreed** |
| Investigate GitHub merge queue before building multi-worktree ordering | External solution may make custom ordering unnecessary | **Agreed** |
| Merge states on WorkSession AFTER base WorkSession stabilizes | Avoid designing two features simultaneously; crash recovery is non-trivial | **Agreed** |
| `dry_run` parameter on `merge_propose` | Read-before-write consistency; preview remote side effects | **Agreed** |
| Merge-back is core scope (per PROJECT.md), not scope creep | Project vision explicitly includes "automated merge-back strategies" | **Agreed** |

---

## Implementation Sketch

### `merge_check` MCP Tool

```
Input:  { spec_slug: String }
Output: { clean: bool, conflicts: Vec<ConflictInfo>, base_sha: String, head_sha: String }

Core: git merge-tree --write-tree <base_sha> <branch_sha> in project_root
Parse: exit code 0 = clean, non-zero = conflicts; parse conflict file list from output
```

### `merge_propose` MCP Tool

```
Input:  { spec_slug: String, dry_run: bool (default false) }
Output: { pr_url: String?, pr_number: u64?, gate_summary: GateRunSummary, dry_run: bool }

Steps:
1. Validate latest gate run passed all required criteria
2. Run merge_check — warn if conflicts detected (PR can still be created)
3. If dry_run: return preview without creating PR
4. Push branch: git push -u origin <branch>
5. Write gate report to temp file
6. Create PR: gh pr create --title "<spec_name>" --body-file <report> --base <base_branch>
7. Parse PR URL/number from gh output
8. Return MergeProposal
```

---

*Consolidated from 3 rounds of explorer/challenger debate — 2026-03-10*
