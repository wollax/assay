# Merge-Back Workflow & Branch Strategy: Explorer Proposals

**Explorer:** explorer-mergeback
**Date:** 2026-03-10
**Context:** v0.3.0 has worktree create/list/status/cleanup but no merge-back. v0.4.0 adds WorkSession + gate_evaluate but "completed" just means gates passed — no merge step.

---

## Proposal 1: `worktree_merge` MCP Tool — The Minimal Viable Merge

### What
Add a single `worktree_merge` MCP tool that takes a spec slug, validates gates passed, and performs a git merge (or squash/rebase per config) of the worktree branch into the base branch. Returns a structured `MergeResult` with commit SHA, files changed, and any warnings.

The tool operates in the **main worktree** (not the feature worktree), running:
1. Pre-merge validation: latest gate run for spec must pass all required criteria
2. Conflict detection via `git merge --no-commit --no-ff` dry run
3. If clean: execute merge with configured strategy (merge/squash/rebase)
4. If conflicts: return structured conflict report (file list, conflict markers count) without completing merge
5. Post-merge: optionally run gates again on the merged result

### Why
- **Smallest useful increment.** One MCP tool closes the orchestration loop.
- **Follows existing patterns.** Same subprocess git model as `worktree_create`/`worktree_cleanup`.
- **Agents can invoke it.** An AI agent can call `worktree_merge` after `gate_finalize` reports success, completing the full spec→work→gate→merge loop.
- **Human stays in control.** MCP tool requires explicit invocation (no daemon auto-merging).

### Scope
Medium (3-5 days). New MCP tool + types + core function + tests. Parallels existing worktree CRUD pattern.

### Risks
- Merge conflicts mid-flow leave git state dirty — needs careful cleanup on failure
- Squash vs rebase vs merge commit choice affects git history readability
- Post-merge gate re-run could fail, leaving merged-but-not-gated code on main
- No rollback mechanism if post-merge gates fail

---

## Proposal 2: `MergeStrategy` Config with Pre-Merge Gate Enforcement

### What
Add a `[merge]` config section to `.assay/config.toml`:

```toml
[merge]
strategy = "squash"           # squash | rebase | merge-commit
require_clean_worktree = true # no uncommitted changes
require_gate_pass = true      # all required gates must pass
require_up_to_date = true     # branch must be rebased on latest base
auto_cleanup = true           # delete worktree+branch after merge
```

The `MergeStrategy` type lives in `assay-types`, config parsing in `assay-core`. All merge operations consult this config. The strategy is spec-overridable via a `[merge]` section in the spec TOML itself.

### Why
- **Declarative over imperative.** Teams declare their branch strategy once; every merge follows it.
- **Spec-level override.** A high-risk spec can require `merge-commit` (preserving full history) while low-risk specs use `squash`.
- **Enforces quality invariants.** `require_gate_pass` prevents merging ungated work. `require_up_to_date` prevents merge conflicts at merge time.
- **Builds on existing config system.** `Config` already has `worktree`, `gates`, `guard` sections.

### Scope
Small-Medium (2-3 days). Types + config parsing + validation logic. No merge execution — that's Proposal 1.

### Risks
- Config complexity: too many knobs creates cognitive overhead
- Spec-level override creates precedence confusion (project vs spec)
- `require_up_to_date` forces rebase before merge, which can itself introduce conflicts

---

## Proposal 3: Conflict Detection & Resolution Pipeline

### What
A two-phase merge pipeline that handles conflicts gracefully:

**Phase 1 — Conflict Detection (`merge_check`):**
- `git merge-tree --write-tree` (Git 2.38+) for zero-side-effect conflict detection
- Returns `MergeCheck { clean: bool, conflicts: Vec<ConflictInfo>, base_sha: String, head_sha: String }`
- `ConflictInfo` includes file path, conflict type (content/rename/delete), and affected line ranges

**Phase 2 — Conflict Resolution Strategies:**
- `auto`: Only proceed if merge is clean (abort on any conflict)
- `rebase-retry`: Rebase worktree branch on latest base, re-run gates, then attempt merge
- `agent-resolve`: Return conflict info to the calling agent for manual resolution in the worktree, then re-check
- `human-escalate`: Write conflict report to `.assay/conflicts/<spec-slug>.json`, set WorkSession status to `conflict_detected`, await human

### Why
- **Conflicts are the #1 merge failure mode.** Multi-agent work on the same codebase guarantees conflicts.
- **`git merge-tree` is revolutionary.** Zero working-tree pollution for conflict detection — perfect for the "check before act" pattern.
- **Graduated response.** Simple cases auto-resolve; complex cases escalate. No single strategy fits all.
- **Agent-compatible.** The `agent-resolve` path lets AI agents fix conflicts in their worktree, re-gate, and retry merge.

### Scope
Medium-Large (5-7 days). Requires git 2.38+ dependency. New types, core logic, integration with merge tool.

### Risks
- `git merge-tree` requires Git 2.38+ (released 2022-10, should be fine)
- Agent conflict resolution could produce worse code than the original
- Rebase-retry loop could theoretically cycle (rebase → new conflicts → rebase → ...)
- Line-range conflict info parsing from git is fragile

---

## Proposal 4: PR Creation Bridge — `merge_propose`

### What
A `merge_propose` MCP tool that creates a pull request (or equivalent) instead of direct merging:

1. Push worktree branch to remote (if not already pushed)
2. Create PR via `gh pr create` (GitHub) or equivalent
3. Attach gate results as PR body/comment (formatted markdown table)
4. Set PR labels based on gate outcome (`gates-passed`, `advisory-warnings`, etc.)
5. Return `MergeProposal { pr_url: String, pr_number: u64, gate_summary: GateRunSummary }`

For non-GitHub forges, shell out to configurable commands:
```toml
[merge.propose]
push_cmd = "git push -u origin {branch}"
create_cmd = "gh pr create --title '{spec_name}' --body-file {gate_report_path} --base {base_branch}"
```

### Why
- **PR is the universal merge pattern.** Every team uses PRs. Direct branch merges are unusual in practice.
- **Human approval is natural.** `autonomous: false` maps perfectly to "PR created, human reviews and merges."
- **Gate results in PR body is compelling.** Reviewers see structured quality evidence alongside code changes.
- **Forge-agnostic via config.** GitHub gets first-class `gh` support; GitLab/Azure DevOps use configurable commands.

### Scope
Medium (3-5 days). MCP tool, template rendering for PR body, configurable push/create commands. GitHub-first, extensible.

### Risks
- PR creation is a side effect visible to others — risky if misconfigured
- `gh` CLI dependency for GitHub (but it's standard tooling)
- PR body formatting needs to handle large gate reports gracefully
- Non-GitHub forges may have very different PR creation semantics

---

## Proposal 5: Multi-Worktree Merge Ordering & Dependency Graph

### What
When N worktrees are active, merging them in the wrong order can create cascading conflicts. This proposal adds:

1. **Merge queue** — ordered list of specs ready to merge, with dependency tracking
2. **Dependency detection** — file-overlap heuristic: if worktree A and B both modify `src/auth.rs`, they conflict-depend
3. **Topological merge order** — merge non-conflicting worktrees first, re-gate conflicting ones after earlier merges land
4. **`merge_queue` MCP tool** — returns `MergeQueue { ready: Vec<MergeCandidate>, blocked: Vec<BlockedMerge> }`

```rust
struct MergeCandidate {
    spec_slug: String,
    gate_passed: bool,
    files_changed: Vec<String>,
    conflicts_with: Vec<String>,  // other spec slugs
}
```

### Why
- **Multi-agent is the whole point.** Assay orchestrates N agents. Without merge ordering, the Nth merge is likely to conflict with changes from merges 1..N-1.
- **File-overlap heuristic is cheap.** `git diff --name-only base..branch` per worktree, then set intersection.
- **Enables batch merging.** Non-conflicting specs merge in parallel; conflicting ones serialize intelligently.
- **Unique differentiator.** No existing tool handles multi-agent merge ordering. agtx certainly doesn't.

### Scope
Large (1-2 weeks). Dependency detection, queue management, merge orchestration. This is orchestrator territory.

### Risks
- File overlap is a heuristic — semantic conflicts (two functions using same global) won't be detected
- Queue management adds state that must be persisted and recovered
- Merge order optimization is NP-hard in the general case (but greedy heuristics suffice)
- Premature for v0.4.0 — this is really v0.5.0+ orchestrator work

---

## Proposal 6: WorkSession Merge State Machine Extension

### What
Extend the planned v0.4.0 `WorkSession` type to include merge-related states:

```
created → agent_running → gate_evaluated → merge_ready → merging → merged | conflict_detected | merge_failed
                                                                          ↓
                                                                   conflict_resolving → gate_evaluated (re-enter)
```

New states:
- `merge_ready`: Gates passed, eligible for merge
- `merging`: Merge in progress (short-lived)
- `merged`: Successfully merged, worktree can be cleaned up
- `conflict_detected`: Merge attempted, conflicts found
- `conflict_resolving`: Agent or human resolving conflicts
- `merge_failed`: Merge failed for non-conflict reasons (push rejected, branch protection, etc.)

Persisted in the same `.assay/sessions/<session-id>.json` as WorkSession.

### Why
- **Natural extension of v0.4.0 work.** WorkSession is already being built with phase transitions. Adding merge phases is incremental.
- **State machine prevents invalid transitions.** Can't merge without gates passing. Can't clean up without merge completing.
- **Observable.** TUI dashboard can show session state. Agents can query "what needs merging?" via MCP.
- **Foundation for orchestrator.** The v0.5.0+ orchestrator/daemon needs exactly this state machine to manage N sessions.

### Scope
Small (1-2 days if WorkSession is already in progress). Just additional enum variants and transition validation.

### Risks
- Adding merge states to v0.4.0 WorkSession scope-creeps the milestone
- State machine complexity: conflict resolution creates cycles (conflict_detected → conflict_resolving → gate_evaluated → merge_ready → ...)
- Persistence format changes may not be backwards-compatible

---

## Proposal 7: Post-Merge Verification Gate — The Safety Net

### What
After merge completes, automatically run gates again on the merged main branch to verify the merge didn't break anything:

1. Merge completes on base branch
2. Run all gates for the merged spec against the base branch working directory
3. If gates pass: mark session as `verified`, clean up worktree
4. If gates fail: **auto-revert the merge commit** and set session to `merge_reverted`
5. Record the post-merge gate run as a separate history entry linked to the original

```toml
[merge]
post_merge_verify = true       # run gates after merge
auto_revert_on_failure = false # revert merge if post-merge gates fail (dangerous!)
```

### Why
- **Merge can introduce bugs even with clean conflict resolution.** Semantic conflicts (function signature changed in one branch, caller changed in another) pass git's merge but fail compilation.
- **Auto-revert is the ultimate safety net.** With `autonomous: false`, this gives humans confidence to approve merges knowing Assay will catch and revert problems.
- **Proves gate value.** Running gates pre-merge AND post-merge demonstrates that gates catch real issues.
- **Closes the trust loop.** The orchestration loop isn't just spec→work→gate→merge — it's spec→work→gate→merge→verify.

### Scope
Medium (3-4 days). Post-merge gate re-run + conditional revert + session state updates.

### Risks
- Auto-revert is destructive — reverting on main affects all developers
- Post-merge gate run on main blocks other merges (serial bottleneck)
- Gate flakiness causes spurious reverts
- Revert of a squash merge loses all commit history for that spec's work
