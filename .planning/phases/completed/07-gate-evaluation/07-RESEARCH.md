# Phase 7: Gate Evaluation - Research

**Researched:** 2026-03-02
**Domain:** Process execution with timeout, structured result capture, CLI streaming output, aggregate gate results
**Confidence:** HIGH

## Summary

Phase 7 implements gate evaluation: running commands, capturing output, enforcing timeouts, and presenting results through CLI. The core technical challenge is combining safe process execution (deadlock-free pipe draining) with timeout enforcement and eventual output truncation. A secondary challenge is the CLI streaming display (cargo-test-style progress).

The recommended approach uses `Command::spawn()` with piped stdout/stderr, reader threads for pipe draining, and `try_wait`-based timeout enforcement. This supersedes the STATE.md decision to use `Command::output()` because `output()` blocks with no timeout mechanism. The brainstorm recommendation for `BufReader` + byte-budget is sound in principle but should be deferred to a post-v0.1.0 issue — unbounded capture with a hardcoded truncation limit at the String level is simpler and sufficient for v0.1.0.

**Primary recommendation:** Implement gate evaluation as a sync function in `assay-core::gate` that spawns a child process, drains pipes in threads, enforces timeout via `try_wait` polling, and returns `GateResult`. No new external crates needed — the `wait-timeout` crate is a reasonable alternative but `try_wait` + `thread::sleep` is simple enough for v0.1.0. Use `process_group(0)` (Unix) to ensure timeout kills the entire process tree.

## Standard Stack

### Core (already in workspace)

| Library        | Version | Purpose                                         | Notes                                          |
| -------------- | ------- | ----------------------------------------------- | ---------------------------------------------- |
| std::process   | std     | `Command::spawn()`, `Child`, `Stdio::piped()`   | Core process execution                         |
| std::thread    | std     | Pipe reader threads for deadlock-free capture    | Join after process exit or timeout             |
| std::time      | std     | `Instant`, `Duration` for timeout enforcement    | No external crate needed                       |
| std::os::unix  | std     | `CommandExt::process_group(0)` for kill-tree     | Unix-only, `#[cfg(unix)]`                      |
| chrono         | 0.4     | `DateTime<Utc>` timestamps on `GateResult`       | Already in workspace                           |
| thiserror      | 2       | New `AssayError` variants for gate errors        | Already in assay-core                          |
| clap           | 4       | `gate run` subcommand, `--timeout`/`--verbose`   | Already in assay-cli                           |
| serde_json     | 1       | `--json` output for gate results                 | Already in assay-cli                           |
| serde          | 1       | Serialization of `GateResult`, updated types     | Already in workspace                           |
| toml           | 0.8     | Loading specs (via existing `spec::load`)        | Already in assay-core                          |

### New Dependencies Required

None. All functionality can be implemented with `std` and existing workspace dependencies.

### Alternatives Considered

| Instead of            | Could Use        | Tradeoff                                                                                  |
| --------------------- | ---------------- | ----------------------------------------------------------------------------------------- |
| `try_wait` loop       | `wait-timeout`   | Cleaner API but adds a dependency for ~15 lines of code                                   |
| Manual pipe threads   | `Command::output`| Simpler but impossible to add timeout (blocks until process exits)                        |
| `BufReader` + budget  | `read_to_end`    | Budget prevents OOM on huge output; deferred to post-v0.1.0 per brainstorm issue          |
| `tokio::process`      | `std::process`   | Async timeout is cleaner but gate eval is sync per GATE-08; MCP uses `spawn_blocking`     |
| `indicatif`           | Manual ANSI      | Richer spinners but adds dependency; project convention is no external color/table libs    |
| `nix` for `killpg`    | `libc::kill(-pg)`| Nicer API but `process_group(0)` + `child.kill()` is sufficient for v0.1.0                |

## Architecture Patterns

### Pattern 1: Gate Evaluate Function Signature

Gate evaluation is a free function in `assay-core::gate` that takes explicit parameters and returns a `GateResult`. No method on a struct — matches the project's functional pattern.

```rust
// crates/assay-core/src/gate/mod.rs

/// Evaluate a single criterion as a gate.
///
/// `working_dir` is required — this function never inherits the process CWD.
/// `timeout` is the maximum wall-clock time for the command to complete.
pub fn evaluate(
    criterion: &Criterion,
    working_dir: &Path,
    timeout: Duration,
) -> Result<GateResult> {
    match criterion_to_gate_kind(criterion) {
        GateKind::Command { cmd } => evaluate_command(&cmd, working_dir, timeout),
        GateKind::AlwaysPass => evaluate_always_pass(),
        GateKind::FileExists { path } => evaluate_file_exists(&path, working_dir),
    }
}
```

Note: The function takes a `Criterion` (config type from spec) and produces a `GateResult` (state type). The `criterion_to_gate_kind` helper derives the `GateKind` from the criterion's fields. This maintains Config != State separation.

### Pattern 2: Command Execution with Timeout

The recommended execution pattern spawns the child, drains pipes in threads, and polls `try_wait` with a timeout.

```rust
fn evaluate_command(
    cmd: &str,
    working_dir: &Path,
    timeout: Duration,
) -> Result<GateResult> {
    let start = Instant::now();

    let mut child = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)  // Unix: new process group for kill-tree
        .spawn()
        .map_err(|source| AssayError::GateExecution { /* ... */ })?;

    // Take pipe handles before waiting (avoids borrow issues)
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Spawn reader threads to drain pipes (prevents deadlock)
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stdout) = stdout_handle {
            let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
        }
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stderr) = stderr_handle {
            let _ = std::io::Read::read_to_end(&mut stderr, &mut buf);
        }
        buf
    });

    // Poll for completion with timeout
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // Reap zombie
                    break None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(AssayError::GateExecution { /* ... */ }),
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Join reader threads (safe: process is dead, pipes will EOF)
    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();

    let stdout = String::from_utf8_lossy(&stdout_bytes);
    let stderr = String::from_utf8_lossy(&stderr_bytes);

    // Apply truncation (tail-biased)
    let stdout = truncate_output(&stdout, MAX_OUTPUT_BYTES);
    let stderr = truncate_output(&stderr, MAX_OUTPUT_BYTES);

    // Build result based on whether timeout occurred
    match status {
        Some(exit_status) => {
            let exit_code = exit_status.code(); // None if killed by signal
            Ok(GateResult {
                passed: exit_status.success(),
                kind: GateKind::Command { cmd: cmd.to_string() },
                stdout: stdout.into_owned(),
                stderr: stderr.into_owned(),
                exit_code,
                duration_ms,
                timestamp: Utc::now(),
            })
        }
        None => {
            // Timeout: report as failed with timeout information in stderr
            Ok(GateResult {
                passed: false,
                kind: GateKind::Command { cmd: cmd.to_string() },
                stdout: stdout.into_owned(),
                stderr: format!("{}\n[timed out after {}s]",
                    stderr, timeout.as_secs()),
                exit_code: None, // No exit code on timeout
                duration_ms,
                timestamp: Utc::now(),
            })
        }
    }
}
```

### Pattern 3: Aggregate Gate Results

Gate results are collected per-spec and summarized. The aggregate type is a core return type, not a DTO in assay-types (it's a computed summary, not persisted config/state).

```rust
// crates/assay-core/src/gate/mod.rs

/// Summary of evaluating all criteria in a spec.
pub struct GateRunSummary {
    /// Spec name that was evaluated.
    pub spec_name: String,
    /// Results for each criterion that was executed.
    pub results: Vec<CriterionResult>,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Number of criteria skipped (descriptive-only, no cmd).
    pub skipped: usize,
    /// Total wall-clock duration for all evaluations.
    pub total_duration_ms: u64,
}

/// A criterion paired with its evaluation result.
pub struct CriterionResult {
    /// The criterion that was evaluated.
    pub criterion_name: String,
    /// The gate result, or None if skipped (no cmd).
    pub result: Option<GateResult>,
}
```

### Pattern 4: Three-Tier Timeout Resolution

Timeout precedence: CLI `--timeout` flag > per-criterion `timeout` field > global `[gates].default_timeout` > hardcoded 300s.

```rust
fn resolve_timeout(
    cli_timeout: Option<u64>,
    criterion_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> Duration {
    let seconds = cli_timeout
        .or(criterion_timeout)
        .or(config_timeout)
        .unwrap_or(300);
    Duration::from_secs(seconds)
}
```

### Pattern 5: CLI Streaming Display

Match the cargo-test feel: show criterion name while running, replace with result on completion. Use `\r` + `\x1b[K` (clear to end of line) for the running state, then `\n` for the final state.

```rust
// While running (overwritable line):
eprint!("\r\x1b[K  {} ... running", criterion_name);

// On completion (permanent line):
eprintln!("\r\x1b[K  {} ... {}", criterion_name,
    if passed { "\x1b[32mok\x1b[0m" } else { "\x1b[31mFAILED\x1b[0m" });
```

Use `eprint!`/`eprintln!` for progress (stderr), `println!` for final summary and `--json` output (stdout). This follows the convention that progress is ephemeral (stderr) and results are durable (stdout).

### Pattern 6: GateKind::FileExists

File existence checks are simple: resolve the path relative to `working_dir`, check `Path::exists()`. No process execution, no timeout needed.

```rust
fn evaluate_file_exists(path: &str, working_dir: &Path) -> Result<GateResult> {
    let start = Instant::now();
    let full_path = working_dir.join(path);
    let exists = full_path.exists();

    Ok(GateResult {
        passed: exists,
        kind: GateKind::FileExists { path: path.to_string() },
        stdout: String::new(),
        stderr: if !exists {
            format!("file not found: {}", full_path.display())
        } else {
            String::new()
        },
        exit_code: None,
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: Utc::now(),
    })
}
```

## Don't Hand-Roll

| Problem                          | Use Instead                               | Why                                                          |
| -------------------------------- | ----------------------------------------- | ------------------------------------------------------------ |
| Pipe reading concurrency         | `std::thread::spawn` + `read_to_end`      | Reimplementing concurrent read is error-prone                |
| UTF-8 conversion from process    | `String::from_utf8_lossy()`               | Never `unwrap()` on process output — binary data is possible |
| Shell command parsing             | `sh -c` (Unix)                            | Don't parse shell syntax; let the shell handle it            |
| Process group management         | `CommandExt::process_group(0)` + `kill()`  | Don't call `libc::killpg` manually unless needed             |
| Exit code interpretation         | `ExitStatus::success()` + `.code()`        | Platform-specific edge cases (signals) handled by std        |
| Timeout polling                  | `try_wait` + `thread::sleep(50ms)`         | Don't busy-loop; 50ms polling is responsive enough           |
| Zombie reaping                   | `child.wait()` after `child.kill()`         | Always wait after kill to reap zombie process                |
| Timestamp generation              | `chrono::Utc::now()`                       | Already the convention from Phase 3                          |

## Common Pitfalls

### Pitfall 1: Forgetting to Reap After Kill

**What goes wrong:** Calling `child.kill()` sends SIGKILL but doesn't wait for the process to exit. The child becomes a zombie process, and the pipe reader threads may hang because stdout/stderr aren't closed until the OS cleans up the zombie.

**Why it happens:** `kill()` is non-blocking. The process isn't fully dead until `wait()` reaps it.

**How to avoid:** Always call `child.wait()` (or `try_wait()` in a brief loop) after `child.kill()`. This reaps the zombie and ensures pipe EOF.

```rust
let _ = child.kill();
let _ = child.wait(); // Reap zombie, ignore error (already dying)
```

**Warning signs:** Tests that hang intermittently on timeout paths; zombie processes in `ps` output.

### Pitfall 2: Pipe Deadlock from Sequential Reading

**What goes wrong:** Reading stdout to completion, then reading stderr (or vice versa). If the child writes enough to fill the unread pipe buffer, it blocks, and the parent blocks waiting for stdout EOF.

**Why it happens:** OS pipe buffers are finite (typically 64KB on Linux, 16KB on macOS). If the child fills stderr while the parent is blocking on stdout, deadlock.

**How to avoid:** Read stdout and stderr concurrently. Spawn a thread per pipe, or use `Command::output()` (which does this internally). Never read one then the other sequentially.

```rust
// WRONG: sequential reads
let stdout = child.stdout.take().unwrap();
let mut out = String::new();
stdout.read_to_string(&mut out)?; // May deadlock here
let stderr = child.stderr.take().unwrap();
// ...

// RIGHT: concurrent reads via threads
let stdout_thread = thread::spawn(move || { /* read stdout */ });
let stderr_thread = thread::spawn(move || { /* read stderr */ });
```

**Warning signs:** Tests that hang when commands produce output on both stdout and stderr.

### Pitfall 3: `child.kill()` Only Kills Direct Child, Not Grandchildren

**What goes wrong:** Running `sh -c "sleep 100 && echo done"` — killing the `sh` process leaves `sleep` running as an orphan.

**Why it happens:** `child.kill()` sends SIGKILL to the direct child process only. Shell commands often spawn subprocesses that aren't in the kill path.

**How to avoid:** Use `CommandExt::process_group(0)` when spawning (puts the child in its own process group), then kill sends to the group. The `child.kill()` call will kill the process group leader, and the OS propagates the signal to the group.

```rust
use std::os::unix::process::CommandExt;

Command::new("sh")
    .args(["-c", cmd])
    .process_group(0)  // New process group
    .spawn()?;
```

**Warning signs:** Orphaned processes after timeout; `sleep` or background processes lingering in tests.

**Note:** `process_group` is Unix-only. This is acceptable for v0.1.0 (macOS/Linux target). If Windows support is added later, use `CREATE_NEW_PROCESS_GROUP` via `std::os::windows::process::CommandExt`.

### Pitfall 4: Non-UTF-8 Process Output

**What goes wrong:** `String::from_utf8(output.stdout)` returns `Err` because the command produced binary or non-UTF-8 bytes.

**Why it happens:** Commands may emit binary content, locale-specific encodings, or ANSI escape sequences with unusual byte patterns.

**How to avoid:** Always use `String::from_utf8_lossy()` which replaces invalid UTF-8 with the Unicode replacement character (U+FFFD). This was identified in Phase 3 research.

**Warning signs:** Panics on `unwrap()` after `from_utf8()`.

### Pitfall 5: `ExitStatus::code()` Returns `None` on Signal Kill

**What goes wrong:** On Unix, if a process is killed by a signal (including SIGKILL from timeout), `code()` returns `None` — not an integer exit code.

**Why it happens:** Unix distinguishes between normal exits (with exit code) and signal termination. `ExitStatus::code()` only returns `Some(code)` for normal exits.

**How to avoid:** Handle `None` exit code as a distinct state. For timeouts, this is expected. Map `None` to the `exit_code: None` field on `GateResult` (already `Option<i32>`).

**Warning signs:** `unwrap()` on `status.code()` panicking in timeout tests.

### Pitfall 6: Inheriting Working Directory

**What goes wrong:** Gate commands run in the wrong directory because `current_dir()` wasn't set, so the child inherits the parent process's CWD.

**Why it happens:** `Command::new("sh")` inherits the parent's CWD by default. In a CLI, the CWD is wherever the user ran `assay`. In MCP, it's wherever the server was started.

**How to avoid:** GATE-04 requires explicit `working_dir`. The `evaluate()` function takes `working_dir: &Path` and always calls `.current_dir(working_dir)`. Never make it optional.

**Warning signs:** Tests passing locally but failing in CI (different CWD).

### Pitfall 7: Criterion `deny_unknown_fields` Blocks New Fields

**What goes wrong:** Adding `timeout` to `Criterion` is a breaking change for existing spec files because `Criterion` has `#[serde(deny_unknown_fields)]`.

**Why it happens:** `deny_unknown_fields` rejects any TOML key not in the struct. New optional fields are rejected in existing files that don't have them — wait, actually, **this is fine**. `deny_unknown_fields` rejects keys *present* in the file that aren't in the struct. Adding a new optional field to the struct with `#[serde(default)]` works: old files simply don't have the key, and `default` fills it in. The risk is the **opposite**: if old code reads a file with the new `timeout` key, it would reject it. But since we control the only consumer, this is safe.

**How to avoid:** Add `timeout` as `Option<u64>` with `#[serde(skip_serializing_if = "Option::is_none", default)]` — exactly the same pattern as `cmd`.

### Pitfall 8: Polling Interval Too Short or Too Long

**What goes wrong:** Polling `try_wait` in a tight loop wastes CPU. Polling too infrequently adds latency to short-running commands.

**Why it happens:** No single polling interval is optimal for both 10ms commands and 300s commands.

**How to avoid:** Use 50ms polling interval. This adds at most 50ms latency (imperceptible to humans) and sleeps ~95% of the time even for a 1-second command. For v0.1.0 this is acceptable; an adaptive backoff is unnecessary complexity.

## Code Examples

### Complete `evaluate_command` (Reference Implementation)

See Architecture Pattern 2 above for the full reference implementation.

### GateKind Enum Extension

```rust
// crates/assay-types/src/gate.rs

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum GateKind {
    /// A gate that runs a shell command and checks its exit code.
    Command {
        /// The shell command to execute.
        cmd: String,
    },

    /// A gate that always passes — useful for placeholder or manual gates.
    AlwaysPass,

    /// A gate that checks whether a file exists at the given path.
    FileExists {
        /// Path to check, relative to the working directory.
        path: String,
    },
}
```

### Criterion Timeout Field Addition

```rust
// crates/assay-types/src/criterion.rs

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,

    /// Optional timeout in seconds for this criterion's command.
    /// Overrides the global default but is overridden by CLI `--timeout`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,
}
```

### New AssayError Variants

```rust
// crates/assay-core/src/error.rs (additions)

/// A gate command failed to start.
#[error("gate execution failed for `{cmd}` in `{working_dir}`: {source}")]
GateExecution {
    cmd: String,
    working_dir: PathBuf,
    source: std::io::Error,
},

/// A spec was not found by name.
#[error("spec `{name}` not found in {specs_dir}")]
SpecNotFound {
    name: String,
    specs_dir: String,
},
```

### CLI Subcommand Structure

```rust
// In main.rs Command enum:

/// Run quality gates for a spec
Gate {
    #[command(subcommand)]
    command: GateCommand,
},

#[derive(Subcommand)]
enum GateCommand {
    /// Run all executable criteria for a spec
    Run {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Override timeout for all criteria (seconds)
        #[arg(long)]
        timeout: Option<u64>,
        /// Show evidence for all criteria, not just failures
        #[arg(short, long)]
        verbose: bool,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
}
```

### Truncation Helper (Tail-Biased)

```rust
/// Maximum bytes to retain from stdout/stderr capture.
const MAX_OUTPUT_BYTES: usize = 65_536; // 64 KB

/// Truncate output, keeping the tail (since errors appear at end).
/// Returns the original string if within budget.
fn truncate_output(output: &str, max_bytes: usize) -> &str {
    if output.len() <= max_bytes {
        return output;
    }
    // Find a valid UTF-8 boundary near the cut point
    let skip = output.len() - max_bytes;
    // Walk forward to the next char boundary
    let start = output.ceil_char_boundary(skip);
    &output[start..]
}
```

## State of the Art

### Process Execution in Rust (2025-2026)

The Rust ecosystem for subprocess management is mature:

- **`std::process`** covers ~90% of use cases. `Command::output()` is the go-to for simple capture, but has no timeout support.
- **`tokio::process`** provides async `Command` with `tokio::time::timeout()` — the cleanest timeout pattern. However, this project's gate evaluation is sync per GATE-08.
- **`wait-timeout` crate** (by alexcrichton) adds `wait_timeout()` to `Child`. MIT/Apache-2.0, minimal dependencies. Good but adds a dependency for ~15 lines of `try_wait` polling.
- **`command_timeout` crate** provides full timeout + process group kill. However, it has Linux-specific code (uses `nix` for `killpg`) and may not work on macOS.
- **`process_group(0)`** was stabilized in Rust 1.64 (2022). Available in the current Rust edition.

### Recommendation for v0.1.0

Use `std::process` directly with manual `try_wait` polling. The 15 lines of polling code are simpler than evaluating and maintaining an external dependency. Add `wait-timeout` only if the manual approach proves insufficient.

### Timeout Pattern: Sync vs Async

The `spawn_blocking` guidance (GATE-08) means: the gate evaluation function itself is sync (uses `std::process::Command`, `std::thread`). When called from the async MCP handler (Phase 8), it's wrapped in `tokio::task::spawn_blocking(move || gate::evaluate(...))`. This is a clean separation — the gate module has zero async code.

## Open Questions

### Resolved During Research

1. **`Command::output()` vs `spawn` + manual pipe reading** — `spawn` is required because `output()` blocks with no timeout mechanism. The STATE.md decision was correct about deadlock avoidance (output() is deadlock-free) but the decision doesn't account for the timeout requirement. Superseded by: spawn + reader threads + try_wait polling, which is also deadlock-free AND supports timeout.

2. **BufReader + byte budget vs unbounded capture** — Deferred to post-v0.1.0 (issue `.planning/issues/open/2026-03-01-phase7-streaming-capture.md` already tracks this). For v0.1.0, capture full output with `read_to_end`, truncate at the String level before storing in `GateResult`. The truncation limit (64KB default, hardcoded) is sufficient for v0.1.0.

3. **Process group for kill-tree** — Use `CommandExt::process_group(0)` (std, Unix-only, stable since 1.64). No need for `nix` or `libc` crates.

4. **`wait-timeout` crate vs manual try_wait** — Manual `try_wait` polling with 50ms sleep is simple enough. Skip the dependency.

### Remaining for Planner

1. **Exit code on timeout:** Should `exit_code` be `None` (killed by signal, no exit code) or a conventional value like `124` (matching GNU timeout)? Recommendation: `None` — it's accurate and the `GateResult.passed == false` already signals failure. The timeout is evident from duration_ms approaching the timeout value, or from the appended `[timed out after Ns]` message in stderr.

2. **`--timeout` scope:** Does CLI `--timeout 30` replace the global default for all criteria, or does it cap even per-criterion timeouts? Recommendation: It replaces the global default only. Per-criterion timeouts in spec files should still win. This matches the three-tier precedence naturally: CLI flag is the "global default override," not a hard cap.

3. **Minimum timeout floor:** Should we validate that timeout > 0? Recommendation: Yes, 1 second minimum. A 0-second timeout would instantly kill every command.

4. **AlwaysPass in summary count:** Should AlwaysPass criteria count toward "N/M passed" or be listed separately? Recommendation: Count them — they're criteria that pass. The distinction between AlwaysPass and Command is in `GateKind`, not in the pass/fail count.

5. **Truncation limit configurability:** Hardcode 64KB for v0.1.0 or make it configurable? Recommendation: Hardcode as `const MAX_OUTPUT_BYTES: usize = 65_536`. Configurability is a post-v0.1.0 concern (tracked in the streaming-capture issue).

## Sources

### Verified (HIGH confidence)

- [Rust std::process::Command documentation](https://doc.rust-lang.org/std/process/struct.Command.html) — `output()`, `spawn()`, `process_group()`
- [Rust std::process::Child documentation](https://doc.rust-lang.org/std/process/struct.Child.html) — `kill()`, `wait()`, `try_wait()`, `wait_with_output()`
- [Tokio spawn_blocking documentation](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) — Pattern for sync code in async context
- [Clap derive cookbook (git-like CLI)](https://docs.rs/clap/latest/clap/_cookbook/git_derive/index.html) — Nested subcommand patterns
- [Rust Issue #45572: piped stdout buffer deadlock](https://github.com/rust-lang/rust/issues/45572) — Confirms `output()` is deadlock-free, `spawn+wait` without pipe draining is not
- [Rust Issue #53402: wait_with_output ordering](https://github.com/rust-lang/rust/issues/53402) — Confirms `output()`/`wait_with_output()` reads pipes before waiting (deadlock-free)
- [Rust Issue #115241: Child::kill doesn't kill children](https://github.com/rust-lang/rust/issues/115241) — Confirms need for `process_group(0)` to kill child process trees
- Phase 3 Research (internal) — GateResult design decisions, non-UTF-8 handling, exit_code as Option<i32>
- Phase 6 Research (internal) — CLI output patterns, ANSI color conventions, NO_COLOR support

### Community Sources (MEDIUM confidence)

- [Rust Forum: Command timeouts](https://users.rust-lang.org/t/command-timeouts/35358) — try_wait polling and channel-based patterns
- [Rust Forum: Process with timeout in tokio](https://users.rust-lang.org/t/spawn-process-with-timeout-and-capture-output-in-tokio/128305) — Async timeout pattern (reference only)
- [wait-timeout crate](https://github.com/alexcrichton/wait-timeout) — ChildExt trait with wait_timeout(), MIT/Apache-2.0
- [Blog: Tokio Command Timeout](https://blog.juliobiason.me/code/tokio-command-timeout-test/) — select! + timeout pattern for async
- [Dealing with long-lived child processes in Rust](https://www.nikbrendler.com/rust-process-communication/) — Thread-based pipe management patterns

## Metadata

| Key                    | Value                                                |
| ---------------------- | ---------------------------------------------------- |
| phase                  | 7                                                    |
| domain                 | Process execution, timeout, CLI output               |
| new_workspace_deps     | 0                                                    |
| new_crate_deps         | 0                                                    |
| types_changes          | GateKind +FileExists variant, Criterion +timeout     |
| core_changes           | gate::evaluate + helpers, new error variants         |
| cli_changes            | Gate subcommand with Run, --timeout, --verbose, --json|
| risk_areas             | Process group kill (Unix-only), timeout polling       |
| estimated_complexity   | Medium-high (process lifecycle, concurrent I/O)       |
