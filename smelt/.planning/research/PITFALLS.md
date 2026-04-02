# Pitfalls — Multi-Agent Worktree Orchestration

Research dimension for Smelt v0.1.0 milestone. Each pitfall includes warning signs, prevention strategy, and the phase that should address it.

---

## 1. Git Worktree Lifecycle

### 1.1 Orphaned Worktrees After Crash

**What goes wrong:** If Smelt (or the host machine) crashes mid-session, worktrees remain on disk with no process managing them. `git worktree list` still shows them. Subsequent runs may fail because the branch is "already checked out in another worktree" — git enforces a one-branch-per-worktree invariant.

**Warning signs:**
- `git worktree list` shows worktrees whose paths no longer have a running agent
- Branches that cannot be checked out with "already checked out" errors
- Disk usage growing over time from accumulated dead worktrees

**Prevention strategy:**
- Record worktree-to-PID mappings in a manifest file (e.g., `.smelt/worktrees.json`)
- On startup, reconcile: for each recorded worktree, check if the PID is alive. If not, run `git worktree remove --force <path>` and clean up
- Register a shutdown hook (signal handler for SIGTERM/SIGINT) that removes worktrees on graceful exit
- Periodic `git worktree prune` to clean stale administrative files

**Phase:** v0.1.0 core — this is day-one infrastructure.

---

### 1.2 Worktree Lock Files Block Operations

**What goes wrong:** `git worktree lock` (or the automatic lock git places on worktrees) prevents removal or pruning. If a worktree was locked before a crash, it stays locked forever. Automated cleanup then silently fails or errors out, and the operator never notices until disk is full or branches are stuck.

**Warning signs:**
- `git worktree remove` fails with "is locked"
- `git worktree list` shows locked worktrees that have no active session

**Prevention strategy:**
- Never use `git worktree lock` in the orchestrator — it exists for manual workflows where humans move worktree directories
- If locks are found during reconciliation, check PID liveness before force-unlocking (`git worktree unlock`)
- Log every lock/unlock decision for auditability

**Phase:** v0.1.0 core.

---

### 1.3 Branch Name Collisions Across Worktrees

**What goes wrong:** Git forbids two worktrees from having the same branch checked out. If Smelt creates predictable branch names (e.g., `smelt/task-1`) and a previous worktree with that branch was not cleaned up, the new session cannot start. Detached HEAD workarounds introduce their own problems (commits not on any branch, easy to lose).

**Warning signs:**
- "fatal: '<branch>' is already checked out at '<path>'" errors
- Sessions starting in detached HEAD unexpectedly

**Prevention strategy:**
- Use unique, generated branch names with a session ID component (e.g., `smelt/<task>/<session-uuid-short>`)
- Before creating a worktree, verify the target branch name is not checked out elsewhere (`git worktree list --porcelain` and parse)
- Never use detached HEAD for agent worktrees — always create a named branch so commits are traceable

**Phase:** v0.1.0 core.

---

### 1.4 Shared Git Objects and Index Contention

**What goes wrong:** All worktrees share the same `.git` object store and certain config. Concurrent `git gc`, `git repack`, or `git maintenance` operations can interfere with active worktrees. Concurrent writes to `packed-refs` can corrupt ref resolution. Running `git gc --prune=now` while an agent is mid-commit can delete objects the agent is about to reference.

**Warning signs:**
- Intermittent "object not found" errors in agent sessions
- Corrupted packfiles or ref resolution failures
- `git fsck` reporting dangling objects that should not be dangling

**Prevention strategy:**
- Disable automatic git maintenance/gc in the main repo during orchestration (`git config gc.auto 0`)
- Run gc/maintenance only when no agent sessions are active (between orchestration rounds)
- Use `git worktree list` to confirm all worktrees are idle before maintenance
- Set a conservative `gc.pruneExpire` (e.g., 2 weeks) so loose objects are not aggressively removed

**Phase:** v0.1.0 core for disabling gc; maintenance scheduling in v0.2+.

---

## 2. Process Management

### 2.1 Zombie Agent Processes

**What goes wrong:** Smelt spawns Claude Code (or other agents) as child processes. If Smelt crashes, these children become orphans and keep running — consuming CPU, memory, and API tokens. Worse: they keep writing to their worktrees, making cleanup impossible without killing them first.

**Warning signs:**
- Agent processes running with no parent (PPID = 1 or init)
- Token/API costs continuing to accrue after Smelt exits
- Worktree directories being modified after Smelt thinks the session ended

**Prevention strategy:**
- Use process groups: spawn each agent in its own process group, so `kill(-pgid, SIGTERM)` cleans up the entire tree
- On macOS/Linux, set `PR_SET_PDEATHSIG` (Linux) or use kqueue `EVFILT_PROC` (macOS) so children receive SIGTERM when parent dies
- Record PIDs in the worktree manifest; reconciliation on startup kills orphans
- Implement a heartbeat: if the agent process does not respond within N seconds, consider it dead and initiate cleanup
- Set a hard timeout per session — agents that exceed it get SIGTERM, then SIGKILL after a grace period

**Phase:** v0.1.0 core — process lifecycle is foundational.

---

### 2.2 Resource Exhaustion From Parallel Agents

**What goes wrong:** Each agent session may spawn subprocesses (language servers, test runners, build tools). With N agents, the machine may run out of file descriptors, memory, or disk I/O. macOS has particularly low default file descriptor limits (256 per process).

**Warning signs:**
- "Too many open files" errors
- System becoming unresponsive during orchestration
- OOM kills (check `dmesg` or Console.app)

**Prevention strategy:**
- Start with a small concurrency limit (2-3 agents) and make it configurable
- Monitor resource usage per session if possible (at minimum, log memory/CPU per agent PID)
- Set `ulimit -n` appropriately before spawning agents
- Document minimum system requirements for N-agent orchestration

**Phase:** v0.1.0 (concurrency limit); v0.2+ (monitoring/observability).

---

### 2.3 Agent Output Capture and Streaming

**What goes wrong:** Claude Code writes to stdout/stderr, may use terminal escape codes, and may prompt for input. If Smelt captures output naively (e.g., piping to a file), interactive features break. If it does not capture output, there is no audit trail. Buffering mismatches (line-buffered vs. block-buffered) cause output to appear delayed or interleaved.

**Warning signs:**
- Agent sessions hanging because they are waiting for input that Smelt never provides
- Garbled or missing output in logs
- Output appearing only after the process exits (block buffering)

**Prevention strategy:**
- Use PTY allocation for agent subprocesses to preserve interactive behavior and avoid buffering issues
- Feed a non-interactive flag to agents where available (`claude --non-interactive` or equivalent)
- Capture output to per-session log files with timestamps
- Detect and handle stdin prompts — either auto-respond or escalate to human

**Phase:** v0.1.0 for basic capture; PTY handling in v0.1.0 if Claude Code requires it.

---

### 2.4 Signal Handling Chain

**What goes wrong:** When the user sends Ctrl+C to Smelt, the signal propagates to child processes unpredictably. Some agents may catch SIGINT and enter cleanup mode (taking time), others may die immediately, leaving partial commits. The orchestrator needs to coordinate graceful shutdown across all agents before cleaning up worktrees.

**Warning signs:**
- Partial commits left in worktree branches after Ctrl+C
- Some agents exit cleanly while others leave corrupted state
- Worktree cleanup failing because agents are still writing

**Prevention strategy:**
- Trap SIGINT/SIGTERM in Smelt and implement ordered shutdown: signal agents first, wait for exit (with timeout), then clean up worktrees
- Spawn agents in separate process groups so they do not receive the terminal's SIGINT directly
- Implement a two-phase shutdown: first SIGTERM (graceful), then SIGKILL after timeout

**Phase:** v0.1.0 core.

---

## 3. Merge and Conflict Resolution

### 3.1 Semantic Conflicts That Merge Cleanly

**What goes wrong:** Two agents modify different files that git merges without conflict, but the combined result is semantically broken. Classic example: Agent A renames a function, Agent B adds a call to the old function name. Git sees no textual conflict, but the code does not compile. This is the single most dangerous class of merge problem because it is silent.

**Warning signs:**
- Merged branch passes `git merge` with no conflicts but fails tests/build
- Bugs appearing in merged code that existed in neither agent's branch individually

**Prevention strategy:**
- ALWAYS run build + tests on the merged result before considering it successful (this is the Assay integration surface for later milestones, but even v0.1.0 needs a basic post-merge verification hook)
- Implement a "merge-then-verify" loop: merge, run verification, and if it fails, flag for human review
- Consider task decomposition that minimizes cross-cutting changes (e.g., do not assign two agents to modify the same module)
- For v0.1.0: at minimum, provide a hook point where a build/test command can be run post-merge

**Phase:** v0.1.0 (hook point); v0.2+ (Assay gate integration).

---

### 3.2 Merge Order Affects Outcome

**What goes wrong:** When merging 3+ agent branches, the order matters. Merging A then B may succeed, but merging B then A may produce conflicts. Worse: both orders may succeed but produce different semantic results. This makes merge outcomes non-deterministic from the user's perspective.

**Warning signs:**
- Same set of agent outputs producing different merged results on retry
- Conflicts appearing that did not appear in a previous run with the same inputs

**Prevention strategy:**
- Define a deterministic merge order (e.g., alphabetical by branch name, or by completion time)
- Document that merge order is significant and make it configurable
- Consider octopus merge (`git merge A B C`) for simple cases, but be aware it aborts on any conflict (no partial resolution)
- For complex cases, implement sequential merge with rollback: if merging branch N fails, try reordering, or flag for human review
- Log the exact merge order used for every orchestration run

**Phase:** v0.1.0 core — deterministic ordering is essential for reproducibility.

---

### 3.3 AI Conflict Resolution Introduces Bugs

**What goes wrong:** Using an LLM to resolve merge conflicts is appealing but risky. The LLM may "resolve" a conflict by picking one side arbitrarily, by hallucinating a synthesis that looks plausible but changes behavior, or by silently dropping code from one side. The resolution looks clean but is semantically wrong.

**Warning signs:**
- AI-resolved conflicts that remove functionality from one side
- Resolved code that does not match the intent of either agent's changes
- Post-merge tests failing on AI-resolved files specifically

**Prevention strategy:**
- Classify conflicts by complexity: trivial (whitespace, import ordering) vs. semantic (logic changes). Only auto-resolve trivial conflicts
- Always present AI resolutions to the human for review, at least in v0.1.0 (build trust before automating)
- Include both sides of the conflict in the AI prompt with full file context, not just the conflict markers
- Run post-resolution verification (build/test) and automatically reject AI resolutions that break the build
- Log every AI resolution decision with the original conflict, the resolution, and the rationale

**Phase:** v0.1.0 core — AI resolution with mandatory human review.

---

### 3.4 Ref Races During Concurrent Operations

**What goes wrong:** If two merge operations run concurrently (or an agent pushes while Smelt is merging), ref updates can race. `git update-ref` is atomic for a single ref, but multi-ref updates are not transactional. Two processes updating the same branch ref simultaneously can result in one overwriting the other's changes (lost update).

**Warning signs:**
- Commits disappearing from branch history
- "non-fast-forward" errors during merge
- Divergent branch states that should be identical

**Prevention strategy:**
- Serialize all merge operations — never run two merges concurrently against the same target branch
- Use a merge queue pattern: agents complete work, then a single merge worker processes them sequentially
- Use `git merge --no-ff` to create explicit merge commits, making history auditable
- Consider advisory file locks (e.g., `.smelt/merge.lock`) to prevent concurrent merge operations

**Phase:** v0.1.0 core — single-threaded merge worker.

---

### 3.5 Large Diffs Overwhelm AI Conflict Resolution

**What goes wrong:** If agents make large, sweeping changes (reformatting, dependency upgrades, generated code), the resulting diffs and conflicts may be too large for LLM context windows. The AI either truncates context and produces wrong resolutions, or the API call fails/times out entirely.

**Warning signs:**
- Conflict files with hundreds of conflict markers
- AI resolution API calls timing out or returning errors
- Token costs spiking for conflict resolution

**Prevention strategy:**
- Set guidelines/limits on the scope of changes per agent session
- Pre-process conflicts: split large conflict files into smaller chunks, resolve independently, reassemble
- Implement a "too complex for AI" threshold (e.g., >N conflict markers or >M lines changed) that automatically escalates to human
- For generated/formatted files, consider resolving by taking one side entirely and re-running the generator

**Phase:** v0.1.0 (threshold + human escalation); v0.2+ (chunked resolution).

---

## 4. Git-Specific Coordination

### 4.1 HEAD and Index State Corruption

**What goes wrong:** Each worktree has its own HEAD and index, but they share refs and objects. If Smelt or an agent runs git commands against the wrong worktree (e.g., using the main repo's git dir instead of the worktree's), it can corrupt the main repo's HEAD or index. This is especially easy to do when using git libraries programmatically — many default to the repo root, not the worktree.

**Warning signs:**
- Main repo HEAD changing unexpectedly
- Staged changes appearing in the wrong worktree
- "fatal: not a git repository" errors in worktrees

**Prevention strategy:**
- Always set `GIT_DIR` and `GIT_WORK_TREE` (or use `git -C <worktree-path>`) when running git commands for a specific worktree
- In git library bindings, explicitly open the worktree path, never the main repo path
- Test that operations in one worktree never affect another worktree's state
- Wrap all git operations in a context that enforces the correct working directory

**Phase:** v0.1.0 core — fundamental to correct operation.

---

### 4.2 Submodule and Config Interactions

**What goes wrong:** Worktrees share `.git/config` but each has its own checked-out content. If the repo uses submodules, each worktree needs its own submodule checkout, but submodule config is shared. An agent running `git submodule update` in one worktree can interfere with another. Similarly, `.gitattributes` (merge drivers, filters) are per-worktree, but misconfigurations propagate.

**Warning signs:**
- Submodule state diverging between worktrees
- Custom merge drivers not being invoked in worktrees
- `.gitattributes` filters producing different results per worktree

**Prevention strategy:**
- For v0.1.0, document that submodule-heavy repos may have issues and defer full support
- If submodules are needed, run `git submodule update --init` per worktree after creation
- Test with repos that use `.gitattributes` merge drivers and LFS

**Phase:** v0.1.0 (document limitation); v0.2+ (full submodule support).

---

### 4.3 Worktree-Aware Hooks

**What goes wrong:** Git hooks (pre-commit, post-merge, etc.) are shared across worktrees via `.git/hooks/` (or the main repo's hooks dir). A hook that assumes it is running in the main repo (e.g., using hardcoded paths) will break in worktrees. Conversely, hooks that agents trigger (pre-commit, pre-push) may interfere with Smelt's orchestration if they modify files or reject commits.

**Warning signs:**
- Hooks failing with path-not-found errors in worktrees
- Agent commits being rejected by hooks that Smelt does not expect
- Hooks modifying files in the wrong worktree

**Prevention strategy:**
- Disable hooks in agent worktrees if they are not needed (`git config core.hooksPath /dev/null` per worktree, or `--no-verify` for specific operations)
- If hooks are needed, ensure they use `$(git rev-parse --show-toplevel)` and not hardcoded paths
- Test the hook behavior in worktree contexts during orchestrator setup

**Phase:** v0.1.0 (disable hooks or verify compatibility).

---

## 5. Orchestration Architecture

### 5.1 Polling vs. Event-Driven Agent Monitoring

**What goes wrong:** If Smelt polls agent processes for completion (checking if PID is alive, reading output files), it either polls too frequently (wasting CPU) or too infrequently (adding latency between agent completion and merge). If it uses event-driven approaches (waitpid, inotify), edge cases around signal delivery and file system events cause hangs.

**Warning signs:**
- Significant delay between agent completion and merge starting
- High CPU usage from polling loops
- Missed completion events causing indefinite waits

**Prevention strategy:**
- Use `waitpid()` / `wait4()` (or the language equivalent) as the primary completion signal — it is the most reliable
- Supplement with periodic health checks (is the PID still alive? is the process still writing output?)
- Set a hard timeout per session as a backstop against hangs
- Avoid filesystem-based signaling (creating marker files) — it is fragile across platforms

**Phase:** v0.1.0 core.

---

### 5.2 State Recovery After Partial Failure

**What goes wrong:** If Smelt crashes after some agents have completed but before merge, the completed work is stranded in worktree branches. If it crashes mid-merge, the target branch may be in a half-merged state. Without recovery logic, the operator must manually figure out what happened and clean up.

**Warning signs:**
- Operator has to manually inspect worktree branches after a crash
- Merged branch has a subset of expected changes
- Re-running Smelt after crash duplicates work or creates conflicting branches

**Prevention strategy:**
- Write orchestration state to a journal file before each phase transition (e.g., "agents started", "agent A completed", "merge started", "merge A into target done")
- On startup, read the journal and resume from the last successful checkpoint
- Make merge operations idempotent: if a branch is already merged into the target, skip it
- Design the merge target branch as append-only (each agent merge is a commit) so partial progress is visible in git log

**Phase:** v0.1.0 (basic journal); v0.2+ (full recovery).

---

### 5.3 Simulated Sessions Diverge From Real Behavior

**What goes wrong:** Smelt supports both real Claude Code sessions and simulated/scripted sessions for testing. If the simulated sessions do not faithfully replicate real agent behavior (timing, file patterns, git operations, error modes), tests pass but real usage fails. Common divergences: simulated sessions commit atomically (one commit), real agents make many incremental commits; simulated sessions never produce merge conflicts; simulated sessions always succeed.

**Warning signs:**
- All tests pass with simulated sessions, but real agent sessions fail
- Merge logic never exercises conflict resolution in tests
- Timing-dependent bugs only appearing with real agents

**Prevention strategy:**
- Design simulated sessions to be configurable: number of commits, delay between commits, which files to modify, whether to produce conflicts
- Include "adversarial" simulated sessions that intentionally create merge conflicts, make overlapping changes, and fail mid-session
- Run integration tests with real Claude Code sessions (even if expensive) before each release
- Document the known divergences between simulated and real sessions

**Phase:** v0.1.0 core — the simulated session design determines test quality.

---

### 5.4 Task Decomposition Determines Success

**What goes wrong:** The orchestrator is only as good as the task decomposition. If two agents are assigned overlapping work (modify the same files, implement conflicting approaches to the same feature), merging becomes impossible regardless of tooling. This is not a Smelt bug, but Smelt gets the blame.

**Warning signs:**
- High conflict rate between agent branches
- Agents undoing each other's changes
- Merged result failing tests that each individual branch passes

**Prevention strategy:**
- For v0.1.0, document that task decomposition is the user's responsibility and provide guidelines (e.g., "assign agents to different modules/directories")
- Log conflict frequency per file/directory to surface hot spots
- In future milestones, use file-level dependency analysis to warn about overlapping assignments before agents start

**Phase:** v0.1.0 (documentation + conflict logging); v0.2+ (overlap detection).

---

## 6. Platform and Environment

### 6.1 macOS-Specific File System Behavior

**What goes wrong:** macOS uses a case-insensitive filesystem by default (APFS case-insensitive). Two agents creating files that differ only in case (`README.md` vs `readme.md`) will silently collide. Additionally, macOS Finder and Spotlight may index worktree directories, causing I/O contention and `.DS_Store` file pollution.

**Warning signs:**
- Files silently overwriting each other due to case differences
- `.DS_Store` files appearing in commits
- Unexpectedly slow git operations due to Spotlight indexing

**Prevention strategy:**
- Create worktrees in a directory excluded from Spotlight indexing (add to System Settings > Spotlight > Privacy)
- Add `.DS_Store` to `.gitignore`
- Document the case-insensitivity risk; consider testing on case-sensitive volumes
- Normalize file paths to lowercase in conflict detection

**Phase:** v0.1.0 (documentation + .gitignore); ongoing awareness.

---

### 6.2 Disk Space Exhaustion

**What goes wrong:** Each worktree is a full checkout of the repository (minus `.git` objects, which are shared). For large repos, N worktrees = N * (repo size minus .git). If agents generate build artifacts, node_modules, or other large files in their worktrees, disk usage can spike rapidly.

**Warning signs:**
- Disk usage growing linearly with agent count
- "No space left on device" errors during agent sessions or merge
- Build artifacts accumulating in worktrees

**Prevention strategy:**
- Check available disk space before creating worktrees; abort with a clear error if insufficient
- Use sparse checkout for worktrees if agents only need a subset of the repo
- Clean up worktrees promptly after merge (do not leave them around "just in case")
- Add build artifact directories to `.gitignore` and document that agents should not commit artifacts

**Phase:** v0.1.0 (disk space check + prompt cleanup).

---

## Summary Matrix

| # | Pitfall | Severity | Phase |
|---|---------|----------|-------|
| 1.1 | Orphaned worktrees after crash | High | v0.1.0 |
| 1.2 | Worktree lock files block operations | Medium | v0.1.0 |
| 1.3 | Branch name collisions | High | v0.1.0 |
| 1.4 | Shared objects / gc contention | High | v0.1.0 |
| 2.1 | Zombie agent processes | Critical | v0.1.0 |
| 2.2 | Resource exhaustion | Medium | v0.1.0 |
| 2.3 | Agent output capture | Medium | v0.1.0 |
| 2.4 | Signal handling chain | High | v0.1.0 |
| 3.1 | Semantic conflicts merge cleanly | Critical | v0.1.0 |
| 3.2 | Merge order affects outcome | High | v0.1.0 |
| 3.3 | AI conflict resolution introduces bugs | High | v0.1.0 |
| 3.4 | Ref races during concurrent ops | High | v0.1.0 |
| 3.5 | Large diffs overwhelm AI resolution | Medium | v0.1.0 |
| 4.1 | HEAD/index state corruption | Critical | v0.1.0 |
| 4.2 | Submodule and config interactions | Low | v0.2+ |
| 4.3 | Worktree-aware hooks | Medium | v0.1.0 |
| 5.1 | Polling vs event-driven monitoring | Medium | v0.1.0 |
| 5.2 | State recovery after partial failure | High | v0.1.0 |
| 5.3 | Simulated sessions diverge from real | High | v0.1.0 |
| 5.4 | Task decomposition determines success | Medium | v0.1.0 |
| 6.1 | macOS filesystem behavior | Low | v0.1.0 |
| 6.2 | Disk space exhaustion | Medium | v0.1.0 |

---

*Researched: 2026-03-09 | Scope: Smelt v0.1.0 Orchestration PoC*
