# Requirements: Assay v0.4.1 Merge Tools

## Merge Tools

- [ ] **MERGE-01**: `merge_check` MCP tool — uses `git merge-tree --write-tree` (Git 2.38+) to detect conflicts without side effects, returns structured `MergeCheck { clean, conflicts, base_sha, head_sha }`
- [ ] **MERGE-02**: `merge_propose` MCP tool — pushes worktree branch, creates PR via `gh pr create` with gate evidence in body, returns `MergeProposal { pr_url, pr_number, gate_summary, dry_run }`
- [ ] **MERGE-03**: `merge_propose` supports `dry_run: bool` parameter — previews PR without creating it, consistent with read-before-write philosophy
- [ ] **MERGE-04**: Gate evidence formatting — gate results formatted as markdown in PR body, truncated with link to full results for GitHub's 65536 char limit
- [ ] **MERGE-05**: Forge-agnostic extensibility — `merge_propose` sets `$ASSAY_BRANCH`, `$ASSAY_SPEC`, `$ASSAY_GATE_REPORT_PATH` env vars; GitHub-first via `gh`, clear error if `gh` not available

## Worktree Fixes

- [ ] **WFIX-01**: Worktree cleanup `--all` uses canonical path from git instead of string comparison
- [ ] **WFIX-02**: Default branch detection provides actionable error instead of silently falling back to `main`
- [ ] **WFIX-03**: Git worktree prune failures are surfaced as warnings instead of silently discarded

## Traceability

| Requirement | Phase | Theme |
|-------------|-------|-------|
| MERGE-01 | TBD | Merge Tools |
| MERGE-02 | TBD | Merge Tools |
| MERGE-03 | TBD | Merge Tools |
| MERGE-04 | TBD | Merge Tools |
| MERGE-05 | TBD | Merge Tools |
| WFIX-01 | TBD | Worktree Fixes |
| WFIX-02 | TBD | Worktree Fixes |
| WFIX-03 | TBD | Worktree Fixes |

**Coverage:** 8/8 requirements mapped (100%)

---

## Future Requirements (deferred)

- [ ] WorkSession merge state machine (merge_ready → merging → merged | conflict_detected) — v0.5.0
- [ ] `worktree_merge` direct merge execution for `autonomous: true` — v0.5.0+
- [ ] Conflict resolution strategies (auto/rebase/agent/human escalation) — v0.5.0+
- [ ] Criteria libraries with `include` field — v0.5.0
- [ ] `spec_diff` git-based spec comparison — v0.5.0
- [ ] `extends:` single-level spec inheritance — v0.5.0

## Out of Scope

- Auto-revert on post-merge gate failure — contradicts `autonomous: false`, flakiness × auto-revert = data loss
- `MergeStrategy` config schema — YAGNI, hardcode sensible defaults first
- Multi-worktree merge ordering — GitHub merge queue handles this externally
- Direct merge to main — PR workflow serves all v0.4.1 use cases
