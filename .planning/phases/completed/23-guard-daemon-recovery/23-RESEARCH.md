# Phase 23: Guard Daemon & Recovery - Research

**Completed:** 2026-03-06
**Confidence:** HIGH (all core patterns validated against codebase and upstream docs)

---

## Standard Stack

### File system watching: `notify` crate

- **Crate:** `notify` v7.0.0 (latest stable; v9.0.0-rc.2 exists but is pre-release)
- **Use `notify` v7.0.0** — stable, battle-tested, uses kqueue on macOS and inotify on Linux natively
- The `RecommendedWatcher` auto-selects the optimal backend per platform
- **Do NOT use** `notify-debouncer-full` (v0.7.0 depends on notify v9 pre-release); manual debouncing is trivial for this use case (single file, sub-second detection)
- Add to workspace `[workspace.dependencies]`: `notify = "7"`

```rust
use notify::{Watcher, RecursiveMode, Event, EventKind, Config};

let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::RecommendedWatcher::new(tx, Config::default())?;
watcher.watch(session_path.as_ref(), RecursiveMode::NonRecursive)?;

// In event loop:
for event in rx.try_recv() {
    if matches!(event.kind, EventKind::Modify(_)) {
        // Session file changed — check thresholds
    }
}
```

### Async runtime: `tokio` (already in workspace)

- Already a workspace dependency at v1 with `features = ["full"]`
- Use `tokio::signal::unix::signal(SignalKind::interrupt())` and `SignalKind::terminate()` for SIGINT/SIGTERM handling
- Use `tokio::time::interval()` for polling loop
- Use `tokio::select!` to multiplex polling, file watching, and signal handling

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sigint = signal(SignalKind::interrupt())?;
let mut sigterm = signal(SignalKind::terminate())?;
let mut poll_interval = tokio::time::interval(Duration::from_secs(poll_secs));

loop {
    tokio::select! {
        _ = sigint.recv() => { /* checkpoint + shutdown */ }
        _ = sigterm.recv() => { /* checkpoint + shutdown */ }
        _ = poll_interval.tick() => { /* check thresholds */ }
        Ok(event) = watcher_rx.recv_async() => { /* reactive check */ }
    }
}
```

### Logging: `tracing` + `tracing-subscriber` (already in workspace)

- Already workspace dependencies
- Use `tracing-appender` for file-based log output — add `tracing-appender = "0.2"` to workspace
- Guard daemon logs to `.assay/guard/guard.log` using a non-blocking file appender
- Use JSON format (via `tracing_subscriber::fmt::format::json()`) for structured log output — easier to filter programmatically with `assay context guard logs --level`

### PID file management: manual implementation

- PID files are simple enough to hand-roll (write PID, check on startup, clean up on exit)
- Use `libc` (already in workspace at v0.2) for `kill(pid, 0)` to check if process is alive
- Store at `.assay/guard/guard.pid`

### Config parsing: `toml` + `serde` (already in workspace)

- Both already workspace dependencies
- Add `GuardConfig` struct to `assay-types` Config, gated behind `Option<GuardConfig>` like `GatesConfig`

---

## Architecture Patterns

### 1. Module location: `crates/assay-core/src/guard/`

New module `guard` in assay-core with this structure:

```
crates/assay-core/src/guard/
  mod.rs          — public API: start_guard, stop_guard, guard_status
  config.rs       — GuardConfig deserialization, threshold types, defaults
  daemon.rs       — main event loop (tokio::select! multiplexing)
  pid.rs          — PID file create/check/remove
  circuit_breaker.rs — CircuitBreaker state machine
  thresholds.rs   — threshold evaluation (soft/hard, token/size)
  watcher.rs      — notify watcher setup and event filtering
```

Register in `crates/assay-core/src/lib.rs`:
```rust
/// Guard daemon: background context protection.
pub mod guard;
```

### 2. CLI integration: extend existing `ContextCommand` enum

The CLI already has `enum ContextCommand` with Diagnose, List, Prune variants. Add:

```rust
/// Start, stop, and monitor the context guard daemon
Guard {
    #[command(subcommand)]
    command: GuardCommand,
},
```

With `GuardCommand` having `Start`, `Stop`, `Status`, `Logs` subcommands.

### 3. Daemon lifecycle: foreground process with daemonization

The guard runs as a **foreground tokio process** by default (for debugging/development), with `--daemon` flag to fork into background using `libc::fork()` + `libc::setsid()`. The `start` subcommand:

1. Checks PID file — if exists and process alive, error "already running"
2. Auto-discovers session file via `context::find_session_dir()` + `context::resolve_session()`
3. Writes PID file
4. Enters main event loop
5. On exit (signal or circuit breaker trip): final checkpoint, remove PID file

### 4. Threshold evaluation pattern

```rust
pub struct ThresholdResult {
    pub level: ThresholdLevel,  // None | Soft | Hard
    pub trigger: ThresholdTrigger,  // TokenBased { pct, tokens } | FileSizeBased { bytes }
}

pub enum ThresholdLevel {
    None,
    Soft,
    Hard,
}
```

Evaluation reads the session file tail (reuse `quick_token_estimate()`) and file metadata for size. Both checks run; whichever fires first (higher level wins) determines the action.

### 5. Circuit breaker state machine

```rust
pub struct CircuitBreaker {
    max_recoveries: u32,       // default 3
    window: Duration,          // default 10 minutes
    recovery_timestamps: VecDeque<Instant>,
    current_tier: PrescriptionTier,  // escalates: Gentle -> Standard -> Aggressive
    tripped: bool,
}

impl CircuitBreaker {
    fn record_recovery(&mut self) -> RecoveryDecision;  // Ok(tier) | Tripped
    fn reset_if_quiet(&mut self);  // called when window expires with no recoveries
}
```

**Discretion decisions:**
- Circuit breaker **halts the daemon entirely** when tripped (writes final checkpoint, removes PID file, exits with code 2). Rationale: a cooldown mode adds complexity and the user should investigate why context is growing uncontrollably.
- Escalating prescription level **resets after a quiet period** (when `window` elapses with no recoveries, tier resets to Gentle). Rationale: avoids requiring manual restart after a burst subsides.

### 6. Recovery action flow

```
Soft threshold:
  1. Checkpoint (discretion: YES, checkpoint before soft prune — it's cheap and protects state)
  2. prune_session(session_path, tier.strategies(), tier, execute=true, backup_dir)
  3. Log result, emit stderr summary
  4. Record recovery in circuit breaker

Hard threshold:
  1. Checkpoint (always)
  2. prune_session(session_path, tier.strategies(), tier, execute=true, backup_dir)
  3. Attempt session reload (see Session Reload section)
  4. Log result, emit stderr summary
  5. Record recovery in circuit breaker
```

### 7. Config schema addition

Add to `assay-types` Config struct:

```rust
/// Guard daemon configuration.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub guard: Option<GuardConfig>,
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardConfig {
    /// Polling interval in seconds. Default: 5.
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    /// Soft threshold as percentage of context window. Default: 60.
    #[serde(default = "default_soft_threshold")]
    pub soft_threshold_pct: f64,

    /// Hard threshold as percentage of context window. Default: 80.
    #[serde(default = "default_hard_threshold")]
    pub hard_threshold_pct: f64,

    /// File size soft threshold in bytes. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_threshold_bytes: Option<u64>,

    /// File size hard threshold in bytes. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_threshold_bytes: Option<u64>,

    /// Maximum recoveries before circuit breaker trips. Default: 3.
    #[serde(default = "default_max_recoveries")]
    pub max_recoveries: u32,

    /// Circuit breaker time window in seconds. Default: 600 (10 minutes).
    #[serde(default = "default_recovery_window")]
    pub recovery_window: u64,
}
```

### 8. State directory

**Discretion decision:** Use `.assay/guard/` (project-local). Rationale:
- Consistent with existing `.assay/checkpoints/`, `.assay/backups/` patterns
- Multiple projects can run independent guards without collision
- No XDG dependency needed
- Files: `guard.pid`, `guard.log`

---

## Don't Hand-Roll

| Problem | Use Instead |
|---|---|
| File system watching (kqueue/inotify) | `notify` v7 `RecommendedWatcher` |
| Async signal handling | `tokio::signal::unix::signal()` |
| Async timer/interval | `tokio::time::interval()` |
| Async multiplexing | `tokio::select!` |
| TOML config parsing | `toml` + `serde` (workspace deps) |
| Structured logging | `tracing` + `tracing-appender` |
| Atomic file writes | `tempfile::NamedTempFile` (existing pattern in checkpoint/pruning) |
| Token estimation | `context::quick_token_estimate()` (existing) |
| Session discovery | `context::find_session_dir()` + `context::resolve_session()` (existing) |
| Pruning pipeline | `context::pruning::prune_session()` (existing) |
| Checkpoint save | `checkpoint::save_checkpoint()` (existing) |
| Team state extraction | `checkpoint::extract_team_state()` (existing) |

---

## Common Pitfalls

### P1: notify watcher dropped too early (HIGH confidence)
The `Watcher` must be kept alive for the duration of monitoring. If stored in a local variable that goes out of scope, events stop. Store it in the daemon's main struct or keep it alive across the `tokio::select!` loop.

### P2: PID file stale after unclean exit (HIGH confidence)
If the daemon crashes or is `kill -9`'d, the PID file remains. On startup, always check if the PID in the file is actually alive using `kill(pid, 0)`. If not, treat as stale and overwrite.

### P3: Session file locked during write (MEDIUM confidence)
The pruning engine uses atomic temp+rename via `tempfile::NamedTempFile::persist()`. This is safe on Unix (rename is atomic). However, the watcher may fire events for the temp file creation in the same directory. Filter events to only react to the actual session file path, not temp files.

### P4: Token estimate stale after prune (HIGH confidence)
After pruning, `quick_token_estimate()` reads the tail of the file for usage data. The pruned file may have lost the last assistant entry with usage data. The guard should re-check thresholds after pruning to confirm the prune was effective, and not enter a tight loop if the token count didn't decrease.

### P5: Race between polling and reactive watcher (MEDIUM confidence)
Both polling and the file watcher can detect the same threshold crossing. Use a simple timestamp-based debounce (e.g., skip if last action was < 2 seconds ago) to prevent double-firing.

### P6: fork() and tokio runtime (HIGH confidence)
If using `libc::fork()` for daemonization, the fork must happen BEFORE the tokio runtime is created. Forking after runtime creation corrupts internal state. The correct order is: parse args -> fork (if `--daemon`) -> create runtime -> enter event loop.

### P7: Ctrl+C during checkpoint write (MEDIUM confidence)
The checkpoint save uses atomic writes internally. The SIGINT handler should set a flag, let the current operation complete, then do the final checkpoint. Use `tokio::select!` with the signal future, which naturally lets the current branch complete.

### P8: Config deny_unknown_fields and backward compatibility (HIGH confidence)
The existing `Config` struct uses `#[serde(deny_unknown_fields)]`. Adding the new `guard` field to `Config` is fine (it's optional), but existing config files that don't have `[guard]` will parse correctly because it defaults to `None`. No migration needed.

---

## Session Reload Research

### What Claude Code supports (HIGH confidence)

Based on the existing hooks system in `plugins/claude-code/hooks/hooks.json`, Claude Code supports these hook events:
- `PostToolUse` (with tool name matcher)
- `PreCompact`
- `Stop`

There is **no** hook event for "session started" or programmatic session reload. Claude Code does not expose an API or IPC mechanism for external tools to trigger `/compact` or session reload.

### Recommendation: Log-only for session reload

For hard threshold recovery, the guard daemon should:
1. Perform the full prune + checkpoint
2. Log a clear message: "Hard threshold exceeded. Session pruned. Consider running `/compact` in Claude Code."
3. Optionally write a marker file (`.assay/guard/reload-needed`) that a future PreToolUse hook could check

This is the honest approach given Claude Code's current architecture. The guard protects context by pruning the JSONL file directly, which is effective even without explicit session reload. Claude Code naturally re-reads the session file on subsequent turns.

---

## Code Examples

### Daemon main loop skeleton

```rust
pub async fn run_guard(
    session_path: PathBuf,
    config: GuardConfig,
    assay_dir: PathBuf,
) -> crate::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let _watcher = setup_watcher(&session_path, tx.clone())?;

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut poll = tokio::time::interval(Duration::from_secs(config.poll_interval));
    let mut breaker = CircuitBreaker::new(config.max_recoveries, Duration::from_secs(config.recovery_window));
    let mut last_action = Instant::now();

    loop {
        tokio::select! {
            _ = sigint.recv() => {
                tracing::info!("SIGINT received, writing final checkpoint");
                final_checkpoint(&assay_dir, &session_path).await;
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("SIGTERM received, writing final checkpoint");
                final_checkpoint(&assay_dir, &session_path).await;
                break;
            }
            _ = poll.tick() => {
                if last_action.elapsed() > Duration::from_secs(2) {
                    if let Some(action) = evaluate_thresholds(&session_path, &config)? {
                        execute_recovery(action, &mut breaker, &session_path, &assay_dir, &config).await?;
                        last_action = Instant::now();
                    }
                }
            }
            Some(_) = rx.recv() => {
                if last_action.elapsed() > Duration::from_secs(2) {
                    if let Some(action) = evaluate_thresholds(&session_path, &config)? {
                        execute_recovery(action, &mut breaker, &session_path, &assay_dir, &config).await?;
                        last_action = Instant::now();
                    }
                }
            }
        }
    }
    Ok(())
}
```

### PID file check

```rust
pub fn is_guard_running(pid_path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(pid_path) else { return false };
    let Ok(pid) = content.trim().parse::<i32>() else { return false };
    // kill(pid, 0) checks existence without sending a signal
    unsafe { libc::kill(pid, 0) == 0 }
}
```

### Circuit breaker

```rust
pub fn record_recovery(&mut self) -> RecoveryDecision {
    let now = Instant::now();
    // Remove timestamps outside the window
    while self.recovery_timestamps.front().is_some_and(|t| now.duration_since(*t) > self.window) {
        self.recovery_timestamps.pop_front();
    }
    self.recovery_timestamps.push_back(now);

    if self.recovery_timestamps.len() as u32 >= self.max_recoveries {
        self.tripped = true;
        return RecoveryDecision::Tripped;
    }

    // Escalate tier based on recovery count in window
    self.current_tier = match self.recovery_timestamps.len() {
        1 => PrescriptionTier::Gentle,
        2 => PrescriptionTier::Standard,
        _ => PrescriptionTier::Aggressive,
    };

    RecoveryDecision::Proceed(self.current_tier)
}
```

### Threshold evaluation

```rust
pub fn evaluate_thresholds(session_path: &Path, config: &GuardConfig) -> crate::Result<Option<ThresholdLevel>> {
    let file_size = std::fs::metadata(session_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // Token-based check
    let token_level = if let Ok(Some(usage)) = quick_token_estimate(session_path) {
        let context_tokens = usage.context_tokens();
        let available = DEFAULT_CONTEXT_WINDOW.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
        let pct = (context_tokens as f64 / available as f64) * 100.0;

        if pct >= config.hard_threshold_pct { ThresholdLevel::Hard }
        else if pct >= config.soft_threshold_pct { ThresholdLevel::Soft }
        else { ThresholdLevel::None }
    } else {
        ThresholdLevel::None
    };

    // File-size check
    let size_level = match (config.hard_threshold_bytes, config.soft_threshold_bytes) {
        (Some(hard), _) if file_size >= hard => ThresholdLevel::Hard,
        (_, Some(soft)) if file_size >= soft => ThresholdLevel::Soft,
        _ => ThresholdLevel::None,
    };

    // Take the higher level
    let level = std::cmp::max(token_level, size_level);
    Ok(if level == ThresholdLevel::None { None } else { Some(level) })
}
```

### notify watcher setup

```rust
fn setup_watcher(
    session_path: &Path,
    tx: tokio::sync::mpsc::Sender<()>,
) -> crate::Result<notify::RecommendedWatcher> {
    let target = session_path.to_path_buf();
    let mut watcher = notify::RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_))
                    && event.paths.iter().any(|p| p == &target)
                {
                    let _ = tx.blocking_send(());
                }
            }
        },
        Config::default(),
    )?;

    watcher.watch(session_path.parent().unwrap_or(Path::new(".")), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
```

### stderr summary format (discretion)

```
[guard] Soft threshold (62.3%) — gentle prune saved 12.4 KB (3 strategies applied)
[guard] Hard threshold (83.1%) — aggressive prune saved 48.2 KB, checkpoint saved
[guard] Circuit breaker tripped (3 recoveries in 8m) — final checkpoint saved, exiting
```

---

## Existing Codebase Integration Points

### Functions to reuse directly

| Function | Location | Purpose in guard |
|---|---|---|
| `context::find_session_dir()` | `context/discovery.rs` | Auto-discover session directory |
| `context::resolve_session()` | `context/discovery.rs` | Find latest session file |
| `context::quick_token_estimate()` | `context/tokens.rs` | Fast token count from file tail |
| `context::pruning::prune_session()` | `context/pruning/mod.rs` | Execute pruning pipeline |
| `checkpoint::save_checkpoint()` | `checkpoint/persistence.rs` | Save team checkpoint |
| `checkpoint::extract_team_state()` | `checkpoint/extractor.rs` | Extract state for checkpoint |
| `PrescriptionTier::strategies()` | `assay-types/context.rs` | Get strategies for a tier |

### Constants to reuse

| Constant | Location | Value |
|---|---|---|
| `DEFAULT_CONTEXT_WINDOW` | `context/tokens.rs` | 200,000 |
| `SYSTEM_OVERHEAD_TOKENS` | `context/tokens.rs` | 21,000 |

### Config struct to extend

The `Config` struct in `assay-types/src/lib.rs` (line 109) needs a new optional `guard` field. The existing pattern with `GatesConfig` (line 133) shows exactly how to add it: `Option<GuardConfig>` with `serde(default, skip_serializing_if = "Option::is_none")`.

### CLI structure to extend

The `ContextCommand` enum in `assay-cli/src/main.rs` (line 275) currently has `Diagnose`, `List`, `Prune`. Add `Guard { command: GuardCommand }` following the same pattern as `GateCommand`.

### Error variants to add

Add to `AssayError` in `crates/assay-core/src/error.rs`:
- `GuardAlreadyRunning { pid: u32 }`
- `GuardNotRunning`
- `GuardCircuitBreakerTripped { recoveries: u32, window_secs: u64 }`

### New workspace dependencies needed

| Crate | Version | Purpose |
|---|---|---|
| `notify` | `7` | File system watching (kqueue/inotify) |
| `tracing-appender` | `0.2` | File-based log output |

---

## Testing Strategy Notes

### Unit-testable components (no I/O)
- `CircuitBreaker` state machine (all transitions, edge cases)
- `ThresholdResult` evaluation logic (pure function given usage data + config)
- `GuardConfig` deserialization from TOML strings
- PID file content parsing

### Integration-testable (with tempdir)
- PID file create/check/remove lifecycle
- Watcher setup and event detection (write to temp file, verify event received)
- Full recovery flow: create session file -> exceed threshold -> verify prune executed

### Not easily testable (acceptance/manual)
- Daemon fork behavior
- Signal handling (SIGINT/SIGTERM)
- Real Claude Code session file watching

---

*Phase: 23-guard-daemon-recovery*
*Research completed: 2026-03-06*
