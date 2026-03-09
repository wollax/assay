# Stack Research: v0.3.0 Orchestration Features

**Research Date:** 2026-03-08
**Scope:** Stack additions for worktree management, Claude Code launching, diff assembly, and session tracking
**Existing Stack:** Rust 1.93 stable (2024 edition), serde 1.x, serde_json 1.x, schemars 1.x, clap 4, rmcp 0.17, tokio 1 (full features), chrono 0.4, thiserror 2, tracing 0.1, tempfile 3, toml 0.8, regex-lite 0.1, notify 7, dirs 6, libc 0.2
**Constraint:** Zero new workspace dependencies preferred; cargo-deny with `multiple-versions = "deny"` and strict source controls

---

## Executive Summary

The v0.3.0 features can be built with **zero new workspace dependencies**. Git worktree management should shell out to `git` CLI (not use git2/gix). Claude Code launching uses `std::process::Command` (existing pattern in `gate/mod.rs`). Diff generation shells out to `git diff`. Session tracking uses the established JSON-file-per-record pattern from `assay-core::history`.

The only workspace-level `Cargo.toml` change is: none. All required capabilities are covered by `std::process`, `tokio::task::spawn_blocking`, `serde_json`, `chrono`, `tempfile`, and `std::fs` -- all already in workspace dependencies.

**Confidence:** High. Every recommendation is grounded in the existing codebase patterns (particularly `gate/mod.rs::evaluate_command`) and verified against current crate versions (2026-03-08).

---

## Area 1: Git Worktree Management

### The Decision: Shell Out to `git` CLI

**Recommendation: Use `std::process::Command` to invoke the `git` CLI.** Do not add `git2` or `gix`.

### Why NOT git2

| Factor | git2 0.20.4 | git CLI |
|--------|------------|---------|
| **Dependency weight** | C library (libgit2) linked via `libgit2-sys` -- adds ~80 transitive deps, C build toolchain requirement (`cmake` or `pkg-config + libgit2-dev`) | Zero deps. Git is a prerequisite for any developer using Assay |
| **cargo-deny impact** | `libgit2-sys` pulls `cc`, `pkg-config`, `openssl-sys` (on Linux). Likely triggers `multiple-versions = "deny"` for `cc` and `libc` variants | No impact |
| **Build time** | +15-30s for libgit2 C compilation on clean builds | Zero |
| **Worktree API completeness** | Has `Repository::worktree()` (add), `worktrees()` (list), `find_worktree()` (open), `Worktree::prune()` (remove). Missing: `worktree remove` (must use `prune` which has different semantics). No `--porcelain` list equivalent | Full worktree API: `add`, `list --porcelain`, `remove`, `lock`, `move`, `repair` |
| **Thread safety** | `Worktree` is `!Send + !Sync` -- cannot be passed across threads, complicates async integration | Subprocess-per-call, naturally thread-safe |
| **Error quality** | libgit2 error codes + English messages | git CLI stderr, well-known format |
| **Cross-platform** | Handles Windows path normalization | Git handles it too |

**Critical issue with git2's Worktree API:** `Worktree` is `!Send` and `!Sync`. Since Assay uses `tokio::task::spawn_blocking` for blocking operations (established in gate evaluation), the worktree handle cannot be moved into the spawned closure. Every operation would need to open a fresh `Repository` + `find_worktree()`. This negates the primary advantage of a library binding (holding handles).

### Why NOT gix (gitoxide)

| Factor | gix 0.80.0 | Assessment |
|--------|-----------|------------|
| **Pure Rust** | Yes -- no C dependency | Advantage over git2 |
| **Dependency weight** | ~150+ transitive deps (gix-hash, gix-object, gix-ref, gix-config, ...) | Worse than git2 |
| **Worktree support** | Partial -- `gix::open()` detects worktrees, but worktree CRUD (create/remove) is not exposed as a high-level API. Would need `gix-command` (shelling out to git anyway) | Defeats the purpose |
| **API stability** | Pre-1.0, breaking changes between 0.x releases | Risk |
| **cargo-deny** | Would add 50+ new crate entries to the allow list | Unacceptable |

### Shell-Out Pattern (Recommended)

The existing `evaluate_command()` in `gate/mod.rs` provides the exact pattern: `Command::new("sh").args(["-c", cmd])` with piped stdout/stderr, reader threads, and timeout enforcement. For git operations, use `Command::new("git")` directly (no shell wrapper needed).

```rust
// In assay-core::worktree

use std::process::{Command, Stdio};
use std::path::Path;

/// Create a git worktree for the given branch at the target path.
pub fn create(
    repo_root: &Path,
    worktree_path: &Path,
    branch_name: &str,
    base_ref: &str,
) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "add", "-b", branch_name])
        .arg(worktree_path)
        .arg(base_ref)
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| /* wrap in AssayError */)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(/* AssayError::WorktreeCreation { ... } */);
    }
    Ok(())
}

/// List worktrees using porcelain format for stable parsing.
pub fn list(repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    // Parse porcelain output: "worktree /path\nHEAD abc123\nbranch refs/heads/name\n\n"
    parse_porcelain_list(&output.stdout)
}
```

**Porcelain output format** (`git worktree list --porcelain`) is stable and machine-parseable:

```
worktree /path/to/main
HEAD abc123def456
branch refs/heads/main

worktree /path/to/feature
HEAD def456abc123
branch refs/heads/feature-branch

```

Each worktree is a block of key-value lines separated by blank lines. Parse with `regex-lite` (already in workspace) or simple line splitting.

### Git CLI Operations Needed

| Operation | Command | Notes |
|-----------|---------|-------|
| Create worktree | `git worktree add -b <branch> <path> <base>` | Creates branch + worktree in one command |
| List worktrees | `git worktree list --porcelain` | Stable parseable format |
| Remove worktree | `git worktree remove <path>` | Cleans up worktree + ref |
| Check if in repo | `git rev-parse --git-dir` | Validates CWD is a git repo |
| Get current branch | `git rev-parse --abbrev-ref HEAD` | For default base branch |
| Get repo root | `git rev-parse --show-toplevel` | For resolving relative paths |

### What NOT to Add

| Crate | Version | Why Considered | Why Rejected |
|-------|---------|----------------|--------------|
| `git2` | 0.20.4 | Native git operations without CLI dependency | C build dep (libgit2-sys), `!Send` worktree handles, 80+ transitive deps, cargo-deny violations |
| `gix` (gitoxide) | 0.80.0 | Pure Rust git implementation | 150+ transitive deps, no high-level worktree CRUD API, pre-1.0 instability |
| `gix-command` | 0.4.3 | gitoxide's subprocess wrapper for git CLI | Adds a dependency just to shell out -- `std::process::Command` does the same thing |
| `git-worktree` | (none) | Hypothetical worktree management crate | Does not exist |

### Integration with Existing Stack

- **Subprocess execution:** Follows `gate/mod.rs::evaluate_command()` pattern exactly
- **Path handling:** `std::path::Path` + `std::path::PathBuf` (stdlib)
- **Async bridge:** `tokio::task::spawn_blocking` for MCP handlers (existing pattern)
- **Output parsing:** `String::from_utf8_lossy` + line splitting or `regex-lite` (workspace dep)
- **Error handling:** New `AssayError` variants: `WorktreeCreation`, `WorktreeNotFound`, `GitNotAvailable`
- **Temp paths:** `tempfile::tempdir()` for test fixtures (workspace dep)

---

## Area 2: Subprocess Management (Claude Code Launcher)

### The Decision: `std::process::Command` with Monitoring

**Recommendation: Use `std::process::Command` (synchronous) wrapped in `tokio::task::spawn_blocking`.** Do not use `tokio::process::Command`.

### Why `std::process::Command` Over `tokio::process::Command`

| Factor | `std::process::Command` | `tokio::process::Command` |
|--------|------------------------|--------------------------|
| **Existing pattern** | Used in `gate/mod.rs` for all command evaluation | Not used anywhere in the codebase |
| **Timeout enforcement** | `try_wait` polling loop (proven in gate eval) | `tokio::time::timeout` + `child.wait()` |
| **Process group kill** | `libc::killpg` (proven in gate eval) | Same, but need `unsafe` in async context |
| **Reader threads** | `std::thread::spawn` for pipe draining | `tokio::spawn` + `AsyncReadExt` |
| **Complexity** | Well-understood, battle-tested in this codebase | Would introduce a second subprocess pattern |

**The critical argument:** The project already has a working, tested subprocess management pattern in `evaluate_command()`. The Claude Code launcher is a longer-running version of the same operation: spawn a process, capture output, enforce a timeout, kill on timeout. Introducing `tokio::process::Command` would create two divergent subprocess management paths in the same codebase with no benefit.

### Claude Code `--print` Mode Integration

Claude Code in `--print` mode (`claude -p "prompt"`) is a non-interactive, single-shot execution:

```bash
claude -p "Implement the auth flow per the spec" \
    --output-format json \
    --allowedTools "Edit,Write,Bash,Read,Glob,Grep" \
    --max-tokens 100000
```

**Output formats:**
- `--output-format text` (default): Plain text response, human-readable
- `--output-format json`: Structured JSON with result, session_id, metadata, cost, duration
- `--output-format stream-json`: NDJSON stream of events (real-time progress)

**Recommendation:** Use `--output-format json` for the v0.3.0 sequential workflow. The JSON output provides structured completion data (exit status, token usage, cost) that feeds directly into the session record. Stream-json is useful for real-time TUI updates in v0.4.

### Launcher Implementation Pattern

```rust
// In assay-core::launcher (or assay-core::claude_code)

use std::process::{Command, Stdio};
use std::path::Path;
use std::time::Duration;

pub struct LaunchConfig {
    /// Working directory (worktree path).
    pub working_dir: PathBuf,
    /// The prompt to send to Claude Code.
    pub prompt: String,
    /// Maximum wall-clock time for the agent session.
    pub timeout: Duration,
    /// Allowed tools (subset of Claude Code tools).
    pub allowed_tools: Vec<String>,
}

pub struct LaunchResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// Launch Claude Code in --print mode.
///
/// Spawns `claude -p <prompt>` with the specified configuration.
/// Uses the same timeout/kill pattern as gate command evaluation.
pub fn launch(config: &LaunchConfig) -> Result<LaunchResult> {
    let mut command = Command::new("claude");
    command
        .arg("-p")
        .arg(&config.prompt)
        .arg("--output-format")
        .arg("json")
        .current_dir(&config.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if !config.allowed_tools.is_empty() {
        command.arg("--allowedTools")
            .arg(config.allowed_tools.join(","));
    }

    // Same process group + timeout + reader thread pattern as evaluate_command()
    // ...
}
```

### Long-Running Process Considerations

Unlike gate commands (typically seconds), Claude Code sessions can run for minutes to hours. Key differences from the gate evaluation pattern:

| Concern | Gate Evaluation | Claude Code Launch |
|---------|----------------|-------------------|
| **Typical duration** | 1-60 seconds | 5-60 minutes |
| **Timeout default** | 300 seconds | 30 minutes (configurable) |
| **Output size** | KB range (test output) | MB range (full implementation log) |
| **Kill semantics** | Kill on timeout | Kill on timeout + user cancel |
| **PID tracking** | Not needed (short-lived) | Store in SessionRecord for status queries |

**Output handling for long sessions:** The existing `MAX_OUTPUT_BYTES = 65_536` truncation in gate evaluation is appropriate for the final result capture. For the v0.3 sequential workflow, we only need the final output (not streaming). Stream-json parsing for real-time progress is a v0.4 concern.

**PID tracking:** The `child.id()` value must be stored in the `SessionRecord` immediately after spawn, before waiting. This enables `assay session show <id>` to display whether the agent is still running (check PID liveness via `libc::kill(pid, 0)` on Unix, already available as a workspace dep).

### What NOT to Add

| Crate | Version | Why Considered | Why Rejected |
|-------|---------|----------------|--------------|
| `tokio-process` | (part of tokio) | Async subprocess management | Would create a second subprocess pattern. The sync `std::process::Command` + `spawn_blocking` approach is proven and consistent. |
| `duct` | 0.13.7 | Ergonomic subprocess piping | Wrapper over `std::process::Command`. Adds a dependency for marginally cleaner syntax. Not worth it. |
| `subprocess` | 0.2.10 | Alternative process library | Less maintained than std. No advantages. |
| `nix` | 0.30.1 | Unix process management (signals, process groups) | `libc` (already in workspace) covers everything needed: `killpg`, `kill`, `waitpid`. `nix` is a safe wrapper but an unnecessary dep. |
| `signal-hook` | 0.3.17 | Signal handling for graceful shutdown | Assay doesn't need to handle signals itself -- it sends them to child processes via `libc::killpg`. |

### Integration with Existing Stack

- **Subprocess:** Reuses the exact `evaluate_command()` pattern from `gate/mod.rs`
- **Process group kill:** `libc::killpg` (workspace dep, already used in gate eval)
- **PID liveness check:** `libc::kill(pid, 0)` (zero-signal probe, POSIX standard)
- **Async bridge:** `tokio::task::spawn_blocking` (existing pattern in MCP handlers)
- **Output parsing:** `serde_json::from_str` for `--output-format json` output (workspace dep)
- **Timeout:** `std::time::Duration` + `Instant::elapsed()` polling (existing pattern)
- **Session recording:** `serde_json::to_string_pretty` + `std::fs::write` (existing history pattern)

---

## Area 3: Diff Generation and Assembly

### The Decision: Shell Out to `git diff`

**Recommendation: Use `git diff` CLI for all diff operations.** Do not add a Rust diff library.

### Why Shell Out to `git diff`

The diff assembly for gate evaluation context needs:

1. **File-level diff between base branch and worktree HEAD** -- what the agent changed
2. **Stat summary** -- which files changed, insertions/deletions
3. **Unified diff format** -- the standard patch format that LLMs understand well

All of these are directly available from `git diff`:

```bash
# Full unified diff between base and worktree HEAD
git diff main...HEAD

# Stat summary only
git diff --stat main...HEAD

# Diff for specific files (token budget management)
git diff main...HEAD -- src/auth.rs tests/auth_test.rs

# Names only (for file list)
git diff --name-only main...HEAD
```

### Why NOT a Rust Diff Library

| Factor | `git diff` CLI | `similar` 2.7.0 | `diffy` 0.4.2 |
|--------|---------------|-----------------|---------------|
| **What it diffs** | Git object tree (handles renames, binary detection, .gitignore) | In-memory strings/bytes | In-memory strings |
| **Rename detection** | Built-in (`-M` flag) | Manual | Not supported |
| **Binary file handling** | Detects and skips automatically | Must handle manually | Must handle manually |
| **Git context** | Knows about staging, branches, worktrees natively | No git awareness | No git awareness |
| **Token budget** | `--stat` for summary, path filters for targeted diffs | Must read all files into memory first | Same |
| **Dependencies** | Zero | +1 crate | +1 crate |
| **Output format** | Unified diff (LLMs understand this natively) | Custom diff types, must format | Unified diff output available |

**The case for `similar`/`diffy` would be:** if we needed to diff in-memory strings that don't come from git (e.g., comparing two spec versions, diffing MCP responses). That is not the v0.3.0 use case. Every diff in gate evaluation is between git commits/branches. Using `git diff` is both simpler and more correct.

### Diff Assembly Module Design

```rust
// In assay-core::diff (or assay-core::context::diff)

use std::process::Command;
use std::path::Path;

/// Context assembled for independent gate evaluation.
pub struct EvaluationContext {
    /// Summary of changes (file list with +/- counts).
    pub stat_summary: String,
    /// Full unified diff, potentially truncated to token budget.
    pub unified_diff: String,
    /// Files changed (for targeted evaluation).
    pub changed_files: Vec<String>,
    /// Base ref the diff is against.
    pub base_ref: String,
    /// Whether the diff was truncated to fit token budget.
    pub truncated: bool,
}

/// Assemble evaluation context from a worktree's changes.
pub fn assemble_context(
    worktree_path: &Path,
    base_ref: &str,
    max_diff_bytes: usize,
) -> Result<EvaluationContext> {
    let stat = git_diff_stat(worktree_path, base_ref)?;
    let changed_files = git_diff_name_only(worktree_path, base_ref)?;
    let (diff, truncated) = git_diff_unified(worktree_path, base_ref, max_diff_bytes)?;

    Ok(EvaluationContext {
        stat_summary: stat,
        unified_diff: diff,
        changed_files,
        base_ref: base_ref.to_string(),
        truncated,
    })
}

fn git_diff_stat(worktree: &Path, base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--stat", &format!("{base}...HEAD")])
        .current_dir(worktree)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

### Token Budget Management for Diffs

Large diffs can overwhelm the evaluating agent's context window. Strategy:

1. **Always include `--stat` summary** (small, gives overview)
2. **Always include `--name-only` file list** (small, identifies scope)
3. **Prioritize spec-referenced files** in the full diff (files mentioned in criterion descriptions)
4. **Prioritize test files** (strongest signal for quality evaluation)
5. **Truncate remaining diff** to fit within budget, noting truncation

The `max_diff_bytes` parameter allows the caller (gate_evaluate MCP tool) to set the budget based on the evaluating agent's available context. The existing `context::tokens` module (token estimation) can inform this.

### What NOT to Add

| Crate | Version | Why Considered | Why Rejected |
|-------|---------|----------------|--------------|
| `similar` | 2.7.0 | In-memory diff algorithm (Myers, patience) | All diffs are git-tracked. `git diff` is more correct (handles renames, binary, .gitignore) and adds zero deps. |
| `diffy` | 0.4.2 | Unified diff generation from strings | Same reasoning. Would need to read files into memory that git already has indexed. |
| `similar-asserts` | 1.7.0 | Test assertion diffs | Only useful in test code. `insta` (already in workspace) provides snapshot diffing for tests. |
| `git-diff` | (none) | Hypothetical structured diff parsing | Does not exist as a crate. Parse `--porcelain` or `--stat` output directly. |
| `patch` | 0.7.0 | Parse unified diff format | Only needed if we apply patches programmatically. v0.3 only reads/displays diffs. |
| `unidiff` | 0.4.0 | Parse unified diff into structured hunks | Overkill for v0.3. We pass the raw diff to the evaluating agent as text. Structured hunk parsing would matter for a v0.4 TUI diff viewer. |

### Integration with Existing Stack

- **Subprocess:** Same `Command::new("git")` pattern as worktree module
- **Output parsing:** `String::from_utf8_lossy` for diff text (stdlib)
- **Token estimation:** Existing `assay-core::context::tokens` module
- **Serialization:** `EvaluationContext` derives `Serialize` for inclusion in MCP responses and session records
- **Path handling:** `std::path::Path` (stdlib)

---

## Area 4: Session Tracking

### The Decision: JSON Files in `.assay/sessions/`

**Recommendation: Follow the existing `assay-core::history` persistence pattern exactly.** No new dependencies needed.

### Pattern Reuse from v0.2.0

The session tracking module mirrors the run history module (`assay-core::history`) in every dimension:

| Aspect | Run History (v0.2) | Session Record (v0.3) |
|--------|-------------------|----------------------|
| **Storage location** | `.assay/results/{spec}/{timestamp}_{id}.json` | `.assay/sessions/{timestamp}_{id}.json` |
| **File format** | Pretty-printed JSON via `serde_json::to_string_pretty` | Same |
| **ID generation** | `chrono::Utc::now()` + random hex suffix | Same (reuse `history::generate_run_id`) |
| **Atomic writes** | `tempfile::NamedTempFile::persist()` | Same |
| **Listing** | `std::fs::read_dir` + filename parsing + sort | Same |
| **Deserialization** | `serde_json::from_str` | Same |
| **Type location** | `SessionRecord` in `assay-types` | `SessionRecord` already sketched in brainstorm |
| **Gitignore** | `.assay/.gitignore` excludes `results/` | Add `sessions/` to `.gitignore` template |

### Session Record Type

The `AgentSession` type already exists in `assay-types` (used by the v0.2 gate session lifecycle). For v0.3.0, the persisted `SessionRecord` is a superset:

```rust
// Already in assay-types or to be added
pub struct SessionRecord {
    pub id: String,                        // Reuse history::generate_run_id()
    pub spec_name: String,
    pub worktree_path: PathBuf,
    pub branch_name: String,
    pub agent_pid: Option<u32>,
    pub status: SessionStatus,             // Active, Completed, Failed, Cancelled
    pub started_at: DateTime<Utc>,         // chrono (workspace dep)
    pub ended_at: Option<DateTime<Utc>>,
    pub gate_run_ids: Vec<String>,         // Links to history records
    pub launch_config: Option<LaunchConfigSummary>, // What was passed to claude
}

pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Cancelled,
}
```

All types use existing derives: `Serialize`, `Deserialize`, `JsonSchema`, `Debug`, `Clone`.

### What NOT to Add

| Crate | Version | Why Considered | Why Rejected |
|-------|---------|----------------|--------------|
| `uuid` | 1.16.0 | Unique session IDs | Timestamp + random hex suffix (existing pattern in `history::generate_run_id`) provides sufficient uniqueness without a new dep |
| `rusqlite` | 0.34.0 | Structured session queries | JSON files are sufficient for <100 concurrent sessions. Same reasoning as v0.2 history research. |
| `sled` / `redb` | 0.34 / 2.x | Embedded key-value store | Overkill for simple session bookkeeping |

### Integration with Existing Stack

- **Serialization:** `serde_json` (workspace dep)
- **Timestamps:** `chrono` (workspace dep)
- **Atomic writes:** `tempfile::NamedTempFile::persist()` (workspace dep)
- **ID generation:** Reuse `assay_core::history::generate_run_id()`
- **File I/O:** `std::fs` (stdlib)
- **Error handling:** New `AssayError` variants: `SessionWrite`, `SessionRead`

---

## Workspace Dependency Changes Summary

### Changes to Root `Cargo.toml` `[workspace.dependencies]`

**None.** Zero new workspace dependencies for v0.3.0.

### Per-Crate Dependency Changes

| Crate | Change | Rationale |
|-------|--------|-----------|
| `assay-core` | None | All needed deps (serde_json, chrono, tempfile, libc, tokio, tracing, regex-lite) are already dependencies |
| `assay-types` | None | New types use existing serde/schemars/chrono derives |
| `assay-mcp` | None | New MCP tools use existing rmcp macros |
| `assay-cli` | None | New CLI commands use existing clap derives |
| `assay-tui` | None | Minimal TUI uses existing ratatui + crossterm |

### cargo-deny Impact

**None.** No new crates means no new license entries, no new advisory checks, no new ban/skip entries. The existing `deny.toml` remains unchanged.

---

## Cross-Cutting: Subprocess Execution Patterns

### Existing Pattern (gate/mod.rs)

The codebase has a mature subprocess execution pattern in `evaluate_command()`:

1. `Command::new("sh").args(["-c", cmd])` -- shell wrapper for gate commands
2. `.current_dir(working_dir)` -- explicit working directory
3. `.stdout(Stdio::piped()).stderr(Stdio::piped())` -- capture output
4. `.process_group(0)` -- Unix process group for clean kill
5. Reader threads (`std::thread::spawn`) for deadlock-free pipe draining
6. `try_wait` polling loop with `POLL_INTERVAL_MS = 50`
7. `libc::killpg` on timeout
8. Output truncation at `MAX_OUTPUT_BYTES = 65_536`

### v0.3.0 Extensions to This Pattern

| Feature | What Changes | Reuse |
|---------|-------------|-------|
| **Worktree ops** | `Command::new("git")` (no shell wrapper needed) | Steps 2-5, 8. Shorter timeouts, smaller output. |
| **Claude Code launch** | `Command::new("claude")` (no shell wrapper) | Steps 2-8. Longer timeouts (30min default), PID tracking. |
| **Diff generation** | `Command::new("git")` with `diff` subcommand | Steps 2-5. No timeout needed (git diff is fast). Larger output (MB range). |

### Consider: Extract a Shared Subprocess Helper

The `evaluate_command()` function is 100+ lines including timeout logic, reader threads, and process group management. v0.3.0 adds three more subprocess call sites. Consider extracting a shared helper:

```rust
// assay-core::process (new internal module, NOT a new crate)

pub struct ProcessConfig {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub use_process_group: bool,
}

pub struct ProcessResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub truncated: bool,
}

/// Execute a process with timeout, output capture, and process group management.
pub fn execute(config: &ProcessConfig) -> Result<ProcessResult> {
    // Consolidation of evaluate_command() pattern
}
```

This is a **refactoring decision**, not a dependency decision. It can happen during week 1 (Worktree Manager) or be deferred to the integration phase. The subprocess pattern is the same regardless.

---

## External Tool Dependencies

### Required on Developer/CI Machines

| Tool | Minimum Version | Used By | Detection |
|------|----------------|---------|-----------|
| `git` | 2.20+ (worktree `--porcelain` support) | Worktree manager, diff assembly | `git --version`, fail fast with `AssayError::GitNotAvailable` |
| `claude` | Any (Claude Code CLI) | Claude Code launcher | `which claude`, fail fast with `AssayError::ClaudeNotAvailable` |

### Detection Strategy

Check for tool availability at command execution time, not at startup. Assay features that don't need git/claude should work without them installed.

```rust
fn ensure_git_available() -> Result<()> {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(AssayError::GitNotAvailable),
    }
}
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `git` CLI not available on target system | Very Low | High | Fail fast with clear error message. Git is a universal developer tool. |
| `claude` CLI not available or wrong version | Low | High | Detect at launch time. Provide clear installation instructions in error. |
| `git worktree list --porcelain` output format changes | Very Low | Medium | Pin to known format. Git is extraordinarily stable across versions. |
| `git diff` output too large for evaluation context | Medium | Medium | Token budget management with truncation. Prioritize spec-referenced + test files. |
| `--print` mode Claude Code limitations surface late | Medium | Medium | Research and validate `--print` behavior early (week 1). The brainstorm already identified this risk. |
| Subprocess helper extraction causes churn in gate/mod.rs | Low | Low | Optional refactor. Can keep `evaluate_command()` as-is and duplicate the pattern for new call sites. |

---

## Sources

- [git2-rs API docs](https://docs.rs/git2/0.20.4/git2/) -- verified 0.20.4 is latest (2026-03-08)
- [git2 Repository::worktree()](https://docs.rs/git2/0.20.4/git2/struct.Repository.html#method.worktree) -- worktree add API
- [git2 Worktree struct](https://docs.rs/git2/0.20.4/git2/struct.Worktree.html) -- `!Send`, `!Sync` verified
- [gix (gitoxide)](https://crates.io/crates/gix) -- 0.80.0 latest (2026-03-08)
- [similar crate](https://crates.io/crates/similar) -- 2.7.0 latest
- [diffy crate](https://crates.io/crates/diffy) -- 0.4.2 latest
- [tokio::process::Command](https://docs.rs/tokio/1.49.0/tokio/process/struct.Command.html) -- async subprocess API
- [Claude Code headless mode](https://code.claude.com/docs/en/headless) -- `--print` mode, output formats
- [Claude Code common workflows](https://code.claude.com/docs/en/common-workflows) -- output format control
- [git-worktree(1) man page](https://git-scm.com/docs/git-worktree) -- CLI reference
- [git-diff(1) man page](https://git-scm.com/docs/git-diff) -- diff output formats
- Current codebase: `crates/assay-core/src/gate/mod.rs` (subprocess pattern), `Cargo.toml` (workspace deps), `deny.toml` (cargo-deny config)

---

*Stack research completed: 2026-03-08*
