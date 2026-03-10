# Phase 4: Sequential Merge - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement the core merge operation — take outputs from multiple agent worktrees and merge them sequentially into a single target branch. Clean merges (no conflicts) work end-to-end. This is the central value proposition of Smelt. Conflict resolution is out of scope (Phase 6-7).

</domain>

<decisions>
## Implementation Decisions

### Target branch semantics
- Merge creates a new branch: `smelt/merge/<manifest-name>`
- Base is the common ancestor commit where sessions originally branched from
- User can override target branch name via `--target` flag
- Name collision (branch already exists) is refused with a clear error — consistent with worktree branch collision behavior

### Merge strategy
- Merge-on-top when target has diverged (no rebase)
- Squash merge per session — each session's commits become one commit on the target branch
- LLM-generated commit messages from session metadata (session name, task description)
- Sessions merged sequentially, one at a time

### Merge failure behavior
- On conflict: abort entire sequence, roll back target branch (delete it)
- Error output: session name + conflicting file list to stderr
- No partial progress preserved — in a clean-only world, an incomplete branch has no value
- Phase 6 will change this to preserve-on-failure when human resolution exists

### Failed/incomplete sessions
- `smelt merge <manifest.toml>` — explicit manifest path required, mirrors `smelt session run`
- Only successfully completed sessions are merged
- Failed/incomplete sessions skipped with warning to stderr
- Require at least 1 completed session, refuse with error if zero

### Output and reporting
- Real-time progress to stderr: `[1/3] Merging "session-name"... ok`
- Final summary to stdout: per-session diff stat (files changed, insertions, deletions)
- stderr for progress/warnings, stdout for summary — consistent with existing convention
- No dry-run flag (deferred to Phase 5: Merge Order Intelligence)

### Cleanup
- Session worktree branches auto-cleaned after full merge sequence succeeds
- Cleanup happens after entire sequence, not per-session
- On failure, worktrees are preserved (rollback deletes target branch, not source worktrees)

### Claude's Discretion
- LLM prompt design for commit message generation
- Exact diff stat formatting
- Internal merge implementation details (git merge mechanics)
- Error message wording and formatting
- How to detect/store the common ancestor commit from session state

</decisions>

<specifics>
## Specific Ideas

- Separate commands intentional: `smelt session run` and `smelt merge` stay independent. Combined lifecycle is Phase 8 (Orchestration Plan).
- Progress output style consistent with `smelt session run` per-session reporting.
- Summary output should be concise — similar to `git diff --stat` density.

</specifics>

<deferred>
## Deferred Ideas

- `--dry-run` for merge order preview — Phase 5 (Merge Order Intelligence)
- Conflict resolution (human fallback) — Phase 6
- AI conflict resolution — Phase 7
- Combined run+merge lifecycle — Phase 8 (Orchestration Plan)
- Structured JSON reporting — Phase 9 (Session Summary)

</deferred>

---

*Phase: 04-sequential-merge*
*Context gathered: 2026-03-09*
