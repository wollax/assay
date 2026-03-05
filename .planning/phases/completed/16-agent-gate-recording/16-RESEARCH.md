# Phase 16: Agent Gate Recording - Research

**Researched:** 2026-03-05
**Domain:** MCP tool state management, serde type evolution, session persistence
**Confidence:** HIGH

## Summary

Phase 16 introduces agent-submitted gate evaluations via MCP, requiring stateful sessions (accumulate results, then finalize), new type variants (`GateKind::AgentReport`), and structural additions to existing serde types (`Criterion`, `GateResult`, `GateRunRecord`). The primary technical challenges are:

1. **Adding shared mutable state to the MCP server** (currently stateless)
2. **Evolving serde types that use `deny_unknown_fields`** without breaking existing serialized data
3. **Designing a session lifecycle** (create on gate_run, accept reports, finalize with timeout)

**Primary recommendation:** Use `Arc<Mutex<HashMap<String, AgentSession>>>` on `AssayServer` for session state, following rmcp's documented counter pattern. Add new fields to existing types with `#[serde(default, skip_serializing_if)]` for backward compatibility despite `deny_unknown_fields`.

## Standard Stack

### Core (already in workspace)

| Library       | Version | Purpose                        | Why Standard                                      |
| ------------- | ------- | ------------------------------ | ------------------------------------------------- |
| rmcp          | 0.17    | MCP server with tool macros    | Already used; `#[tool]` macro, `Arc<Mutex>` state |
| tokio         | 1       | Async runtime, Mutex, timeouts | Already used; `tokio::sync::Mutex` for async lock |
| serde         | 1       | Serialization framework        | Already used everywhere                           |
| serde_json    | 1       | JSON ser/de                    | Already used for history persistence              |
| chrono        | 0.4     | Timestamps                     | Already used in `GateResult`, `GateRunRecord`     |
| schemars      | 1       | JSON Schema generation         | Already used for all types                        |
| tempfile      | 3       | Atomic writes                  | Already used in history persistence               |

### Supporting (no new crates needed)

| Concern               | Solution                          | Rationale                                                   |
| --------------------- | --------------------------------- | ----------------------------------------------------------- |
| Session state          | `Arc<tokio::sync::Mutex<..>>`    | rmcp examples use this exact pattern; server must be `Clone` |
| Session timeout        | `tokio::time::sleep` + background task | Already have tokio with `features = ["full"]`         |
| Unique session IDs     | Reuse `history::generate_run_id`  | Already generates timestamp + 6-char hex                    |
| Session crash recovery | `serde_json` to `.assay/sessions/` | Same atomic write pattern as history                      |

### No new crates required

The entire phase can be implemented with existing workspace dependencies. The session model is a HashMap behind an async Mutex -- no external state management library needed.

## Architecture Patterns

### Recommended Changes by Crate

```
assay-types/
  src/
    criterion.rs       # Add `kind` and `prompt` fields to Criterion
    gate.rs            # Add AgentReport variant to GateKind, agent fields to GateResult
    gate_run.rs        # No structural changes (GateRunRecord already supports this)
    gates_spec.rs      # Add `kind` and `prompt` fields to GateCriterion
    session.rs         # NEW: AgentSession, EvaluatorRole, Confidence, AgentEvaluation types

assay-core/
  src/
    gate/
      mod.rs           # Update evaluate dispatch for AgentReport (skip, mark pending)
      session.rs       # NEW: Session lifecycle (create, report, finalize, timeout)
    error.rs           # Add session-related error variants

assay-mcp/
  src/
    server.rs          # Add state, gate_report tool, gate_finalize tool
                       # Modify gate_run to create sessions for mixed specs
```

### Pattern 1: Stateful MCP Server with Session Map

**What:** Add `Arc<tokio::sync::Mutex<HashMap<String, AgentSession>>>` to `AssayServer`.
**When to use:** When MCP tools need to share state across calls.
**Why:** rmcp requires `Clone` on the server struct; `Arc<Mutex>` is the documented pattern.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AssayServer {
    tool_router: ToolRouter<Self>,
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
}
```

### Pattern 2: Serde Type Evolution with `deny_unknown_fields`

**What:** Adding optional fields to structs that use `deny_unknown_fields`.
**When to use:** When existing serialized data must still deserialize correctly.
**Key insight:** `deny_unknown_fields` rejects unknown *keys*, not missing *keys*. Adding a new field with `#[serde(default, skip_serializing_if = "Option::is_none")]` is backwards-compatible -- old data without the field deserializes fine (defaults to `None`), and old consumers that don't know about the field will reject new data with the field present.

**Impact on `GateRunRecord`:** The `deny_unknown_fields` on `GateRunRecord` means records with new agent fields will NOT load in older assay versions. This is acceptable because:
- Records include `assay_version` for schema migration
- The project is pre-1.0; strict backwards compatibility is not required
- The `deny_unknown_fields` is intentional as a version guard

```rust
// Adding optional agent fields to GateResult -- backwards compatible for deserialization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    pub passed: bool,
    pub kind: GateKind,
    // ... existing fields ...

    /// Evidence the agent observed (concrete facts).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<String>,

    /// Agent's reasoning for pass/fail conclusion.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<String>,

    /// Agent's confidence level in the evaluation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confidence: Option<Confidence>,

    /// Role of the evaluator who produced this result.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evaluator_role: Option<EvaluatorRole>,
}
```

### Pattern 3: Criterion Kind Dispatch

**What:** Explicit `kind` field on `Criterion`/`GateCriterion` with mutual exclusivity validation.
**When to use:** When a criterion type determines evaluation strategy.

```rust
// On Criterion (and similarly GateCriterion)
#[serde(skip_serializing_if = "Option::is_none", default)]
pub kind: Option<CriterionKind>,

#[serde(skip_serializing_if = "Option::is_none", default)]
pub prompt: Option<String>,
```

**Validation rule:** If `kind == Some(CriterionKind::AgentReport)`, then `cmd` and `path` must both be `None`. If `cmd` or `path` is `Some`, then `kind` must not be `AgentReport`.

**Dispatch priority (in evaluate):**
1. Explicit `kind` field takes precedence
2. Fall back to existing `cmd`/`path` inference

### Pattern 4: Session Lifecycle

**What:** Sessions track pending agent evaluations across MCP tool calls.
**State machine:**

```
gate_run (with AgentReport criteria)
    |
    v
Session::Active { pending_criteria, completed, ... }
    |  <-- gate_report calls fill in criteria
    v
gate_finalize  OR  timeout
    |
    v
GateRunRecord persisted to history
```

### Pattern 5: Multiple Evaluations per Criterion with Priority

**What:** Allow multiple evaluations from different roles; highest-priority role is authoritative.
**Priority:** `human > independent > self`

```rust
pub struct AgentSession {
    pub session_id: String,
    pub spec_name: String,
    pub created_at: DateTime<Utc>,
    pub command_results: Vec<CriterionResult>,  // Immediate results from gate_run
    pub agent_evaluations: HashMap<String, Vec<AgentEvaluation>>,  // criterion_name -> evaluations
    pub criteria_names: HashSet<String>,  // Valid agent criterion names from spec
    pub spec_enforcement: HashMap<String, Enforcement>,  // Trust ceiling per criterion
}
```

### Anti-Patterns to Avoid

- **Blocking the MCP server on agent evaluation:** `gate_run` must return immediately with pending status for agent criteria, not wait for agent reports.
- **Mutable global state without Mutex:** rmcp clones the server struct; raw state won't be shared. Must use `Arc<Mutex>`.
- **Storing sessions only in memory:** Sessions must be serializable for crash recovery. Persist to `.assay/sessions/` on every state change.
- **Allowing agent to escalate enforcement:** Agent-submitted enforcement must be clamped to spec-defined ceiling.

## Don't Hand-Roll

| Problem                    | Don't Build                     | Use Instead                        | Why                                        |
| -------------------------- | ------------------------------- | ---------------------------------- | ------------------------------------------ |
| Run ID generation          | Custom UUID/random              | `history::generate_run_id`         | Already exists, tested, deterministic format |
| Atomic file writes         | Manual temp + rename            | `tempfile::NamedTempFile::persist` | Already used in history module             |
| Async mutual exclusion     | `std::sync::Mutex` in async     | `tokio::sync::Mutex`              | Holding std Mutex across await is UB-adjacent |
| Session timeout scheduling | Manual thread + sleep           | `tokio::spawn` + `tokio::time::sleep` | Already in the async runtime           |
| JSON schema generation     | Manual schema writing           | `schemars::JsonSchema` derive      | Already used for all types                 |
| Enforcement resolution     | Custom logic                   | `gate::resolve_enforcement`        | Already exists, handles spec defaults      |

**Key insight:** The session model is fundamentally a HashMap + serialization. No need for a database, actor framework, or message queue. The MCP server is single-process; `Arc<Mutex>` is sufficient.

## Common Pitfalls

### Pitfall 1: `deny_unknown_fields` Breaks Forward Compatibility

**What goes wrong:** New fields on `GateRunRecord` or `GateResult` cause old assay versions to reject new records.
**Why it happens:** `deny_unknown_fields` is on `GateRunRecord` and `Criterion`.
**How to avoid:** This is expected and acceptable pre-1.0. The `assay_version` field exists for exactly this purpose. Document that records from newer versions cannot be read by older versions.
**Warning signs:** Tests that deserialize old JSON fixtures will pass (new fields default to None), but new JSON won't deserialize in old code.

### Pitfall 2: Deadlock on Session Mutex

**What goes wrong:** Holding the session lock across an `.await` point that itself needs the lock.
**Why it happens:** `gate_finalize` needs to read session state, persist it, then remove it -- tempting to hold the lock the entire time.
**How to avoid:** Clone the session data out of the lock, drop the lock, do I/O, then re-acquire to remove.
**Warning signs:** MCP server hangs on second tool call.

```rust
// WRONG: holds lock across await/blocking IO
let mut sessions = self.sessions.lock().await;
let session = sessions.get("id").unwrap();
history::save(&assay_dir, &record, max_history)?;  // IO while holding lock
sessions.remove("id");

// RIGHT: clone out, drop lock, do IO, re-acquire
let session = {
    let sessions = self.sessions.lock().await;
    sessions.get("id").cloned()
};
// ... build record from session, do IO ...
{
    let mut sessions = self.sessions.lock().await;
    sessions.remove("id");
}
```

### Pitfall 3: Session Timeout Race with Finalize

**What goes wrong:** Timeout fires after agent calls `gate_finalize` but before cleanup, double-persisting.
**Why it happens:** Background timeout task and foreground finalize call race.
**How to avoid:** Use a `tokio::sync::watch` channel or simply check-and-remove atomically. The `remove` from the HashMap is the atomic operation -- if finalize removes it first, the timeout handler finds nothing.
**Warning signs:** Duplicate run records in history.

### Pitfall 4: `CriterionKind` vs `GateKind` Naming Confusion

**What goes wrong:** Confusing the criterion-level `kind` field (spec definition) with `GateKind` (result metadata).
**Why it happens:** Both relate to "what kind of gate" but at different layers.
**How to avoid:** Use distinct names. `Criterion.kind` is `Option<CriterionKind>` (an enum with `AgentReport`). `GateResult.kind` is `GateKind` (already has `Command`, `FileExists`, `AlwaysPass`, will add `AgentReport`). The `CriterionKind` enum can be a simple `#[serde(tag = "kind")]` string enum -- or even just a `String` field that's validated.

**Recommendation:** Since the CONTEXT.md says `kind = "AgentReport"` on the Criterion, and existing `GateKind` already uses `#[serde(tag = "kind")]`, the simplest approach is:
- Add an optional `kind` field to `Criterion`/`GateCriterion` as `Option<String>` (validated to known values)
- Add `GateKind::AgentReport` variant (no inner data needed -- the definition is "this criterion is agent-evaluated")

### Pitfall 5: Session Crash Recovery File Accumulation

**What goes wrong:** Crashed sessions leave orphan files in `.assay/sessions/`.
**Why it happens:** If the server crashes between session creation and finalization, the session file persists.
**How to avoid:** On server startup, scan for stale session files and auto-finalize them as failed (missing required criteria = failure).
**Warning signs:** Growing `.assay/sessions/` directory.

### Pitfall 6: `deny_unknown_fields` on Criterion Blocks New Fields

**What goes wrong:** Adding `kind` and `prompt` to `Criterion` (which has `deny_unknown_fields`) means existing spec files with the new fields won't parse on old versions, AND the new fields must be properly declared.
**Why it happens:** `deny_unknown_fields` is strict.
**How to avoid:** This is fine -- new spec files using `kind = "AgentReport"` are inherently incompatible with old assay versions. The `deny_unknown_fields` catches mistyped field names, which is valuable.

## Code Examples

### Adding AgentReport to GateKind

```rust
// In assay-types/src/gate.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum GateKind {
    Command { cmd: String },
    AlwaysPass,
    FileExists { path: String },
    /// A gate evaluated by an agent via structured reasoning.
    AgentReport,
}
```

### EvaluatorRole Enum

```rust
// In assay-types/src/session.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluatorRole {
    /// Same agent that wrote the code evaluates its own work.
    #[serde(rename = "self")]
    SelfEval,
    /// A different agent evaluates without having written the code.
    Independent,
    /// A human submitted the evaluation.
    Human,
}
```

Note: `self` is a Rust keyword. The variant must be named differently (e.g., `SelfEval`) with `#[serde(rename = "self")]` to serialize as `"self"` in JSON/TOML.

### Confidence Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}
```

### MCP gate_report Tool Parameters

```rust
#[derive(Deserialize, JsonSchema)]
struct GateReportParams {
    /// Spec name this evaluation belongs to.
    #[schemars(description = "Spec name (e.g. 'auth-flow')")]
    spec_name: String,

    /// Session ID returned by gate_run.
    #[schemars(description = "Session ID from the gate_run response")]
    session_id: String,

    /// Criterion name being evaluated (must match a criterion in the spec).
    #[schemars(description = "Name of the criterion being evaluated")]
    criterion_name: String,

    /// Whether the criterion passed.
    passed: bool,

    /// What the agent observed (concrete facts).
    evidence: String,

    /// Why those facts lead to pass/fail.
    reasoning: String,

    /// Confidence in the evaluation.
    #[serde(default)]
    confidence: Option<Confidence>,

    /// Role of the evaluator.
    evaluator_role: EvaluatorRole,
}
```

### Session Timeout Pattern

```rust
// After creating a session, spawn a background timeout task
let sessions = self.sessions.clone();
let session_id = session_id.clone();
let timeout_duration = Duration::from_secs(SESSION_TIMEOUT_SECS);

tokio::spawn(async move {
    tokio::time::sleep(timeout_duration).await;
    let mut sessions = sessions.lock().await;
    if let Some(session) = sessions.remove(&session_id) {
        // Auto-finalize: missing required criteria = failure
        let record = session.finalize_as_timed_out();
        // Persist record (fire-and-forget, log errors)
        if let Err(e) = persist_record(&record) {
            tracing::error!("Failed to persist timed-out session: {e}");
        }
    }
});
```

**Recommended session timeout:** 30 minutes. Agent workflows can take significant time, but stale sessions should not persist indefinitely. This is long enough for complex evaluations but short enough to catch abandoned sessions within the same working session.

## State of the Art

| Old Approach                    | Current Approach              | When Changed | Impact                                          |
| ------------------------------- | ----------------------------- | ------------ | ----------------------------------------------- |
| Stateless MCP server            | Stateful with session map     | This phase   | Enables accumulate-then-commit pattern           |
| All criteria evaluated at once  | Mixed: cmd immediate, agent deferred | This phase | `gate_run` returns partial results + session ID |
| `GateResult` = command output   | `GateResult` = command OR agent output | This phase | Agent fields optional on same struct       |
| Single evaluation per criterion | Multiple evaluations with priority | This phase | Support for independent + self-evaluation      |

**Not deprecated (preserved):**
- Existing `evaluate_all` / `evaluate_all_gates` paths for command-only specs (no session needed)
- Existing `GateRunRecord` structure (extended, not replaced)
- Existing MCP tools (`spec_list`, `spec_get`, `gate_run`) -- `gate_run` behavior extended

## Open Questions

1. **Session timeout duration** (Claude's discretion)
   - What we know: Must be long enough for agent workflows, short enough to catch stale sessions
   - Recommendation: 30 minutes (1800 seconds) as a const, configurable later
   - Confidence: MEDIUM -- this is a reasonable default but may need tuning

2. **Crash recovery on server restart**
   - What we know: Sessions should be serialized to `.assay/sessions/`
   - What's unclear: Should the server scan for stale sessions on startup, or only on the next `gate_run`?
   - Recommendation: Scan on startup. Simple, predictable, ensures cleanup.

3. **`gate_run` response shape when session is created**
   - What we know: Must include session_id, command results, and pending agent criteria
   - Recommendation: Extend `GateRunResponse` with optional `session_id` and a `status` field per criterion that can be `"pending"` for agent criteria

4. **Whether `CriterionKind` should be an enum or string field on `Criterion`**
   - What we know: CONTEXT.md says `kind = "AgentReport"` as a field value
   - Recommendation: Use `Option<String>` with validation, keeping it simple. The string value `"AgentReport"` is validated during spec parsing. This avoids needing a separate enum that might conflict with the existing `GateKind` naming.
   - Alternative: A proper `CriterionKind` enum with `#[serde(rename_all = "PascalCase")]`. This is cleaner for type safety.
   - **Decision for planner:** Either works. Enum is more type-safe; string is simpler for TOML authoring. Recommend enum since it gets JSON Schema support via schemars.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_rmcp` -- rmcp tool macro, shared state with `Arc<Mutex>`, `#[tool_router]` pattern
- Codebase analysis: `crates/assay-types/src/gate.rs`, `criterion.rs`, `gate_run.rs`, `gates_spec.rs`, `enforcement.rs`
- Codebase analysis: `crates/assay-core/src/gate/mod.rs`, `history/mod.rs`, `error.rs`
- Codebase analysis: `crates/assay-mcp/src/server.rs`

### Secondary (MEDIUM confidence)
- Serde documentation on `deny_unknown_fields` + `default` interaction (well-established behavior)

### Tertiary (LOW confidence)
- Session timeout of 30 minutes (reasonable guess, not backed by external data)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in workspace, patterns verified against codebase and Context7
- Architecture: HIGH -- patterns derived directly from existing codebase conventions and rmcp documentation
- Pitfalls: HIGH -- identified from concrete code analysis (deny_unknown_fields, Mutex holding patterns)
- Session design: MEDIUM -- follows established patterns but the specific timeout/recovery behaviors are design choices

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (stable domain, no fast-moving dependencies)
