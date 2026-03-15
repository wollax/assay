---
phase: 42
plan: 02
title: "Startup recovery scan for stale sessions"
wave: 2
depends_on: ["42-01"]
files_modified:
  - crates/assay-core/src/work_session.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/src/lib.rs
autonomous: true
source_issue: null

must_haves:
  truths:
    - "On startup, `.assay/sessions/` is scanned for stale `agent_running` sessions — each is marked `abandoned` with a recovery note and timestamp"
    - "Recovery scan handles corrupt session files gracefully — logs warning, skips file, continues scan"
    - "Recovery is idempotent — running twice produces the same result"
    - "Already-abandoned or non-AgentRunning sessions are skipped"
    - "Staleness is measured from the transition timestamp into AgentRunning, not from created_at"
  artifacts:
    - path: "crates/assay-core/src/work_session.rs"
      provides: "recover_stale_sessions, RecoverySummary, build_recovery_note"
    - path: "crates/assay-mcp/src/server.rs"
      provides: "Recovery call in serve() before binding transport"
  key_links:
    - from: "crates/assay-mcp/src/server.rs"
      to: "crates/assay-core/src/work_session.rs"
      via: "serve() calls recover_stale_sessions before AssayServer::new().serve()"
    - from: "crates/assay-core/src/work_session.rs"
      to: "crates/assay-types/src/lib.rs"
      via: "reads stale_threshold from SessionsConfig"
---

<objective>
Implement the startup recovery scan that detects and marks stale `agent_running` sessions as `abandoned`. Wire it into the MCP server startup so recovery runs before any tool call.

Purpose: When the MCP server crashes or is killed, sessions stuck in `agent_running` become orphans. Recovery ensures these are automatically cleaned up on next startup, preventing phantom sessions from accumulating.

Output: `recover_stale_sessions` function in assay-core, wired into the MCP server `serve()` function.
</objective>

<context>
@crates/assay-core/src/work_session.rs — persistence layer + convenience functions from PLAN-01
@crates/assay-types/src/lib.rs — Config with SessionsConfig (from PLAN-01)
@crates/assay-types/src/work_session.rs — SessionPhase, PhaseTransition types
@crates/assay-mcp/src/server.rs — serve() function at bottom of file
@crates/assay-mcp/src/lib.rs — re-exports serve()
@.planning/phases/pending/42-session-recovery/42-RESEARCH.md — recovery scan implementation details
@.planning/phases/pending/42-session-recovery/42-01-SUMMARY.md — PLAN-01 output
</context>

<tasks>

### Task 1: Implement recovery scan in assay-core

<files>
crates/assay-core/src/work_session.rs
</files>

<action>

Add the recovery scan implementation after the convenience functions (from PLAN-01) and before the `#[cfg(test)]` block.

**1a. Add `RecoverySummary` type:**

```rust
/// Summary of a recovery scan for stale sessions.
#[derive(Debug, Default)]
pub struct RecoverySummary {
    /// Number of sessions recovered (transitioned to Abandoned).
    pub recovered: usize,
    /// Number of sessions skipped (wrong phase, not stale, data inconsistency).
    pub skipped: usize,
    /// Number of errors encountered (corrupt files, save failures).
    pub errors: usize,
}
```

**1b. Add `build_recovery_note` helper:**

Format: `"Recovered on startup: stale for 3h 12m (threshold: 1h). Host: macbook.local, PID: 12345"`

```rust
fn build_recovery_note(
    stale_duration: chrono::Duration,
    threshold: chrono::Duration,
    hostname: &str,
    pid: u32,
) -> String {
    let format_duration = |d: chrono::Duration| -> String {
        let h = d.num_hours();
        let m = d.num_minutes() % 60;
        if h > 0 {
            format!("{h}h {m}m")
        } else {
            format!("{m}m")
        }
    };

    format!(
        "Recovered on startup: stale for {} (threshold: {}). Host: {}, PID: {}",
        format_duration(stale_duration),
        format_duration(threshold),
        hostname,
        pid
    )
}
```

**1c. Implement `recover_stale_sessions`:**

```rust
/// Scan for stale `agent_running` sessions and mark them as abandoned.
///
/// Called on MCP server startup before any tool call. Sessions in `AgentRunning`
/// phase that are older than `stale_threshold_secs` (measured from the transition
/// timestamp into `AgentRunning`) are transitioned to `Abandoned` with a recovery
/// note containing hostname, PID, and timing details.
///
/// Corrupt session files are logged and skipped — one bad file does not block
/// recovery of other sessions. The scan is capped at 100 sessions (oldest first).
pub fn recover_stale_sessions(
    assay_dir: &Path,
    stale_threshold_secs: u64,
) -> RecoverySummary {
    // implementation per research doc
}
```

Key implementation requirements (from RESEARCH.md):

1. Call `list_sessions(assay_dir)` — if it fails, log warning and return empty summary.
2. Cap at 100 candidates (`.take(100)`) — ULID sort is chronological, oldest first.
3. For each session ID, `load_session` — on error, `tracing::warn!`, increment `errors`, continue.
4. Skip if `session.phase != SessionPhase::AgentRunning`.
5. Find the transition timestamp into `AgentRunning` by scanning `session.transitions` in reverse for `t.to == SessionPhase::AgentRunning`. If no such transition exists, `tracing::warn!` (data inconsistency), increment `skipped`, continue.
6. Compute age: `Utc::now() - entered_at`. If `age < threshold`, skip (not stale).
7. Build recovery note with `build_recovery_note(age, threshold, hostname, pid)`.
8. Call `transition_session(&mut session, SessionPhase::Abandoned, "startup_recovery", Some(&note))`. On error, `tracing::warn!`, increment `skipped`, continue.
9. Call `save_session(assay_dir, &session)`. On error, `tracing::warn!`, increment `errors`, continue. On success, `tracing::warn!` per recovered session (with session_id, spec_name, stale duration), increment `recovered`.
10. After loop, if `recovered > 0`, `tracing::info!` summary (recovered, skipped, errors).

Hostname: `hostname::get().map(|h| h.to_string_lossy().to_string()).unwrap_or_else(|_| "unknown".to_string())`

PID: `std::process::id()`

**1d. Add tests:**

1. **recover_no_sessions**: Empty sessions dir returns `RecoverySummary { recovered: 0, skipped: 0, errors: 0 }`.
2. **recover_no_sessions_dir**: Non-existent assay_dir returns empty summary (no panic).
3. **recover_stale_session**: Create a session, transition to `AgentRunning` with a timestamp 2 hours ago (manipulate the `PhaseTransition` timestamp directly after creation), save it, run recovery with 1-hour threshold. Verify session on disk is now `Abandoned`, recovery note contains "Recovered on startup", and summary shows `recovered: 1`.
4. **recover_skips_non_agent_running**: Create sessions in `Created`, `GateEvaluated`, `Completed`, `Abandoned` phases. Run recovery. Verify none are modified, `recovered: 0`.
5. **recover_skips_fresh_session**: Create a session in `AgentRunning` with transition timestamp just now. Run recovery with 1-hour threshold. Verify not recovered.
6. **recover_corrupt_file**: Write invalid JSON to a `.json` file in sessions dir. Create a valid stale session. Run recovery. Verify the valid one is recovered (`recovered: 1`) and corrupt one is counted (`errors: 1`).
7. **recover_idempotent**: Run recovery twice on the same stale session. First run: `recovered: 1`. Second run: `recovered: 0` (already abandoned, skipped).
8. **recover_missing_transition_record**: Create a session with `phase: AgentRunning` but empty `transitions` vec (data inconsistency). Run recovery. Verify `skipped: 1`, not panicked.
9. **build_recovery_note_format**: Test the note format with known values: 3h12m stale, 1h threshold, "testhost", PID 12345. Assert the string matches expected format.
10. **build_recovery_note_minutes_only**: Test with 45m stale, 30m threshold. Assert no "0h" prefix.

For tests that need to manipulate timestamps, directly construct the `PhaseTransition` struct and push it to `session.transitions`, then set `session.phase = SessionPhase::AgentRunning` before saving. This avoids needing to mock `Utc::now()`.

</action>

<verify>
`rtk cargo test -p assay-core -- work_session` — all new and existing tests pass. `rtk cargo test -p assay-core -- recovery` specifically passes.
</verify>

<done>
`recover_stale_sessions` correctly identifies stale `AgentRunning` sessions, marks them `Abandoned` with recovery notes, handles corrupt files gracefully, and is idempotent. All edge cases tested.
</done>

### Task 2: Wire recovery into MCP server startup

<files>
crates/assay-mcp/src/server.rs
crates/assay-mcp/src/lib.rs
</files>

<action>

**2a. Add recovery call to `serve()` in `server.rs`:**

The current `serve()` function (around line 2016) is:

```rust
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    let service = AssayServer::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}
```

Insert recovery between the info log and the service creation:

```rust
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    // Recover stale sessions before accepting any tool calls.
    // Runs synchronously — capped at 100 sessions, completes in <50ms.
    if let Ok(cwd) = std::env::current_dir() {
        let assay_dir = cwd.join(".assay");
        if assay_dir.join("sessions").is_dir() {
            let stale_threshold = load_recovery_threshold(&cwd);
            let summary = assay_core::work_session::recover_stale_sessions(
                &assay_dir,
                stale_threshold,
            );
            if summary.recovered > 0 || summary.errors > 0 {
                tracing::info!(
                    recovered = summary.recovered,
                    skipped = summary.skipped,
                    errors = summary.errors,
                    "session recovery scan complete"
                );
            }
        }
    }

    let service = AssayServer::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}
```

**2b. Add `load_recovery_threshold` helper in `server.rs`:**

Add a private helper near the bottom of `server.rs` (before `serve()`):

```rust
/// Load the stale session threshold from config, falling back to 3600 seconds.
fn load_recovery_threshold(cwd: &Path) -> u64 {
    let config_path = cwd.join(".assay").join("config.toml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return 3600,
    };
    let config: assay_types::Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return 3600,
    };
    config
        .sessions
        .map(|s| s.stale_threshold)
        .unwrap_or(3600)
}
```

This is deliberately simple and fault-tolerant: if config can't be read or parsed, fall back to the default. Recovery should never fail to run because of a config issue.

**2c. Update module doc comment in `server.rs`:**

The top-of-file doc comment currently says "seventeen tools". No new tools are added in this plan — verify the count is still accurate. If it was updated in PLAN-01, leave it as-is.

**2d. Verify `lib.rs` re-export:**

The `serve()` function in `lib.rs` just delegates to `server::serve()`. No changes needed here, but verify `toml` is in `assay-mcp`'s dependencies (it's needed for `load_recovery_threshold`). If not present, add `toml.workspace = true` to `crates/assay-mcp/Cargo.toml`. Also ensure `assay_types` is in scope for the Config import — check existing imports.

</action>

<verify>
`rtk cargo build -p assay-mcp` compiles. `rtk cargo clippy -p assay-mcp` — no warnings. `just ready` passes.
</verify>

<done>
Recovery scan runs on MCP server startup before any tool call. Reads threshold from `[sessions].stale_threshold` in config with 3600-second default. Recovery failures are logged, never fatal.
</done>

</tasks>

<verification>
- [ ] `just build` compiles without errors
- [ ] `just test` — all new and existing tests pass
- [ ] `just lint` — no clippy warnings
- [ ] `just fmt-check` — formatting is clean
- [ ] `just ready` passes
- [ ] `recover_stale_sessions` correctly marks stale `AgentRunning` sessions as `Abandoned`
- [ ] Recovery note includes hostname, PID, stale duration, and threshold
- [ ] Corrupt session files are logged and skipped — do not block recovery
- [ ] Non-AgentRunning sessions are not touched by recovery
- [ ] Fresh AgentRunning sessions (under threshold) are not touched
- [ ] Recovery is idempotent — second run recovers nothing
- [ ] Staleness measured from AgentRunning transition timestamp, not created_at
- [ ] Sessions with missing transition records are warned and skipped
- [ ] Recovery scan capped at 100 sessions
- [ ] `serve()` calls recovery before binding MCP transport
- [ ] Threshold read from `[sessions].stale_threshold` config with 3600 default
</verification>

<success_criteria>
1. On startup, `.assay/sessions/` is scanned for stale `agent_running` sessions — each is marked `abandoned` with a recovery note and timestamp
2. Recovery scan handles corrupt session files gracefully — logs warning, skips file, continues scan
3. Recovery is wired into MCP server startup, runs before any tool call
4. Staleness threshold is configurable via `[sessions].stale_threshold` in `assay.toml`
</success_criteria>

<output>
After completion, create `.planning/phases/pending/42-session-recovery/42-02-SUMMARY.md`
</output>
