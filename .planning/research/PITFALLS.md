# Pitfalls Research: v0.3.0 Features

**Date:** 2026-03-08
**Scope:** Common mistakes when adding worktree management, agent launching, session tracking, independent evaluation, diff assembly, and TUI to the existing Assay v0.2.0 codebase (23,385 lines, 493 tests, 5 crates).

**Supersedes:** v0.2.0 PITFALLS.md (all v0.2.0 pitfalls P-21 through P-40 have been mitigated in the shipped codebase).

---

## 1. Git Worktree Pitfalls

### P-41: Worktree cleanup failure leaves orphaned directories and git references

**Area:** Worktree Management
**Confidence:** High (well-documented git behavior)

**What goes wrong:** `git worktree add` creates both a filesystem directory and a reference in `.git/worktrees/`. If Assay creates a worktree for an agent session and the cleanup path is interrupted (process crash, SIGKILL, OOM), the worktree directory may be deleted while the git reference persists (or vice versa). This leaves the repo in a state where `git worktree list` shows a phantom worktree, and attempting to create a new worktree at the same path fails with "already registered". The inverse (directory exists, ref gone) silently corrupts any git operations run inside the orphan directory.

The existing guard daemon (`crates/assay-core/src/guard/daemon.rs`) already handles graceful shutdown with PID cleanup, but worktree cleanup is fundamentally harder because it involves both filesystem and git state.

**Warning signs:**
- `git worktree list` shows entries with `(prunable)` annotation
- Worktree creation fails with "already locked" or "already registered"
- Tests leave `.git/worktrees/` entries across runs
- Disk usage grows unexpectedly in CI environments

**Prevention:**
- Implement a two-phase cleanup: remove the git ref first (`git worktree remove --force`), then verify the directory is gone. Never delete the directory directly with `rm -rf`.
- Add a startup reconciliation pass: on Assay init or session start, run `git worktree prune` to clean orphans
- Use `git worktree lock` during active agent sessions to prevent accidental removal by other tooling
- Register worktree paths in a manifest file (`.assay/worktrees.json`) so Assay can enumerate and clean up independent of git state
- Write cleanup as an RAII-style guard (Rust `Drop` impl) that shells out to `git worktree remove`
- Test with `SIGKILL` scenarios, not just graceful shutdown

**Which phase should address it:** The first phase that introduces worktree creation (likely the worktree management phase).

---

### P-42: Path confusion between parent repo and worktree CWD

**Area:** Worktree Management / Path Threading
**Confidence:** High (verified against existing codebase patterns)

**What goes wrong:** The existing codebase uses `working_dir` extensively — `evaluate_command()` in `gate/mod.rs` receives an explicit `working_dir: &Path`, the guard daemon stores `session_path` and `assay_dir` as separate `PathBuf` fields, and checkpoint extraction calls `std::env::current_dir()`. When agents run in worktrees, there are now *three* relevant directories: the parent repo root, the worktree directory, and the `.assay/` directory (which lives in the parent repo, not the worktree).

Specific failure modes:
1. Agent runs `cargo test` in the worktree but gate evaluation resolves `working_dir` to the parent repo
2. `.assay/specs/` lookup resolves relative to the worktree (which has no `.assay/`)
3. `std::env::current_dir()` in `try_save_checkpoint()` returns the wrong directory if the daemon was spawned from a worktree
4. Relative paths in spec files (e.g., `path = "src/auth.rs"`) resolve against the wrong root

**Warning signs:**
- "File not found" errors for files that clearly exist (but in the other directory)
- Gate results that pass in one context but fail in another
- Checkpoint extraction returning empty agent state

**Prevention:**
- Introduce a `WorkContext` struct that bundles `repo_root`, `worktree_root`, and `assay_dir` explicitly — pass it through instead of raw `Path` arguments
- Never use `std::env::current_dir()` — always derive paths from the `WorkContext`. Audit the existing `try_save_checkpoint()` call in `daemon.rs` (line 304) which already uses `current_dir()`
- Gate evaluation should receive the worktree path as `working_dir`, not the parent repo
- Spec/config resolution should always use `assay_dir` (parent repo), never the worktree
- Add integration tests that run gates from a worktree and verify paths resolve correctly

**Which phase should address it:** Must be solved in the worktree management phase, before agent launching depends on it.

---

### P-43: Nested worktree or submodule interactions

**Area:** Worktree Management
**Confidence:** Medium (edge case, but common in real-world repos)

**What goes wrong:** If the parent repo contains git submodules, `git worktree add` does not automatically initialize submodules in the worktree. Agents running `cargo build` or `cargo test` in the worktree will get missing dependency errors. Additionally, if a user accidentally runs `assay` inside a worktree that Assay itself created (creating worktrees-of-worktrees), git's behavior becomes unpredictable.

**Warning signs:**
- Build failures in worktrees that succeed in the parent repo
- "not a git repository" errors inside worktree subdirectories
- `.gitmodules` references that don't resolve in the worktree

**Prevention:**
- After `git worktree add`, run `git submodule update --init --recursive` in the worktree
- Detect whether the current repo is itself a worktree (`git rev-parse --git-common-dir` differs from `--git-dir`) and refuse to create nested worktrees
- Document that worktrees share the object store and ref namespace — concurrent agents pushing to the same branch will conflict

**Which phase should address it:** Worktree management phase, as a validation step after creation.

---

### P-44: Worktree branch conflicts with concurrent agents

**Area:** Worktree Management
**Confidence:** High (git fundamental constraint)

**What goes wrong:** Git does not allow two worktrees to have the same branch checked out simultaneously. If Agent A is working on branch `feature/auth` in worktree A, and Agent B tries to check out the same branch in worktree B, git will refuse. This is a fundamental git constraint, not a bug. But if Assay doesn't account for it, launching multiple agents against the same spec will fail with cryptic git errors.

**Warning signs:**
- `git worktree add` fails with "branch is already checked out"
- Agent sessions fail to start but the error message doesn't surface clearly

**Prevention:**
- Create a unique detached branch per worktree (e.g., `assay/agent-<session-id>`)
- Track which branches are in use via the worktree manifest
- When creating worktrees from a base branch, use `git worktree add -b <unique-name> <path> <base-branch>`

**Which phase should address it:** Worktree management phase.

---

## 2. Subprocess Management Pitfalls

### P-45: Zombie processes from agent subprocesses

**Area:** Agent Launching / Subprocess Management
**Confidence:** High (the existing `evaluate_command()` already handles this for gate commands, but agent processes are long-lived)

**What goes wrong:** Gate command execution (`gate/mod.rs:433-616`) already handles subprocess lifecycle well: process groups, SIGKILL on timeout, zombie reaping via `child.wait()`. However, agent processes (Claude Code sessions) are fundamentally different — they are long-lived, interactive, and may themselves spawn subprocesses (cargo, npm, etc.). If Assay spawns an agent and the agent's child processes outlive the agent (e.g., a background server started by the agent), those grandchild processes become zombies or orphans.

The existing `command.process_group(0)` pattern (line 447) sends SIGKILL to the entire process group on timeout, but this assumes the group is still intact. Agents that `setsid()` or background processes will escape the group.

**Warning signs:**
- `ps aux | grep defunct` shows zombie processes after agent sessions
- Port conflicts from orphaned servers started by agents
- System resource exhaustion in long-running CI environments

**Prevention:**
- Use cgroups (Linux) or process groups (macOS) to contain the entire agent subtree
- On session cleanup, enumerate and kill the entire process tree, not just the direct child
- Set a hard timeout on agent sessions with `SIGKILL` escalation: `SIGTERM` → wait 5s → `SIGKILL`
- Track spawned agent PIDs in the session manifest so they can be cleaned up on Assay restart
- Consider using a PID namespace (Linux containers) for complete isolation

**Which phase should address it:** Agent launching phase.

---

### P-46: stdout/stderr buffering causes incomplete output capture from agents

**Area:** Agent Launching / Output Capture
**Confidence:** Medium (depends on agent implementation)

**What goes wrong:** The existing gate evaluation uses reader threads to drain stdout/stderr pipes concurrently (lines 465-482 in `gate/mod.rs`). This works for short-lived commands. For long-lived agent sessions, buffering behavior changes: most programs line-buffer when connected to a TTY but block-buffer when connected to a pipe. Agent output may appear to stall or arrive in large chunks. If the agent process dies while a block-buffer is partially filled, the last output is lost.

Additionally, if Assay captures agent output into a `Vec<u8>` in memory (as the current `read_to_end` approach does), a verbose agent session could consume gigabytes of RAM.

**Warning signs:**
- Agent output appears in bursts rather than streaming
- Last few lines of agent output missing after crash
- Memory usage grows linearly with agent session duration

**Prevention:**
- For long-lived agents, stream output to a log file rather than buffering in memory
- Use `pty` (pseudo-terminal) allocation if real-time line-buffered output is needed
- Set output size limits per session with rotation (similar to the existing `MAX_OUTPUT_BYTES` constant)
- For the TUI real-time display, use a ring buffer of the last N lines rather than accumulating everything

**Which phase should address it:** Agent launching phase, with TUI integration in the TUI phase.

---

### P-47: Signal forwarding to agent processes

**Area:** Agent Launching
**Confidence:** High (Unix process model)

**What goes wrong:** When the user sends SIGINT (Ctrl-C) to Assay, the existing guard daemon handles this cleanly (lines 105-108 in `daemon.rs`). But for the orchestrator that manages multiple agents, SIGINT should be forwarded to all agent processes so they can save their state. If Assay catches SIGINT and exits without forwarding, agents become orphaned. If Assay ignores SIGINT and only forwards, the user can't cancel.

**Warning signs:**
- Ctrl-C kills Assay but agents keep running
- Ctrl-C kills everything immediately without agent cleanup
- Double Ctrl-C required to exit

**Prevention:**
- First SIGINT: signal all agents with SIGTERM, display "shutting down..." in TUI
- Second SIGINT (within 3s): SIGKILL all agents and exit immediately
- Use the existing pattern from `daemon.rs` but extend it to manage multiple process handles
- Register agent process handles in a shared `Vec<Child>` protected by a mutex

**Which phase should address it:** Agent launching phase, refined in TUI phase.

---

### P-48: PID reuse causing wrong-process signals

**Area:** Subprocess Management / Session Tracking
**Confidence:** Medium (race window is small but real)

**What goes wrong:** The existing PID management in `guard/pid.rs` checks liveness via `kill(pid, 0)`. But PID reuse on modern systems (especially macOS with its small PID space) means a stored PID could be reused by an unrelated process between the time the agent dies and the time Assay checks it. Sending SIGKILL to the wrong process is catastrophic.

The existing code (line 82-95 in `pid.rs`) already guards against PID 0 and negative values, but doesn't guard against reuse.

**Warning signs:**
- Random processes dying when Assay cleans up sessions
- "Operation not permitted" errors when trying to signal agent processes (the new process belongs to a different user)

**Prevention:**
- Store `(pid, start_time)` tuples rather than bare PIDs. Verify process start time matches before signaling.
- On macOS, use `proc_pidinfo()` to verify the process is the expected one
- On Linux, use `/proc/<pid>/stat` start time or `pidfd_open()` for race-free signaling
- Prefer `pidfd` (Linux 5.3+) or `kqueue` `EVFILT_PROC` (macOS) for race-free process monitoring
- As a defense-in-depth, always check that the process command line matches expectations before sending signals

**Which phase should address it:** Agent launching phase.

---

## 3. Session Tracking Pitfalls

### P-49: Stale session state after crash prevents new sessions

**Area:** Session Tracking
**Confidence:** High (directly observed in existing AgentSession design)

**What goes wrong:** The current `AgentSession` type (`assay-types/src/session.rs`) is designed for crash recovery — it tracks in-progress evaluations. But sessions are currently in-memory only (created in `gate/session.rs:create_session`, no persistence between process restarts). When v0.3.0 adds persistent sessions for long-lived agent work, a crash leaves a session in "active" state with no process backing it. New session creation may refuse if there's a "limit 1 active session per spec" rule, and the user has no way to recover without manually editing state files.

**Warning signs:**
- "Session already active" errors after a crash
- Users manually deleting `.assay/sessions/` files to recover
- Accumulation of orphaned session files

**Prevention:**
- Add a `status` field to persisted sessions: `active`, `completed`, `abandoned`, `crashed`
- On startup, scan for `active` sessions with no live process (using PID tracking from P-48) and transition them to `crashed`
- Provide an explicit `assay session recover <id>` command that transitions crashed sessions
- Set a TTL on active sessions — auto-abandon after configurable duration (e.g., 24h)
- Use advisory file locks (`flock`) on session files to detect whether the owning process is alive

**Which phase should address it:** Session tracking phase.

---

### P-50: Concurrent session access corrupts state

**Area:** Session Tracking
**Confidence:** High (same class as v0.2.0's P-22 but for sessions instead of results)

**What goes wrong:** Multiple processes may access the same session state: the MCP server (handling `gate_report` calls from the coding agent), the orchestrator (monitoring progress), the TUI (displaying status), and the independent evaluator (writing evaluation results). Without synchronization, concurrent writes to the session file produce corrupted JSON.

The existing `finalize_session()` in `gate/session.rs` takes an immutable reference to `AgentSession`, which is safe in-process. But when sessions are persisted to disk and accessed by multiple processes, the in-process safety doesn't help.

**Warning signs:**
- Truncated or malformed JSON in session files
- Evaluations silently lost (overwritten by a concurrent read-modify-write)
- Non-deterministic test failures in parallel test runs

**Prevention:**
- Use append-only JSONL for session events (not mutable JSON) — each evaluation is a new line, finalization reads and reduces
- If mutable state is needed, use `flock()` advisory locks with `LOCK_EX` for writes and `LOCK_SH` for reads
- Consider SQLite for session storage (single-writer, multiple-reader with WAL mode)
- For inter-process communication between orchestrator/evaluator/agent, prefer a Unix domain socket or named pipe over shared file mutation
- Write deterministic tests that simulate concurrent access using threads

**Which phase should address it:** Session tracking phase.

---

### P-51: Schema evolution breaks session deserialization

**Area:** Session Tracking / Schema Evolution
**Confidence:** High (directly follows from v0.2.0's P-21 on `deny_unknown_fields`)

**What goes wrong:** As sessions become persistent, their schema must evolve across Assay versions. Adding a field to `AgentSession` (e.g., `worktree_path`, `agent_pid`) means that session files written by v0.3.1 cannot be read by v0.3.0 if `deny_unknown_fields` is active. Unlike specs and configs (which are human-authored and rarely downgraded), session files are machine-generated and may persist across version upgrades.

**Warning signs:**
- "Unknown field" deserialization errors after upgrading Assay
- Session recovery fails after upgrade
- Users lose in-progress work during routine updates

**Prevention:**
- Do NOT use `deny_unknown_fields` on session types (already absent on `AgentSession` — verify this stays true)
- Add a `schema_version: u32` field to the session format, defaulting to 1
- Write migration functions that upgrade session format from version N to N+1
- Use `#[serde(default)]` for all new optional fields so old sessions deserialize cleanly
- Add roundtrip tests: serialize with current version, deserialize with "old" schema (by removing fields)

**Which phase should address it:** Session tracking phase.

---

## 4. Diff Assembly Pitfalls

### P-52: Binary files in diffs crash or produce garbage

**Area:** Diff Assembly
**Confidence:** High (common git diff pitfall)

**What goes wrong:** When assembling diffs for independent evaluation, `git diff` may encounter binary files (images, compiled artifacts, SQLite databases). Raw binary content embedded in a diff produces invalid UTF-8, and `String::from_utf8_lossy()` will replace bytes, producing misleading diff content. Worse, large binary files (node_modules accidentally committed, ML model weights) can exhaust memory or blow the context window for AI evaluation.

**Warning signs:**
- Replacement characters (U+FFFD) appearing in diff output
- OOM kills when generating diffs on repos with binary artifacts
- AI evaluator confused by binary garbage in the diff

**Prevention:**
- Use `git diff --no-binary` or `git diff --stat` to detect binary files before including content
- For binary files, include only the path and size delta (e.g., "Binary file logo.png changed: 45KB → 52KB")
- Set a per-file size limit (e.g., 100KB) — truncate with a marker
- Use `.gitattributes` awareness to respect project's binary file declarations
- Validate diff output as valid UTF-8 before passing to evaluators

**Which phase should address it:** Diff assembly phase.

---

### P-53: Large diffs exceed AI context window limits

**Area:** Diff Assembly / Independent Evaluation
**Confidence:** High (fundamental constraint of LLM evaluation)

**What goes wrong:** An agent working on a large feature may produce a diff spanning thousands of lines. The independent evaluator (itself an AI) has a finite context window. If the diff + spec + evaluation prompt exceeds the window, the evaluator either truncates (losing important context) or fails entirely. The existing `MAX_OUTPUT_BYTES` (64KB) in `gate/mod.rs` provides a precedent for truncation, but diff assembly needs its own budget accounting.

The existing `context/tokens.rs` module estimates tokens from bytes — reuse this for diff budget calculation.

**Warning signs:**
- Evaluator returns low-confidence results on large diffs
- Evaluator misses issues in files that were truncated away
- Inconsistent evaluation results depending on diff order

**Prevention:**
- Budget the context window explicitly: spec (N tokens) + diff (M tokens) + prompt template (K tokens) = total
- Implement diff prioritization: changed test files first, then implementation files, then config/boilerplate last
- Support multi-pass evaluation: split large diffs into chunks, evaluate each, then synthesize
- Use `git diff --stat` as a summary fallback when full diff exceeds budget
- Reuse the existing `estimate_tokens_from_bytes()` from `context/tokens.rs` for budget calculation
- Allow the spec to declare which files/patterns are most important for evaluation

**Which phase should address it:** Diff assembly phase, with evaluator integration in independent evaluation phase.

---

### P-54: Rename detection produces confusing diffs

**Area:** Diff Assembly
**Confidence:** Medium (git-specific behavior)

**What goes wrong:** `git diff` with rename detection (`-M`) shows renames as a delete+add pair or as a rename with partial diff. Without `-M`, moving a file looks like a complete deletion and a complete addition, doubling the diff size and confusing evaluators. With `-M` and a low threshold, unrelated files with similar content may be falsely detected as renames.

**Warning signs:**
- AI evaluator reports "entire file was rewritten" when it was just moved
- Diff sizes unexpectedly double for refactoring commits
- False rename detection between generated files (e.g., two similar test fixtures)

**Prevention:**
- Use `git diff -M50%` (50% similarity threshold) as a reasonable default
- Include a rename summary header before the diff content so evaluators understand the context
- Allow per-spec override of rename detection threshold
- Test with a known rename scenario and verify the diff output is evaluator-friendly

**Which phase should address it:** Diff assembly phase.

---

### P-55: Diff computed against wrong base

**Area:** Diff Assembly
**Confidence:** High (critical correctness issue)

**What goes wrong:** The diff should represent "what the agent changed" — the delta between the worktree's starting state and its current state. But `git diff` computes against HEAD by default, which may have moved if the agent made commits. The correct base is the commit the worktree was created from (the "merge base"). If the parent branch advanced while the agent was working, `git diff main..agent-branch` includes changes from both the agent and the parent branch.

**Warning signs:**
- Evaluator sees changes the agent didn't make
- Diff includes merge conflict markers
- Evaluation results change depending on when the diff is computed

**Prevention:**
- Record the base commit SHA when creating the worktree — store it in the session manifest
- Use `git diff <base-sha>..HEAD` rather than `git diff main..HEAD`
- For uncommitted agent work, use `git diff <base-sha>` (working tree diff against base)
- Validate that the base commit is an ancestor of HEAD before computing the diff

**Which phase should address it:** Diff assembly phase, with base SHA tracking in worktree management phase.

---

## 5. TUI Pitfalls

### P-56: Terminal state corruption on unexpected exit

**Area:** TUI
**Confidence:** High (verified in existing TUI code)

**What goes wrong:** The current TUI (`assay-tui/src/main.rs`) already handles this partially: it installs a panic hook that calls `ratatui::restore()` (line 14) and restores on normal exit (line 21). However, `SIGKILL` (uncatchable), `SIGTERM` (not currently handled), and `SIGABRT` (e.g., from a failed assertion) bypass both the panic hook and the normal exit path. The terminal is left in raw mode: no echo, no line buffering, cursor hidden. The user must run `reset` manually.

When v0.3.0 adds agent management to the TUI, the risk increases because agent failures can trigger panics in the event handling code.

**Warning signs:**
- Terminal unresponsive after Assay crash
- No cursor visible after exit
- Keyboard input invisible (raw mode persists)
- User reports needing `stty sane` or `reset` after using Assay

**Prevention:**
- Add `SIGTERM` and `SIGHUP` handlers (using `tokio::signal` or `signal-hook`) that restore terminal state
- Use `scopeguard` or a RAII wrapper to ensure `ratatui::restore()` runs on all exit paths except SIGKILL
- Write terminal state to a recovery file (`.assay/tui-state`) so a subsequent launch can detect and fix a dirty terminal
- Test with `kill -TERM <pid>` and `kill -HUP <pid>`, not just Ctrl-C
- Consider running the TUI in an alternate screen buffer (already implicit with ratatui) — exiting alternate screen restores the original terminal state

**Which phase should address it:** TUI phase.

---

### P-57: TUI resize handling causes panics or rendering corruption

**Area:** TUI
**Confidence:** Medium (ratatui handles most cases, but layout math can panic)

**What goes wrong:** When the terminal window is resized, ratatui receives a `Resize` event and redraws. If the layout code uses hardcoded constraints (e.g., `Constraint::Length(10)`) that don't fit in a very small terminal (fewer than 10 rows), the layout arithmetic can produce zero-size or negative-size areas. Ratatui's `Layout::split` handles this gracefully, but custom rendering code that indexes into areas by position may panic on `index out of bounds`.

This is especially relevant for v0.3.0's TUI which will display multiple panes (agent status, logs, gate results).

**Warning signs:**
- Panic when resizing to very small window
- Rendering artifacts (overlapping text, missing borders)
- TUI becomes unresponsive after resize

**Prevention:**
- Define a minimum terminal size (e.g., 80x24) and display a "terminal too small" message below it
- Use `Constraint::Min(n)` and `Constraint::Percentage(p)` instead of `Constraint::Length(n)` for primary layout
- Test rendering at extreme sizes: 1x1, 10x3, 300x100
- Use ratatui's `Frame::area()` to check bounds before rendering optional panels

**Which phase should address it:** TUI phase.

---

### P-58: TUI event loop blocks on synchronous operations

**Area:** TUI
**Confidence:** High (architectural constraint)

**What goes wrong:** The TUI event loop must remain responsive (target: 60fps, ~16ms per frame). If the event loop directly calls synchronous operations — gate evaluation (which can take minutes), session file I/O, git commands for diff assembly — the TUI freezes. The user can't scroll, resize, or cancel.

The existing gate evaluation is explicitly documented as synchronous (see `gate/mod.rs` doc comments recommending `spawn_blocking`), and the TUI will need to orchestrate multiple agents while remaining responsive.

**Warning signs:**
- TUI freezes during gate runs
- Keyboard input queued but not processed until operation completes
- Resize events "catch up" in a burst after unblock

**Prevention:**
- Run all blocking operations on a `tokio::task::spawn_blocking` thread or a dedicated thread pool
- Use channels (`tokio::sync::mpsc` or `std::sync::mpsc`) to send state updates from background tasks to the TUI render loop
- Design TUI state as a read-only snapshot that the render loop consumes, updated by background tasks via message passing
- Use `crossterm::event::poll(Duration::from_millis(16))` for non-blocking event polling (the current `event::read()` on line 34 is blocking)
- Consider an `App` struct that owns both the TUI state and the channel receivers, updated each frame

**Which phase should address it:** TUI phase.

---

### P-59: TUI testing requires headless terminal emulation

**Area:** TUI / Testing
**Confidence:** High (well-known TUI testing challenge)

**What goes wrong:** The existing 493 tests are all unit or integration tests that don't require a terminal. TUI code is notoriously hard to test: rendering output goes to a terminal buffer, keyboard input comes from stdin, and the event loop is stateful. Without test infrastructure, TUI code accumulates untested rendering logic.

**Warning signs:**
- TUI bugs discovered only through manual testing
- Rendering regressions after refactoring
- No TUI test coverage in CI

**Prevention:**
- Separate TUI into layers: pure state machine (fully testable) + rendering (snapshot-testable) + event handling (integration-testable)
- Use `ratatui::backend::TestBackend` for snapshot tests — render to a buffer, assert on cell contents
- Keep the state machine pure: `fn update(state: State, event: Event) -> State` is trivially testable
- Use `insta` crate for snapshot testing of rendered frames
- Limit the rendering layer to a thin `fn view(state: &State, frame: &mut Frame)` that's called from both production code and tests

**Which phase should address it:** TUI phase.

---

## 6. Independent Evaluation Pitfalls

### P-60: Evaluator-agent collusion when using same model/provider

**Area:** Independent Evaluation
**Confidence:** Medium (AI evaluation reliability concern)

**What goes wrong:** The `EvaluatorRole::Independent` variant (already defined in `assay-types/src/session.rs:28`) is designed for a separate agent evaluating work. But if the independent evaluator uses the same LLM model and provider as the coding agent, it may share the same biases: overly generous evaluation of LLM-generated code, consistent blind spots for the same categories of bugs, and "model agreement" that masquerades as independent verification.

**Warning signs:**
- Independent evaluator consistently agrees with self-eval (>95% concordance)
- Known bugs slip through both self-eval and independent eval
- Evaluation confidence is always "high" regardless of actual quality

**Prevention:**
- Allow configuring different models/providers for the evaluator vs. the coding agent
- Track concordance rate between self-eval and independent eval — alert if it exceeds a threshold
- Include adversarial prompting in the evaluator's system prompt (explicitly ask it to look for problems)
- Provide the evaluator with the spec criteria but NOT the agent's self-eval reasoning (avoid anchoring)
- Consider a "red team" prompt variant that specifically tries to find issues

**Which phase should address it:** Independent evaluation phase.

---

### P-61: Evaluator sees agent's self-eval and anchors on it

**Area:** Independent Evaluation
**Confidence:** High (well-documented cognitive bias, applies to LLMs too)

**What goes wrong:** The existing `resolve_evaluator_priority()` in `gate/session.rs:77-93` correctly selects the highest-priority evaluation. But if the independent evaluator's *input* includes the coding agent's self-eval results (evidence, reasoning), the evaluator anchors on those conclusions rather than performing a truly independent review. This defeats the purpose of independent evaluation.

**Warning signs:**
- Independent evaluator uses similar phrasing to the self-eval
- Independent evaluator references the self-eval's evidence rather than gathering its own
- Evaluation time is suspiciously short (evaluator rubber-stamps the self-eval)

**Prevention:**
- Structure the evaluator's input to include ONLY: the spec, the diff, and the evaluation criteria
- Explicitly exclude the self-eval from the evaluator's context
- Run independent evaluation BEFORE self-eval results are finalized (or in a separate session that has no access to self-eval state)
- Include a "derive your own evidence" instruction in the evaluator prompt

**Which phase should address it:** Independent evaluation phase.

---

## 7. Integration Pitfalls

### P-62: Error type proliferation across new modules

**Area:** Error Handling / Integration
**Confidence:** High (verified against existing `AssayError` structure)

**What goes wrong:** The existing `AssayError` enum in `error.rs` already has 20 variants (from `Io` through `GuardCircuitBreakerTripped`), each with contextual fields. v0.3.0 adds at minimum: worktree errors (creation, cleanup, lock), agent errors (spawn, timeout, signal), session persistence errors (lock, corruption, migration), diff assembly errors (git failures, encoding), and evaluator errors (model unavailable, timeout, invalid response). Naively adding all of these produces a 40+ variant enum that's impossible to match exhaustively.

The existing `#[non_exhaustive]` on `AssayError` (line 13) already acknowledges this growth pattern, but the ergonomics degrade regardless.

**Warning signs:**
- Error messages become generic ("something went wrong in the worktree module")
- Match arms proliferate with identical error handling
- New code uses `anyhow` or `Box<dyn Error>` to avoid adding variants

**Prevention:**
- Group related errors into sub-enums: `WorktreeError`, `AgentError`, `DiffError`, then embed them as single variants in `AssayError` (e.g., `AssayError::Worktree(WorktreeError)`)
- Each sub-enum lives in its module (like `ConfigError` and `SpecError` already do)
- Keep `AssayError` as the public API boundary — internal code can use sub-enums
- Use `From` impls for ergonomic conversion (e.g., `impl From<WorktreeError> for AssayError`)
- Maintain the existing pattern of structured fields (operation, path, source) for diagnosability

**Which phase should address it:** First phase that adds new error variants. Ideally, establish the sub-enum pattern in the worktree phase and use it consistently in all subsequent phases.

---

### P-63: Feature coupling between worktree, session, and evaluator

**Area:** Architecture / Integration
**Confidence:** High (architectural analysis)

**What goes wrong:** Worktree management, session tracking, and independent evaluation are conceptually distinct but operationally coupled: creating a session requires a worktree, the evaluator needs the session's diff, cleaning up a session requires worktree removal, and the TUI displays all of it. If these features are developed as tightly coupled modules, changes to one cascade through all others. Test failures become impossible to isolate.

**Warning signs:**
- Changing the worktree creation logic breaks evaluator tests
- Session tests require setting up git repos with worktrees
- Circular dependencies between modules
- "God struct" that holds worktree + session + evaluator state

**Prevention:**
- Define clear interfaces between modules: worktree module exports `WorktreeHandle` (path + cleanup), session module accepts any `Path`, evaluator accepts `DiffContent` (a string, not a worktree reference)
- Use trait objects or generics at module boundaries to allow testing with mocks
- Build and test each module independently with its own test fixtures
- Integration tests that exercise the full pipeline live in a separate test crate or `tests/` directory
- Follow the existing pattern: `assay-types` for data, `assay-core` for logic, binaries for orchestration

**Which phase should address it:** Architecture design, before any implementation phases begin.

---

### P-64: CWD mutation leaks between tests or concurrent operations

**Area:** Testing / Integration
**Confidence:** High (existing code already avoids CWD mutation mostly, but new features may regress)

**What goes wrong:** The existing codebase passes explicit paths instead of relying on CWD — `evaluate_command()` takes `working_dir`, the guard daemon stores `session_path`. But `std::env::set_current_dir()` is process-global. If any new code changes CWD (e.g., to run git commands in a worktree), it affects all threads. In parallel test execution (`cargo test` runs tests in threads by default), CWD mutation in one test corrupts another.

The existing `try_save_checkpoint()` already uses `std::env::current_dir()` (daemon.rs line 304), which is read-only and safe. But worktree code that needs to `cd` into a worktree to run git commands is a regression risk.

**Warning signs:**
- Tests pass individually but fail when run together (`cargo test` vs `cargo test -- test_name`)
- "File not found" errors that depend on test execution order
- Flaky CI that passes on retry

**Prevention:**
- Never call `std::env::set_current_dir()` — always pass `--work-tree` and `--git-dir` to git commands, or use `Command::current_dir()`
- Audit all `std::env::current_dir()` calls — they should only appear in CLI argument resolution, never in library code
- Each test that needs a filesystem context should use `tempfile::TempDir` (already the pattern in existing tests)
- Run `cargo test` with `--test-threads=1` in CI as a canary for thread-safety issues, then fix the root cause

**Which phase should address it:** All phases. Establish as a lint rule or code review checkpoint.

---

### P-65: Tokio runtime mismatch between sync and async boundaries

**Area:** Integration / Runtime
**Confidence:** Medium (depends on architectural decisions)

**What goes wrong:** The existing codebase has a split personality: gate evaluation is synchronous (with explicit `spawn_blocking` guidance in doc comments), the guard daemon is async (tokio), and the MCP server is async. v0.3.0 adds more async work: agent process monitoring, TUI event loops, evaluator API calls. If synchronous code accidentally runs on the tokio runtime thread (e.g., `std::thread::sleep` or blocking file I/O inside an `async fn`), it blocks the entire runtime, stalling all concurrent operations.

**Warning signs:**
- TUI becomes unresponsive while agents are running
- MCP server stops responding to requests during gate evaluation
- "tokio runtime cannot start from within a tokio runtime" panic (nested runtime creation)

**Prevention:**
- Establish a clear async/sync boundary: `assay-core` remains sync, orchestration layer is async
- Use `tokio::task::spawn_blocking()` for all calls from async code into sync `assay-core` functions
- Never create a new tokio runtime inside existing async code — pass the runtime handle or use `Handle::current()`
- Add `#[deny(clippy::large_futures)]` to catch accidentally large futures (which often contain blocking operations)
- Consider using `tokio-console` during development to detect blocked tasks

**Which phase should address it:** Agent launching phase (first phase that requires significant async orchestration).

---

## Summary by Phase

| Phase | Pitfalls | Critical |
|-------|----------|----------|
| Worktree Management | P-41, P-42, P-43, P-44 | P-41, P-42 |
| Agent Launching | P-45, P-46, P-47, P-48, P-65 | P-45, P-47 |
| Session Tracking | P-49, P-50, P-51 | P-49, P-50 |
| Diff Assembly | P-52, P-53, P-54, P-55 | P-53, P-55 |
| Independent Evaluation | P-60, P-61 | P-61 |
| TUI | P-56, P-57, P-58, P-59 | P-56, P-58 |
| Cross-cutting | P-62, P-63, P-64 | P-62, P-63 |
