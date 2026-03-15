# Phase 42: Session Recovery & Internal API — Research

**Researched:** 2026-03-15
**Confidence:** HIGH (all findings verified against codebase)

## Standard Stack

| Need | Use | Why |
|------|-----|-----|
| Hostname retrieval | `hostname::get()` from `hostname` crate | `std::env` has no hostname API; `libc::gethostname` works but requires unsafe + buffer management. The `hostname` crate is a thin safe wrapper (~50 lines, no transitive deps beyond `libc` on unix). Already idiomatic in the Rust ecosystem. |
| PID retrieval | `std::process::id()` | Returns `u32`, stable, no dependencies needed. |
| Duration formatting | `chrono::Duration` arithmetic | Already a workspace dependency. Use `Utc::now() - transition_timestamp` then format as `Xh Ym`. |
| Config parsing | Existing `assay_types::Config` + `serde` | Add `[sessions]` section following the `[gates]`, `[guard]`, `[worktree]` pattern. |
| Atomic writes | Existing `tempfile::NamedTempFile` + rename | Already proven in `save_session`. |
| Logging | `tracing` crate (workspace dep) | `warn!` per recovery, `info!` summary. |

### New dependency: `hostname`

- Crate: [`hostname`](https://crates.io/crates/hostname) v0.4
- Size: Minimal (~50 lines of code, wraps `libc::gethostname` on unix, `GetComputerNameExW` on windows)
- Dependencies: `libc` on unix (already in workspace), `windows-targets` on windows
- Alternatives considered:
  - Raw `libc::gethostname`: Works, but requires `unsafe`, manual buffer allocation, and `CStr` conversion. Not worth it when a well-maintained crate exists.
  - `std::env::var("HOSTNAME")`: Unreliable — not always set, varies by shell configuration.
- **Recommendation:** Add `hostname = "0.4"` to workspace dependencies, use in `assay-core`.

## Architecture Patterns

### 1. Recovery scan at MCP server init

The `AssayServer::new()` is currently synchronous (returns `Self`). Recovery needs filesystem I/O. Two options:

**Option A — Synchronous in `new()`**: Recovery is fast (list dir + read JSON + write JSON per stale session). The scan is capped at 100 sessions. File I/O for ~100 small JSON files completes in <50ms. The MCP server starts once and runs long — startup cost is negligible.

**Option B — Async in `serve()`**: Add recovery after `AssayServer::new()` but before `.serve(stdio())`. This keeps `new()` pure and puts I/O in the async context.

**Recommendation: Option B** — add a `pub fn recover_stale_sessions(assay_dir: &Path) -> RecoverySummary` function in `assay_core::work_session` called from `serve()` before binding the transport. This keeps `AssayServer::new()` free of side effects and makes recovery independently testable.

```rust
// In crates/assay-mcp/src/server.rs
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    // Recovery runs before any tool call
    if let Ok(cwd) = std::env::current_dir() {
        let assay_dir = cwd.join(".assay");
        let summary = assay_core::work_session::recover_stale_sessions(&assay_dir, &config);
        if summary.recovered > 0 {
            tracing::info!(
                recovered = summary.recovered,
                skipped = summary.skipped,
                "Recovered stale sessions on startup"
            );
        }
    }

    let service = AssayServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### 2. `with_session` helper pattern

The load-mutate-save pattern appears in every MCP session handler (`session_update`, and will appear in Phase 43's `gate_evaluate`). Extract to a reusable helper:

```rust
/// Load a session, apply a mutation, and save atomically.
///
/// Returns the mutated session on success. The closure receives a mutable
/// reference and can return an error to abort without saving.
pub fn with_session<F>(assay_dir: &Path, session_id: &str, mutate: F) -> Result<WorkSession>
where
    F: FnOnce(&mut WorkSession) -> Result<()>,
{
    let mut session = load_session(assay_dir, session_id)?;
    mutate(&mut session)?;
    save_session(assay_dir, &session)?;
    Ok(session)
}
```

**Key design decisions:**
- Returns `Result<WorkSession>` so callers get the final state (needed for response building)
- Closure returns `Result<()>` so transition errors abort the save
- No locking — file-level atomicity via tempfile-rename is sufficient for the single-writer model
- Aligns with existing `save_session`'s tempfile-then-rename pattern

### 3. Convenience functions for Phase 43

Based on Phase 43's flow (`gate_evaluate` computes diff, spawns evaluator, parses results, persists record), the internal API needs:

```rust
/// Create a session and immediately transition to AgentRunning.
/// Saves atomically. Returns the saved session.
pub fn start_session(
    assay_dir: &Path,
    spec_name: &str,
    worktree_path: PathBuf,
    agent_command: &str,
    agent_model: Option<&str>,
) -> Result<WorkSession>

/// Transition to GateEvaluated and link a gate run ID.
/// Convenience for the common gate_evaluate flow.
pub fn record_gate_result(
    assay_dir: &Path,
    session_id: &str,
    gate_run_id: &str,
    trigger: &str,
    notes: Option<&str>,
) -> Result<WorkSession>

/// Mark a session as completed after successful evaluation.
pub fn complete_session(
    assay_dir: &Path,
    session_id: &str,
    notes: Option<&str>,
) -> Result<WorkSession>

/// Mark a session as abandoned with recovery context.
pub fn abandon_session(
    assay_dir: &Path,
    session_id: &str,
    reason: &str,
) -> Result<WorkSession>
```

All convenience functions compose `with_session` + `transition_session` internally. They are separate functions (not a builder) because Phase 43's flow is linear and each step maps to a discrete operation.

### 4. Config shape for staleness threshold

Follow the existing pattern of config sections (`[gates]`, `[guard]`, `[worktree]`):

```toml
[sessions]
# Staleness threshold for recovery sweep (seconds). Default: 3600 (1 hour).
stale_threshold = 3600
```

In types:

```rust
/// Session management configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionsConfig {
    /// Staleness threshold in seconds for recovery sweep.
    /// Sessions in `agent_running` phase older than this are marked abandoned on startup.
    /// Default: 3600 (1 hour).
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold: u64,
}

fn default_stale_threshold() -> u64 {
    3600
}
```

Add to `Config`:
```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions: Option<SessionsConfig>,
}
```

**Per-agent/session threshold:** The CONTEXT.md mentions "settable per agent/session." For Phase 42, the global `[sessions].stale_threshold` is sufficient. Per-session overrides would require adding a field to `WorkSession` itself, which is a schema change best deferred until there's a concrete use case. The global default handles 99% of cases.

### 5. Recovery note format

```
Recovered on startup: stale for 3h 12m (threshold: 1h). Host: macbook.local, PID: 12345
```

Constructed with:
```rust
fn build_recovery_note(
    stale_duration: chrono::Duration,
    threshold: chrono::Duration,
    hostname: &str,
    pid: u32,
) -> String {
    let stale_h = stale_duration.num_hours();
    let stale_m = stale_duration.num_minutes() % 60;
    let threshold_h = threshold.num_hours();
    let threshold_m = threshold.num_minutes() % 60;

    let stale_fmt = if stale_h > 0 {
        format!("{stale_h}h {stale_m}m")
    } else {
        format!("{stale_m}m")
    };
    let threshold_fmt = if threshold_h > 0 {
        format!("{threshold_h}h")
    } else {
        format!("{threshold_m}m")
    };

    format!(
        "Recovered on startup: stale for {stale_fmt} (threshold: {threshold_fmt}). \
         Host: {hostname}, PID: {pid}"
    )
}
```

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Hostname retrieval | `hostname::get()` — not `libc::gethostname` with unsafe buffer management |
| Atomic file writes | Existing `tempfile::NamedTempFile` + `persist()` in `save_session` |
| Duration formatting | `chrono::Duration` arithmetic — not manual epoch math |
| Session age calculation | `Utc::now() - transition.timestamp` on the `PhaseTransition` that targets `AgentRunning` |
| Config section | Follow `GatesConfig`/`GuardConfig`/`WorktreeConfig` pattern — not ad-hoc parsing |

## Common Pitfalls

### 1. Age measured from wrong timestamp
The staleness age MUST be measured from the `transition_timestamp` into `AgentRunning`, NOT from `created_at`. A session can sit in `Created` for hours before the agent picks it up — that is not staleness.

**How to find the transition timestamp:** Scan `session.transitions` for the entry where `to == SessionPhase::AgentRunning` and use its `timestamp` field. If no such transition exists but `phase == AgentRunning`, this is a data inconsistency — log a warning and skip.

### 2. Recovery must be idempotent
If recovery runs twice (e.g., server restarts), already-abandoned sessions must be skipped. Check `session.phase != SessionPhase::AgentRunning` before attempting recovery. The `can_transition_to(Abandoned)` check on terminal phases handles this naturally.

### 3. Corrupt session files
`load_session` currently returns `AssayError::Json` for malformed JSON. The recovery scan MUST catch this, log a `tracing::warn!`, and continue to the next file. Do NOT propagate the error — one corrupt file should not block recovery of other sessions.

### 4. Config deserialization backward compatibility
Adding `[sessions]` to `Config` with `#[serde(deny_unknown_fields)]` means old configs without `[sessions]` must still parse. Using `Option<SessionsConfig>` with `#[serde(default)]` handles this — `None` means "use defaults."

### 5. Recovery scan cap
The scan is capped at 100 sessions (per CONTEXT.md). Process oldest first — sort session IDs (ULID = chronological) and take the first 100. This matches the `timed_out_sessions` eviction precedent in the MCP server.

### 6. `with_session` must not swallow transition errors
If the closure's transition fails, `with_session` must NOT save the session. The current design (closure returns `Result<()>`, early-return on error before `save_session`) handles this correctly.

### 7. MCP handler refactoring scope
Phase 42 should refactor `session_update` to use `with_session` internally, proving the pattern. But do NOT refactor `gate_run`/`gate_report`/`gate_finalize` — those use in-memory `AgentSession` (not on-disk `WorkSession`) and have different semantics.

## Code Examples

### Recovery scan implementation

```rust
/// Summary of a recovery scan.
pub struct RecoverySummary {
    /// Number of sessions recovered (transitioned to Abandoned).
    pub recovered: usize,
    /// Number of sessions skipped (corrupt, wrong phase, etc.).
    pub skipped: usize,
    /// Number of errors encountered (corrupt files).
    pub errors: usize,
}

/// Scan for stale `agent_running` sessions and mark them as abandoned.
pub fn recover_stale_sessions(
    assay_dir: &Path,
    stale_threshold_secs: u64,
) -> RecoverySummary {
    let mut summary = RecoverySummary { recovered: 0, skipped: 0, errors: 0 };

    let ids = match list_sessions(assay_dir) {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!("recovery scan: cannot list sessions: {e}");
            return summary;
        }
    };

    let threshold = chrono::Duration::seconds(stale_threshold_secs as i64);
    let now = Utc::now();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let pid = std::process::id();

    // Process oldest first (ULID sort = chronological), cap at 100
    let candidates: Vec<_> = ids.into_iter().take(100).collect();

    for id in &candidates {
        let mut session = match load_session(assay_dir, id) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(session_id = %id, "recovery scan: skipping corrupt session: {e}");
                summary.errors += 1;
                continue;
            }
        };

        if session.phase != SessionPhase::AgentRunning {
            continue; // Not a candidate
        }

        // Find the transition timestamp into AgentRunning
        let entered_at = session
            .transitions
            .iter()
            .rev()
            .find(|t| t.to == SessionPhase::AgentRunning)
            .map(|t| t.timestamp);

        let Some(entered_at) = entered_at else {
            tracing::warn!(
                session_id = %id,
                "recovery scan: session in AgentRunning but no transition record, skipping"
            );
            summary.skipped += 1;
            continue;
        };

        let age = now - entered_at;
        if age < threshold {
            continue; // Not stale yet
        }

        let note = build_recovery_note(age, threshold, &hostname, pid);
        match transition_session(&mut session, SessionPhase::Abandoned, "startup_recovery", Some(&note)) {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(session_id = %id, "recovery scan: transition failed: {e}");
                summary.skipped += 1;
                continue;
            }
        }

        match save_session(assay_dir, &session) {
            Ok(_) => {
                tracing::warn!(
                    session_id = %id,
                    spec_name = %session.spec_name,
                    stale_duration = %format_duration(age),
                    "recovered stale session"
                );
                summary.recovered += 1;
            }
            Err(e) => {
                tracing::warn!(session_id = %id, "recovery scan: save failed: {e}");
                summary.errors += 1;
            }
        }
    }

    if summary.recovered > 0 {
        tracing::info!(
            recovered = summary.recovered,
            skipped = summary.skipped,
            errors = summary.errors,
            "recovery scan complete"
        );
    }

    summary
}
```

### `with_session` usage in MCP handler refactoring

```rust
// Before (current session_update handler):
let mut session = load_session(&assay_dir, &session_id)?;
transition_session(&mut session, phase, trigger, notes)?;
for id in &gate_run_ids { ... }
save_session(&assay_dir, &session)?;

// After (using with_session):
let session = assay_core::work_session::with_session(
    &assay_dir,
    &session_id,
    |session| {
        transition_session(session, phase, trigger, notes)?;
        for id in &gate_run_ids {
            if !session.gate_runs.contains(id) {
                session.gate_runs.push(id.clone());
            }
        }
        Ok(())
    },
)?;
```

### Phase 43 flow using convenience functions

```rust
// In gate_evaluate MCP tool handler (Phase 43):

// 1. Create session and transition to AgentRunning atomically
let session = assay_core::work_session::start_session(
    &assay_dir, spec_name, worktree_path, agent_command, agent_model,
)?;

// 2. Spawn evaluator subprocess, parse results
// ... (Phase 43's domain)

// 3. Record gate result and transition to GateEvaluated
let session = assay_core::work_session::record_gate_result(
    &assay_dir, &session.id, &run_id, "gate_evaluate", Some("all criteria evaluated"),
)?;

// 4. Complete the session
let session = assay_core::work_session::complete_session(
    &assay_dir, &session.id, None,
)?;
```

## Open Questions

None — all questions from CONTEXT.md have been resolved by codebase investigation.

## Verification Notes

- `SessionPhase::can_transition_to(Abandoned)` returns `true` for all non-terminal phases, confirmed at `/Users/wollax/Git/personal/assay/crates/assay-types/src/work_session.rs:43-55`
- `Config` uses `#[serde(deny_unknown_fields)]` — new `sessions` field needs `Option<T>` with `#[serde(default)]` to maintain backward compatibility
- `save_session` already uses atomic tempfile-then-rename at `/Users/wollax/Git/personal/assay/crates/assay-core/src/work_session.rs:81-108`
- `list_sessions` returns IDs in ULID-sorted order (chronological) at line 166
- The `libc` crate is already a workspace dependency (used in `guard` and `gate` modules)
- `AssayServer::new()` is synchronous; recovery should run in `serve()` before binding transport

---

*Phase: 42-session-recovery*
*Research completed: 2026-03-15*
