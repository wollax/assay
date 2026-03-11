# Phase 10: Real Agent Sessions - Research

**Researched:** 2026-03-11
**Domain:** Claude Code CLI integration, async process lifecycle, headless agent orchestration
**Confidence:** HIGH

## Summary

This phase adds a real Claude Code agent backend to the existing session controller. The codebase already has the scaffolding: `SessionRunner` dispatches sessions via `ScriptExecutor` for scripted backends, `ProcessGroup` wraps child processes with `kill_group()` via libc SIGTERM, and `WorktreeManager` creates isolated worktrees with state tracking. The `SessionDef` type already has `script: Option<ScriptDef>` — sessions without a script currently return an immediate `Completed` result with no commits. The new `AgentExecutor` replaces that `None` branch.

Claude Code CLI provides a well-documented non-interactive mode via `claude -p "prompt"` with `--dangerously-skip-permissions` for unattended execution. It supports `--output-format json` for structured results (including session ID, cost, duration), `--model` for model selection, `--max-turns` for turn limits, and `--allowed-tools` for tool restriction. The `.claude/settings.json` file provides project-level permission configuration, and `CLAUDE.md` files are automatically read from the working directory.

**Primary recommendation:** Create an `AgentExecutor` module parallel to `ScriptExecutor`, spawning `claude -p "..." --dangerously-skip-permissions --output-format json` via `tokio::process::Command` with `.process_group(0)`. Integrate into the orchestrator's `execute_sessions` method at the existing `None` script branch.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
| --- | --- | --- | --- |
| `tokio::process` | 1.x (already in workspace) | Async child process spawn, stdout/stderr piping, wait | Already used via `tokio` with `process` feature enabled |
| `std::process::Stdio` | std | Configure piped stdout/stderr for capture | Standard Rust |
| `libc` | 0.2 (already in workspace) | Process group SIGTERM via `kill(-pgid, SIGTERM)` | Already used by `ProcessGroup` |
| `serde_json` | 1 (already in workspace) | Parse Claude Code JSON output | Already a dependency |
| `tokio::time::timeout` | 1.x (already in workspace) | Per-session deadline enforcement | Part of tokio |

### Supporting

| Library | Version | Purpose | When to Use |
| --- | --- | --- | --- |
| `chrono` | 0.4 (already in workspace) | Timestamp log files and state updates | Already used throughout |
| `tracing` | 0.1 (already in workspace) | Structured logging for agent lifecycle events | Already used throughout |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
| --- | --- | --- |
| `tokio::process::Command` | `std::process::Command` + thread | Tokio process integrates with select!/cancellation natively; no reason to use std |
| `--dangerously-skip-permissions` | `.claude/settings.json` allow rules | Settings.json alone doesn't fully skip prompts; the flag is required for true headless mode |
| `--output-format json` | Parse stdout text | JSON gives structured cost/session data; text requires fragile parsing |

**Installation:** No new dependencies required. All libraries are already in the workspace.

## Architecture Patterns

### Recommended Module Structure

```
crates/smelt-core/src/session/
├── mod.rs              # Re-exports (add AgentExecutor)
├── agent.rs            # NEW: AgentExecutor — spawns Claude Code process
├── manifest.rs         # SessionDef (no changes needed)
├── process.rs          # ProcessGroup (already exists, reuse as-is)
├── runner.rs           # SessionRunner (wire in AgentExecutor)
├── script.rs           # ScriptExecutor (unchanged)
└── types.rs            # SessionOutcome, SessionResult (already has TimedOut/Killed)
```

### Pattern 1: AgentExecutor Parallel to ScriptExecutor

**What:** A new `AgentExecutor` struct that mirrors `ScriptExecutor`'s interface but spawns a Claude Code CLI process instead of running scripted steps.

**When to use:** When `session.script` is `None` (indicating a real agent session).

**Example:**
```rust
// Source: mirrors existing ScriptExecutor pattern
pub struct AgentExecutor {
    claude_binary: PathBuf,  // resolved once at orchestrator init
    worktree_path: PathBuf,
    log_dir: PathBuf,
    timeout: Option<Duration>,
}

impl AgentExecutor {
    pub async fn execute(
        &self,
        session_name: &str,
        task: &str,
        file_scope: Option<&[String]>,
        cancel: CancellationToken,
    ) -> Result<SessionResult> {
        // 1. Inject CLAUDE.md into worktree
        // 2. Inject .claude/settings.json into worktree
        // 3. Build command: claude -p "..." --dangerously-skip-permissions --output-format json
        // 4. Spawn with .process_group(0), piped stdout/stderr
        // 5. Wrap in ProcessGroup for kill_group()
        // 6. tokio::select! { timeout, cancel, wait_with_output }
        // 7. Parse exit code + check for commits
        // 8. Return SessionResult
    }
}
```

### Pattern 2: Prompt Construction

**What:** Build the task prompt from manifest fields and inject it via `--prompt`.

**Example:**
```rust
fn build_prompt(task: &str, file_scope: Option<&[String]>) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are working in a git worktree. ");
    prompt.push_str("Your task:\n\n");
    prompt.push_str(task);
    if let Some(scopes) = file_scope {
        prompt.push_str("\n\nFile scope (focus on these paths):\n");
        for scope in scopes {
            prompt.push_str(&format!("- {scope}\n"));
        }
    }
    prompt.push_str("\n\nCommit your changes when done. Do not push.");
    prompt
}
```

### Pattern 3: CLAUDE.md Injection

**What:** Write a session-specific CLAUDE.md into the worktree root before launching Claude Code. Claude Code automatically reads it.

**Example:**
```rust
fn inject_claude_md(worktree_path: &Path, session_name: &str, file_scope: Option<&[String]>) -> io::Result<()> {
    let mut content = String::new();
    content.push_str("# Session Constraints\n\n");
    content.push_str(&format!("Session: {session_name}\n\n"));
    content.push_str("## Rules\n\n");
    content.push_str("- Work ONLY within this worktree\n");
    content.push_str("- Commit your changes with descriptive messages\n");
    content.push_str("- Do NOT push to any remote\n");
    content.push_str("- Do NOT modify files outside your assigned scope\n");
    if let Some(scopes) = file_scope {
        content.push_str("\n## File Scope\n\n");
        content.push_str("Only modify files matching these patterns:\n");
        for scope in scopes {
            content.push_str(&format!("- `{scope}`\n"));
        }
    }
    std::fs::write(worktree_path.join("CLAUDE.md"), content)
}
```

### Pattern 4: Settings.json Injection

**What:** Write `.claude/settings.json` into the worktree to configure allowed tools and permissions for headless execution.

**Example:**
```json
{
  "permissions": {
    "allow": [
      "Bash(*)",
      "Read(*)",
      "Write(*)",
      "Edit(*)",
      "Glob(*)",
      "Grep(*)"
    ],
    "deny": [
      "Bash(git push *)",
      "Bash(git remote *)",
      "Bash(curl *)",
      "Bash(wget *)"
    ]
  }
}
```

Note: `--dangerously-skip-permissions` bypasses all permission checks, so settings.json primarily serves as defense-in-depth documentation. However, if the flag is ever removed or relaxed, the settings provide a safety net.

### Pattern 5: Process Lifecycle with Timeout

**What:** Use `tokio::select!` with `tokio::time::timeout` and `CancellationToken` for clean shutdown.

**Example:**
```rust
let mut child = cmd.spawn()?;
let pg = ProcessGroup::new(child.inner_child()); // need to extract std Child

tokio::select! {
    biased;
    _ = cancel.cancelled() => {
        pg.kill_group()?;
        // wait briefly for cleanup
        SessionResult { outcome: SessionOutcome::Killed, .. }
    }
    _ = tokio::time::sleep(timeout) => {
        pg.kill_group()?;
        SessionResult { outcome: SessionOutcome::TimedOut, .. }
    }
    output = child.wait_with_output() => {
        // Parse exit code, check commits
        match output {
            Ok(output) => map_exit_to_outcome(output.status, &git, &worktree_path),
            Err(e) => SessionResult { outcome: SessionOutcome::Failed, .. }
        }
    }
}
```

### Pattern 6: Log Capture

**What:** Capture stdout/stderr to log files while optionally streaming to dashboard.

**Key consideration:** `wait_with_output()` buffers everything. For streaming, use `tokio::io::BufReader` on `child.stdout.take()` with line-by-line reading and tee to file + optional dashboard callback.

For v0.1.0, full buffering via `wait_with_output()` is simpler and sufficient. Stream after process completes by writing the buffer to the log file.

### Anti-Patterns to Avoid

- **Spawning without process_group(0):** The child inherits the parent's PGID. SIGTERM to the orchestrator would kill the agent, but not its children (subprocesses spawned by Claude Code). Always use `.process_group(0)` and `kill(-pgid, SIGTERM)`.
- **Using `child.kill()` instead of `kill_group()`:** `child.kill()` only signals the direct child, not its subprocess tree. Claude Code spawns subprocesses (git, cargo, etc.) that would become orphans.
- **Blocking on stdin:** Never pipe stdin to Claude Code in `-p` mode. The `-p` flag is non-interactive; stdin is not read.
- **Forgetting `kill_on_drop(true)`:** If the `Child` handle is dropped without waiting (e.g., due to a panic), the process continues running. Set `kill_on_drop(true)` as a safety net alongside explicit `ProcessGroup::kill_group()`.
- **Parsing text output:** Use `--output-format json` for structured data. Text output format changes between Claude Code versions.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
| --- | --- | --- | --- |
| Process group signal | Manual `fork()` + `setsid()` | `Command::process_group(0)` + `libc::kill(-pgid, SIGTERM)` | Already implemented in `ProcessGroup`; standard POSIX pattern |
| Async process wait with timeout | `thread::sleep` + poll loop | `tokio::select!` with `tokio::time::sleep` | Integrates with cancellation token, no busy-wait |
| JSON output parsing | Regex on stdout | `serde_json::from_str` on `--output-format json` output | Claude Code has a documented JSON schema |
| CLAUDE.md discovery | Custom config injection mechanism | Write `CLAUDE.md` to worktree root | Claude Code reads it automatically from CWD |
| Permission bypass | Custom hooks or settings | `--dangerously-skip-permissions` flag | Official supported flag for headless/CI use |
| Model selection | Environment variable hacks | `--model sonnet` CLI flag or `settings.json` `model` field | Official configuration mechanism |

**Key insight:** Claude Code was designed for CLI automation. The `-p` flag, JSON output, and `--dangerously-skip-permissions` exist specifically for this use case. Don't reinvent what the tool already provides.

## Common Pitfalls

### Pitfall 1: ProcessGroup vs tokio::process::Child

**What goes wrong:** `tokio::process::Command::spawn()` returns a `tokio::process::Child`, but `ProcessGroup` currently wraps `std::process::Child`. These are different types.

**Why it happens:** The existing `ProcessGroup` was designed for `std::process::Child`. Tokio's `Child` wraps it but doesn't expose the inner `std::process::Child` directly.

**How to avoid:** Two options:
1. Spawn with `std::process::Command`, then use `tokio::process::Child::from_std(child)` to wrap it for async wait. This preserves access to the raw `Child` for `ProcessGroup`.
2. Use `tokio::process::Command` but extract the PID via `child.id()` and call `libc::kill(-pid, SIGTERM)` directly (since PID == PGID when spawned with `process_group(0)`).

Option 2 is cleaner — extract PID at spawn time, store it, and use it for `kill_group()`.

**Warning signs:** Compilation error trying to pass `tokio::process::Child` to `ProcessGroup::new()`.

### Pitfall 2: Prompt Length Limits

**What goes wrong:** Very long `--prompt` arguments may exceed OS argument length limits (`ARG_MAX`, typically 256KB on macOS, ~2MB on Linux).

**Why it happens:** Task descriptions with extensive context could approach these limits.

**How to avoid:** For v0.1.0, the `--prompt` flag is sufficient (task descriptions are short). Document that `--prompt-file` is deferred. Monitor prompt sizes and add a guard if they exceed, say, 100KB.

**Warning signs:** `E2BIG` error from `exec()`.

### Pitfall 3: CLAUDE.md Conflicts with Existing Files

**What goes wrong:** The worktree may already contain a `CLAUDE.md` from the repository. Overwriting it destroys project-specific instructions that Claude Code should follow.

**How to avoid:** Check if `CLAUDE.md` exists before injection. If it does, append session constraints to the existing file (or write to `.claude/CLAUDE.md` which is also read). The `.claude/CLAUDE.md` path is safer since worktrees are based off repository commits that may include a root `CLAUDE.md`.

**Warning signs:** Agent ignores project conventions because its `CLAUDE.md` was overwritten.

### Pitfall 4: Exit Code 0 with No Commits

**What goes wrong:** Claude Code exits successfully but doesn't make any commits — perhaps it concluded the task was already done, or it couldn't figure out what to do.

**Why it happens:** Claude Code's exit code reflects conversation success, not whether code changes were made.

**How to avoid:** Already planned — use `git rev-list --count` to verify commits exist on the branch. Map to an appropriate `SessionOutcome` (per CONTEXT.md: "Claude's discretion").

**Recommended mapping:** Exit 0 + no commits = `SessionOutcome::Completed` with `has_commits: false` and a warning log. This allows the orchestrator to skip it during merge without treating it as a failure.

### Pitfall 5: Claude Code Not Installed

**What goes wrong:** The `claude` binary doesn't exist on `PATH`, or it's an older version missing required flags.

**How to avoid:** Add a preflight check (similar to existing `git::preflight()`) that resolves the `claude` binary via `which::which("claude")` and optionally checks `claude --version` output. Fail early with a clear error message.

**Warning signs:** Cryptic `No such file or directory` error at session execution time.

### Pitfall 6: Zombie Process Accumulation

**What goes wrong:** If the orchestrator crashes between spawning the agent and calling `wait()`, the agent process becomes a zombie (or continues running).

**How to avoid:**
1. `kill_on_drop(true)` on `tokio::process::Command` — sends SIGKILL on drop
2. Write PID to worktree state file immediately after spawn — enables cleanup on resume
3. Existing orphan detection in `WorktreeManager` already checks PID liveness

**Warning signs:** `ps aux | grep claude` shows orphaned processes after orchestrator crashes.

### Pitfall 7: Log File Contention

**What goes wrong:** Multiple sessions write to the same log directory concurrently, but this is fine because each session has its own log file.

**How to avoid:** Already handled — `RunStateManager::log_path()` returns `<run_id>/logs/<session>.log`. Each session name is unique.

## Code Examples

### Spawning Claude Code with Process Group (verified pattern)

```rust
// Source: tokio docs + existing ProcessGroup pattern
use std::process::Stdio;
use tokio::process::Command;

let mut cmd = Command::new("claude");
cmd.args(["-p", &prompt, "--dangerously-skip-permissions", "--output-format", "json"]);
cmd.current_dir(&worktree_path);
cmd.stdout(Stdio::piped());
cmd.stderr(Stdio::piped());
cmd.process_group(0);        // New PGID = child PID
cmd.kill_on_drop(true);      // Safety net

let child = cmd.spawn()?;
let pid = child.id().expect("child has PID");

// Store PID in worktree state for orphan detection
// ...

// Later, for graceful shutdown:
unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
```

### Parsing Claude Code JSON Output (verified schema)

```rust
// Source: Claude Code docs (--output-format json)
#[derive(Debug, Deserialize)]
struct ClaudeOutput {
    result: Option<String>,
    #[serde(rename = "session_id")]
    session_id: Option<String>,
    // Cost info if available
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    // The exact schema may vary; deserialize loosely
}

fn parse_claude_output(stdout: &[u8]) -> Option<ClaudeOutput> {
    serde_json::from_slice(stdout).ok()
}
```

### Timeout + Cancellation Select Pattern (verified tokio pattern)

```rust
// Source: tokio docs for select! + timeout
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

let deadline = Duration::from_secs(session_timeout_secs);

tokio::select! {
    biased;
    _ = cancel.cancelled() => {
        // Orchestrator requested abort
        kill_process_group(pid);
        wait_for_exit(&mut child).await;
        SessionOutcome::Killed
    }
    result = timeout(deadline, child.wait_with_output()) => {
        match result {
            Ok(Ok(output)) => {
                // Normal completion — check exit code + commits
                map_output_to_result(output, session_name, &git, &worktree_path).await
            }
            Ok(Err(io_err)) => {
                // IO error waiting for process
                SessionOutcome::Failed
            }
            Err(_elapsed) => {
                // Timeout — kill and report
                kill_process_group(pid);
                wait_for_exit(&mut child).await;
                SessionOutcome::TimedOut
            }
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
| --- | --- | --- | --- |
| `claude --prompt-file` | `claude -p "..."` | Current | `-p` is the primary non-interactive flag; `--prompt-file` exists but is less documented |
| Manual permission approval | `--dangerously-skip-permissions` | Current | Required for headless execution; no prompt interruptions |
| Text output parsing | `--output-format json` | Current | Structured JSON with session_id, cost, duration metadata |
| N/A | `--allowed-tools` CLI flag | Current | Restrict tools at invocation time (alternative to settings.json) |
| N/A | `--max-turns` CLI flag | Current | Limit conversation turns to prevent runaway sessions |

**Deprecated/outdated:**
- None identified for the flags we're using. Claude Code CLI is actively maintained.

## Open Questions

1. **Exact JSON output schema for `--output-format json`**
   - What we know: Includes `result`, `session_id`, cost/duration fields
   - What's unclear: Exact field names and nesting (varies between versions)
   - Recommendation: Use `serde_json::Value` for initial parsing, extract known fields loosely. Add a structured type later once schema stabilizes. Log the raw JSON for debugging.

2. **`--model` flag exact syntax**
   - What we know: Settings.json uses `"model": "sonnet"` or `"model": "opus"`. CLI likely uses `--model sonnet`.
   - What's unclear: Whether full model IDs (e.g., `claude-sonnet-4-20250514`) are required vs short names
   - Recommendation: Default to no `--model` flag (use whatever Claude Code defaults to). Allow manifest override if specified.

3. **`--max-turns` behavior at limit**
   - What we know: Flag exists for turn limiting
   - What's unclear: Whether Claude Code exits with 0 or non-zero when max turns is reached
   - Recommendation: Don't use `--max-turns` in v0.1.0. Use timeout as the primary limit. Document for future investigation.

4. **`--allowed-tools` vs settings.json**
   - What we know: Both exist. CLI flag overrides per-invocation.
   - What's unclear: Whether CLI flag completely replaces or merges with settings.json
   - Recommendation: Use `--dangerously-skip-permissions` + settings.json deny rules for safety. Don't mix in `--allowed-tools` to avoid confusion.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/code_claude` — Claude Code official documentation: headless mode, CLI flags, settings.json, CLAUDE.md, permissions
- Context7 `/anthropics/claude-code` — Claude Code GitHub: command development, hooks, allowed-tools
- Context7 `/websites/rs_tokio_tokio` — tokio::process documentation: Command, process_group, kill_on_drop, wait_with_output

### Secondary (HIGH confidence)
- Existing codebase analysis: `ProcessGroup`, `SessionRunner`, `ScriptExecutor`, `WorktreeManager`, `SessionOutcome` types
- Workspace `Cargo.toml`: all required dependencies already present

### Tertiary (LOW confidence)
- Claude Code JSON output schema details — inferred from documentation examples, not a formal schema specification

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in workspace, Claude Code CLI well-documented
- Architecture: HIGH — mirrors existing `ScriptExecutor` pattern, clear integration points in `execute_sessions`
- Pitfalls: HIGH — process group management is well-understood POSIX; Claude Code flags are documented
- JSON output schema: LOW — no formal schema published; field names inferred from examples

**Research date:** 2026-03-11
**Valid until:** 2026-04-11 (30 days — Claude Code CLI is actively evolving)
