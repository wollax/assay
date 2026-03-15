# Phase 40: WorkSession Type & Persistence — Research

Researched: 2026-03-15

## Standard Stack

| Need | Use | Confidence |
|------|-----|------------|
| ULID generation | `ulid` crate (dylanhart/ulid-rs) v1.2, features: `["serde"]` | HIGH |
| Serialization | `serde` + `serde_json` (already in workspace) | HIGH |
| Timestamps | `chrono` (already in workspace, `DateTime<Utc>`) | HIGH |
| Schema generation | `schemars` (already in workspace) | HIGH |
| Atomic file writes | `tempfile` (already in workspace) | HIGH |
| Error handling | `thiserror` via existing `AssayError` enum | HIGH |

### ULID Crate Decision

Use `ulid` (crate name: `ulid`, repo: `dylanhart/ulid-rs`). This is the only maintained Rust ULID implementation with high source reputation. Features needed: `serde` (for Serialize/Deserialize). The crate does NOT support `schemars` natively.

**schemars workaround:** The `WorkSession` type's `id` field should be typed as `ulid::Ulid` internally but use `#[schemars(with = "String")]` or store as `String` in the serializable type. Recommendation: **Store as `String` in the persisted type** (matching the existing `session_id: String` pattern in `AgentSession` and `run_id: String` in `GateRunRecord`). Use `ulid::Ulid` only at generation time, immediately convert to string. This avoids a schemars compatibility issue entirely and keeps the serialized JSON human-readable.

**Workspace dependency:** Add to root `Cargo.toml`:
```toml
ulid = { version = "1.2", features = ["serde"] }
```
Add to `assay-types/Cargo.toml` (for generation) or `assay-core/Cargo.toml` (if generation happens in core). Since the type definition lives in `assay-types` but ID generation is business logic, the `ulid` dep belongs in `assay-core` only. The `WorkSession` type in `assay-types` stores `id: String`.

## Architecture Patterns

### Type Placement

Following existing conventions:

| What | Where | Rationale |
|------|-------|-----------|
| `WorkSession` struct | `assay-types/src/work_session.rs` | Shared serializable type, matches `session.rs`, `gate_run.rs`, `worktree.rs` |
| `SessionPhase` enum | `assay-types/src/work_session.rs` | Co-located with `WorkSession` |
| `PhaseTransition` struct | `assay-types/src/work_session.rs` | Part of the audit trail, serialized with session |
| Persistence functions | `assay-core/src/work_session.rs` (or `assay-core/src/work_session/mod.rs`) | Business logic: save/load/transition, matches `history/mod.rs` pattern |
| New error variants | `assay-core/src/error.rs` | Extend existing `AssayError` enum |

### WorkSession Type Design

```rust
// assay-types/src/work_session.rs

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkSession {
    /// ULID session identifier (string representation).
    pub id: String,

    /// Name/slug of the spec this session is working on.
    pub spec_name: String,

    /// Absolute path to the worktree directory.
    pub worktree_path: PathBuf,

    /// Current phase of this session.
    pub phase: SessionPhase,

    /// Ordered list of all phase transitions (audit trail).
    pub transitions: Vec<PhaseTransition>,

    /// Agent invocation details.
    pub agent: AgentInvocation,

    /// References to gate run IDs produced during this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gate_runs: Vec<String>,

    /// Version of assay that created this session.
    pub assay_version: String,
}
```

### SessionPhase Enum (State Machine)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Created,
    AgentRunning,
    GateEvaluated,
    Completed,
    Abandoned,
}
```

### PhaseTransition (Audit Trail Entry)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PhaseTransition {
    /// The phase being transitioned FROM.
    pub from: SessionPhase,
    /// The phase being transitioned TO.
    pub to: SessionPhase,
    /// When this transition occurred.
    pub timestamp: DateTime<Utc>,
    /// What triggered this transition (e.g., "gate_run:20260315T...", "mcp:session_abandon").
    pub trigger: String,
    /// Optional freeform notes/context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
```

### AgentInvocation

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgentInvocation {
    /// The spec name being worked on.
    pub spec_name: String,
    /// The invocation command or tool name (e.g., "claude-code", "aider").
    pub command: String,
    /// Model identifier, if known (e.g., "claude-sonnet-4-20250514").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
```

### State Transition Validation

Valid transitions form a linear pipeline with an escape hatch:

```
created -> agent_running -> gate_evaluated -> completed
   |            |                |
   +------------+----------------+---> abandoned
```

Implement as a pure function:

```rust
impl SessionPhase {
    pub fn can_transition_to(&self, next: SessionPhase) -> bool {
        matches!(
            (self, next),
            (Self::Created, SessionPhase::AgentRunning)
                | (Self::AgentRunning, SessionPhase::GateEvaluated)
                | (Self::GateEvaluated, SessionPhase::Completed)
                | (_, SessionPhase::Abandoned) // any -> abandoned
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Abandoned)
    }
}
```

### Persistence Pattern

Follow the existing `history/mod.rs` pattern exactly:

1. **Directory structure:** `.assay/sessions/<session-id>.json` (flat, not nested by spec)
2. **Atomic writes:** `tempfile::NamedTempFile` + `persist()` (same as gate run records)
3. **Path validation:** Reuse or extract `validate_path_component()` from `history/mod.rs`
4. **Pretty-printed JSON:** `serde_json::to_string_pretty()` for human readability
5. **Listing:** Read directory, filter `.json`, parse filenames
6. **Loading:** `serde_json::from_str()` with error context

### File Naming

Use the ULID directly as the filename: `.assay/sessions/01JQXYZ123ABC456DEF789GH.json`

ULIDs are lexicographically sortable by creation time, so directory listing order = chronological order (same property that `generate_run_id` achieves via timestamp prefix for gate runs).

### Gate Run References

Store **bare IDs** (strings), not embedded summaries. Rationale:
- Gate run records are already persisted in `.assay/results/<spec>/`; duplicating data invites staleness
- Phase 41 (MCP tools) can load the full record on demand via `history::load()`
- Keeps session files lightweight
- Matches the reference pattern used by `GateRunRecord.run_id`

### Worktree Path Storage

Store as **absolute `PathBuf`**. Rationale:
- `WorktreeInfo.path` is already `PathBuf` (absolute) throughout the codebase
- Relative paths would require knowing the base directory at load time, adding complexity
- Sessions are machine-local artifacts (not committed to git), so absolute paths are stable

### Spec Reference

Store **slug only** (not slug + content hash). Rationale:
- Content hashing is not part of the current spec system
- The spec can change during a session (that's expected: agent modifies code, gates re-evaluate)
- Slug is sufficient for `spec::load_spec_entry()` lookups

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Unique sortable IDs | `ulid::Ulid::new().to_string()` |
| Atomic file writes | `tempfile::NamedTempFile` + `persist()` — pattern from `history::save()` |
| Timestamp generation | `chrono::Utc::now()` |
| Path validation | Extract existing `validate_path_component()` from `history/mod.rs` (or call it directly if made `pub(crate)`) |
| Error types | Add variants to existing `AssayError` enum |
| State machine validation | Simple `match` on `(current, next)` tuple — no state machine crate needed |

## Common Pitfalls

### 1. Forgetting `deny_unknown_fields` for versioned artifacts
**Mitigation:** `GateRunRecord` uses `#[serde(deny_unknown_fields)]` for strict deserialization. `WorkSession` should NOT use this initially — sessions are mutable documents that evolve across phases, and future phases (41, 42) will add fields. Use permissive deserialization (`#[serde(default)]` on optional fields) to support forward compatibility.

**Confidence: HIGH** — Direct observation of the codebase pattern and the roadmap.

### 2. Non-atomic writes leaving corrupt session files
**Mitigation:** Use the tempfile-then-rename pattern from `history::save()`. Never write directly to the final path.

**Confidence: HIGH** — Existing pattern proven in production.

### 3. Race conditions on session file updates
**Mitigation:** Sessions are single-writer (one agent per session). Document this invariant. Phase 42 (recovery) may need advisory file locking, but that's out of scope for Phase 40.

**Confidence: MEDIUM** — Single-writer assumption needs validation against Phase 41/42 designs.

### 4. ULID collision within the same millisecond
**Mitigation:** ULID spec includes 80 bits of randomness per millisecond. Collision probability is negligible for this use case (session creation is infrequent). No mitigation needed beyond using `Ulid::new()`.

**Confidence: HIGH** — ULID spec guarantees.

### 5. `PathBuf` serialization differences across platforms
**Mitigation:** `PathBuf` serializes as a string via serde. On the same platform this is lossless. Sessions are machine-local (not shared across OS), so this is safe. Document that sessions are not portable across operating systems.

**Confidence: HIGH** — Sessions are `.assay/` local artifacts.

### 6. Transition from `AgentSession` (v0.3) to `WorkSession` (v0.4)
**Mitigation:** These are distinct types for distinct purposes. `AgentSession` is in-memory state for a single gate evaluation. `WorkSession` is on-disk state for the full development lifecycle. They coexist — `WorkSession.gate_runs` references the gate runs that `AgentSession` produces. No migration needed.

**Confidence: HIGH** — CONTEXT.md explicitly states "WorkSession (on-disk) is distinct from AgentSession (in-memory v0.3.0)".

### 7. `.assay/sessions/` gitignore handling
**Mitigation:** The existing `.assay/.gitignore` only excludes `results/`. Sessions should also be gitignored — they contain absolute paths and machine-local state. Add `sessions/` to `.assay/.gitignore`.

**Confidence: HIGH** — Sessions contain absolute paths = not portable across clones.

## Code Examples

### Creating a WorkSession

```rust
use ulid::Ulid;
use chrono::Utc;

pub fn create_work_session(
    spec_name: &str,
    worktree_path: PathBuf,
    agent_command: &str,
    agent_model: Option<&str>,
) -> WorkSession {
    let now = Utc::now();
    let id = Ulid::new().to_string();

    WorkSession {
        id,
        spec_name: spec_name.to_string(),
        worktree_path,
        phase: SessionPhase::Created,
        transitions: vec![PhaseTransition {
            from: SessionPhase::Created,
            to: SessionPhase::Created,
            timestamp: now,
            trigger: "session_create".to_string(),
            notes: None,
        }],
        agent: AgentInvocation {
            spec_name: spec_name.to_string(),
            command: agent_command.to_string(),
            model: agent_model.map(String::from),
        },
        gate_runs: vec![],
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
```

**Note on initial transition:** The first entry in `transitions` records creation itself. The `from` and `to` are both `Created` — this is the "birth record." Alternatively, omit this and let the first real transition be the first entry. Planner's discretion.

### Transitioning Phase

```rust
pub fn transition(
    session: &mut WorkSession,
    next: SessionPhase,
    trigger: &str,
    notes: Option<&str>,
) -> Result<()> {
    if session.phase.is_terminal() {
        return Err(AssayError::WorkSessionTransition {
            session_id: session.id.clone(),
            message: format!(
                "cannot transition from terminal phase {:?}",
                session.phase
            ),
        });
    }
    if !session.phase.can_transition_to(next) {
        return Err(AssayError::WorkSessionTransition {
            session_id: session.id.clone(),
            message: format!(
                "invalid transition: {:?} -> {:?}",
                session.phase, next
            ),
        });
    }

    session.transitions.push(PhaseTransition {
        from: session.phase,
        to: next,
        timestamp: Utc::now(),
        trigger: trigger.to_string(),
        notes: notes.map(String::from),
    });
    session.phase = next;
    Ok(())
}
```

### Saving a WorkSession (Atomic Write)

```rust
pub fn save_session(assay_dir: &Path, session: &WorkSession) -> Result<PathBuf> {
    let sessions_dir = assay_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)
        .map_err(|e| AssayError::io("creating sessions directory", &sessions_dir, e))?;

    let json = serde_json::to_string_pretty(session)
        .map_err(|e| AssayError::json("serializing work session", &sessions_dir, e))?;

    let mut tmpfile = NamedTempFile::new_in(&sessions_dir)
        .map_err(|e| AssayError::io("creating temp file for session", &sessions_dir, e))?;

    tmpfile.write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing work session", &sessions_dir, e))?;

    tmpfile.as_file().sync_all()
        .map_err(|e| AssayError::io("syncing work session", &sessions_dir, e))?;

    let final_path = sessions_dir.join(format!("{}.json", session.id));
    tmpfile.persist(&final_path)
        .map_err(|e| AssayError::io("persisting work session", &final_path, e.error))?;

    Ok(final_path)
}
```

### Loading a WorkSession

```rust
pub fn load_session(assay_dir: &Path, session_id: &str) -> Result<WorkSession> {
    let path = assay_dir.join("sessions").join(format!("{session_id}.json"));
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AssayError::io("reading work session", &path, e))?;
    serde_json::from_str(&content)
        .map_err(|e| AssayError::json("deserializing work session", &path, e))
}
```

### Listing Sessions

```rust
pub fn list_sessions(assay_dir: &Path) -> Result<Vec<String>> {
    let sessions_dir = assay_dir.join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut ids: Vec<String> = std::fs::read_dir(&sessions_dir)
        .map_err(|e| AssayError::io("listing sessions", &sessions_dir, e))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                path.file_stem().and_then(|s| s.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();
    ids.sort(); // ULID string sort = chronological sort
    Ok(ids)
}
```

### Error Variants to Add

```rust
// In AssayError enum:

/// WorkSession transition error (invalid phase transition).
#[error("work session `{session_id}` transition error: {message}")]
WorkSessionTransition {
    session_id: String,
    message: String,
},

/// WorkSession not found on disk.
#[error("work session `{session_id}` not found")]
WorkSessionNotFound {
    session_id: String,
},
```

## Existing Pattern Reference

| Pattern | Location | How Phase 40 Reuses It |
|---------|----------|----------------------|
| Serializable type with serde + schemars | `assay-types/src/session.rs` (`AgentSession`) | Same derive pattern for `WorkSession` |
| Atomic JSON persistence | `assay-core/src/history/mod.rs` (`save()`) | Same tempfile-then-rename for session files |
| Run ID as timestamp+hex string | `assay-core/src/history/mod.rs` (`generate_run_id()`) | Replaced by ULID for sessions (sortable, no collision risk) |
| Schema registry | `assay-types/src/schema_registry.rs` | `inventory::submit!` for `WorkSession`, `SessionPhase`, etc. |
| Error variants | `assay-core/src/error.rs` (`AssayError`) | Add `WorkSessionTransition`, `WorkSessionNotFound` |
| Worktree metadata JSON | `assay-core/src/worktree.rs` (`write_metadata()`) | Similar pattern but with atomic writes |
| `.assay/.gitignore` exclusion | `.assay/.gitignore` (currently excludes `results/`) | Add `sessions/` line |
| Module re-export from lib.rs | `assay-types/src/lib.rs` | Add `pub mod work_session;` and re-exports |

## Open Questions for Planner

1. **Initial transition entry:** Should the `transitions` vec include a "birth" entry (`Created -> Created`) or start empty with the first real transition? The birth entry makes the audit trail self-documenting but is slightly redundant since `WorkSession` already has the creation timestamp derivable from the ULID.

2. **`validate_path_component` extraction:** The function in `history/mod.rs` is `fn validate_path_component(...)` (private). Session persistence needs the same validation. Options: (a) make it `pub(crate)`, (b) duplicate it, (c) extract to a shared utility module. Recommend (a) — minimal change.

3. **Abandoned session TTL cleanup:** CONTEXT.md mentions "TTL-based cleanup (auto-delete after configurable duration)" for abandoned sessions. This is a persistence concern — should the cleanup logic live in Phase 40 or Phase 42 (Session Recovery)? Recommend: define the TTL config field in Phase 40 (e.g., `abandoned_ttl_days: Option<u32>` on config), implement cleanup logic in Phase 42 where startup recovery already scans sessions.
