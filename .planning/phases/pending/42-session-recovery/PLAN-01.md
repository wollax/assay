---
phase: 42
plan: 01
title: "Internal API surface: with_session, convenience functions, SessionsConfig"
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/assay-types/src/lib.rs
  - crates/assay-core/Cargo.toml
  - crates/assay-core/src/work_session.rs
  - crates/assay-mcp/src/server.rs
autonomous: true
source_issue: null

must_haves:
  truths:
    - "`gate_evaluate` (Phase 43) can call session management through direct Rust function calls, never MCP round-trips"
    - "`with_session` loads, mutates, and saves atomically — closure errors abort without saving"
    - "`session_update` MCP handler uses `with_session` internally, proving the pattern"
    - "`SessionsConfig` with `stale_threshold` is parseable from `assay.toml` and backward-compatible with existing configs"
  artifacts:
    - path: "crates/assay-types/src/lib.rs"
      provides: "SessionsConfig type with stale_threshold field, sessions field on Config"
    - path: "crates/assay-core/src/work_session.rs"
      provides: "with_session, start_session, record_gate_result, complete_session, abandon_session"
    - path: "Cargo.toml"
      provides: "hostname workspace dependency"
  key_links:
    - from: "crates/assay-core/src/work_session.rs"
      to: "crates/assay-types/src/lib.rs"
      via: "SessionsConfig used for stale_threshold default"
    - from: "crates/assay-mcp/src/server.rs"
      to: "crates/assay-core/src/work_session.rs"
      via: "session_update refactored to use with_session"
---

<objective>
Build the internal Rust API surface that Phase 43's `gate_evaluate` will consume. This replaces MCP round-trips with direct function calls for session management.

Purpose: Phase 43's `gate_evaluate` needs to create sessions, record gate results, and complete sessions through Rust function calls — not by invoking MCP tools on itself. This plan builds that API.

Output: `with_session` helper, four convenience functions, `SessionsConfig` type, and proof that the pattern works by refactoring `session_update`.
</objective>

<context>
@crates/assay-core/src/work_session.rs — current persistence layer (create, transition, save, load, list)
@crates/assay-types/src/work_session.rs — WorkSession, SessionPhase, PhaseTransition types
@crates/assay-types/src/lib.rs — Config struct with existing config sections (GatesConfig, GuardConfig, WorktreeConfig)
@crates/assay-mcp/src/server.rs — session_update handler to refactor (search for `async fn session_update`)
@Cargo.toml — workspace dependencies
@crates/assay-core/Cargo.toml — assay-core dependencies
@.planning/phases/pending/42-session-recovery/42-RESEARCH.md — research findings and code examples
</context>

<tasks>

### Task 1: Add hostname dependency and SessionsConfig type

<files>
Cargo.toml
crates/assay-core/Cargo.toml
crates/assay-types/src/lib.rs
</files>

<action>

**1a. Add `hostname` to workspace dependencies in root `Cargo.toml`:**

```toml
hostname = "0.4"
```

Add it alphabetically in the `[workspace.dependencies]` section (between `dirs` and `insta`).

**1b. Add `hostname` to `assay-core/Cargo.toml`:**

Add `hostname.workspace = true` in the `[dependencies]` section.

**1c. Add `SessionsConfig` to `assay-types/src/lib.rs`:**

Add after the existing `GuardConfig` section (before `fn default_soft_threshold`):

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

Add the `inventory::submit!` block for schema registration:

```rust
inventory::submit! {
    schema_registry::SchemaEntry {
        name: "sessions-config",
        generate: || schemars::schema_for!(SessionsConfig),
    }
}
```

**1d. Add `sessions` field to `Config`:**

Add after the `worktree` field in the `Config` struct:

```rust
/// Session management configuration.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub sessions: Option<SessionsConfig>,
```

This follows the exact same pattern as `gates`, `guard`, and `worktree`. Using `Option<T>` with `#[serde(default)]` ensures backward compatibility — existing configs without `[sessions]` parse as `None`.

</action>

<verify>
`rtk cargo check -p assay-types` compiles. Create a quick test: parse a Config TOML with and without `[sessions]` section to confirm backward compatibility.
</verify>

<done>
`SessionsConfig` type exists with `stale_threshold` field defaulting to 3600. `Config` has `sessions: Option<SessionsConfig>`. Old configs without `[sessions]` still parse. `hostname` crate is wired into workspace and assay-core.
</done>

### Task 2: Add with_session helper and convenience functions

<files>
crates/assay-core/src/work_session.rs
</files>

<action>

Add the following public functions after the existing `list_sessions` function, before the `#[cfg(test)]` block.

**2a. `with_session` helper:**

```rust
/// Load a session, apply a mutation, and save atomically.
///
/// Returns the mutated session on success. If the closure returns an error,
/// the session is NOT saved — the on-disk state remains unchanged.
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

**2b. `start_session` convenience function:**

Creates a session and immediately transitions to `AgentRunning`. Saves atomically.

```rust
/// Create a session and immediately transition to AgentRunning.
///
/// This is the common Phase 43 entry point: create + transition + save
/// in a single atomic operation. Returns the saved session.
pub fn start_session(
    assay_dir: &Path,
    spec_name: &str,
    worktree_path: PathBuf,
    agent_command: &str,
    agent_model: Option<&str>,
) -> Result<WorkSession> {
    let mut session = create_work_session(spec_name, worktree_path, agent_command, agent_model);
    transition_session(&mut session, SessionPhase::AgentRunning, "session_start", None)?;
    save_session(assay_dir, &session)?;
    Ok(session)
}
```

**2c. `record_gate_result` convenience function:**

```rust
/// Transition a session to GateEvaluated and link a gate run ID.
///
/// Convenience for the common gate_evaluate flow: load → transition → link run → save.
pub fn record_gate_result(
    assay_dir: &Path,
    session_id: &str,
    gate_run_id: &str,
    trigger: &str,
    notes: Option<&str>,
) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(session, SessionPhase::GateEvaluated, trigger, notes)?;
        if !session.gate_runs.contains(&gate_run_id.to_string()) {
            session.gate_runs.push(gate_run_id.to_string());
        }
        Ok(())
    })
}
```

**2d. `complete_session` convenience function:**

```rust
/// Mark a session as completed after successful evaluation.
pub fn complete_session(
    assay_dir: &Path,
    session_id: &str,
    notes: Option<&str>,
) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(session, SessionPhase::Completed, "session_complete", notes)
    })
}
```

**2e. `abandon_session` convenience function:**

```rust
/// Mark a session as abandoned with a reason.
pub fn abandon_session(
    assay_dir: &Path,
    session_id: &str,
    reason: &str,
) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(session, SessionPhase::Abandoned, "session_abandon", Some(reason))
    })
}
```

**2f. Add tests for all new functions:**

Add in the `#[cfg(test)] mod tests` block:

1. **`with_session` happy path**: Create + save a session, then use `with_session` to transition, verify the returned session has the new phase and on-disk copy matches.
2. **`with_session` aborts on closure error**: Create + save a session in `Created`, call `with_session` with a closure that attempts an invalid transition (`Created -> Completed`). Verify it returns an error AND the on-disk session still has `Created` phase.
3. **`start_session` happy path**: Call `start_session`, verify session is in `AgentRunning` with one transition entry.
4. **`record_gate_result` happy path**: Start a session, call `record_gate_result`, verify phase is `GateEvaluated` and gate_run_id is in the list.
5. **`record_gate_result` deduplicates**: Call twice with same gate_run_id, verify only one entry.
6. **`complete_session` happy path**: Full lifecycle: `start_session` → `record_gate_result` → `complete_session`, verify `Completed` phase.
7. **`abandon_session` happy path**: Start a session, abandon it, verify `Abandoned` phase and reason in notes.
8. **`abandon_session` from created**: Create + save (in `Created`), abandon, verify it works.

</action>

<verify>
`rtk cargo test -p assay-core -- work_session` — all new and existing tests pass.
</verify>

<done>
`with_session` atomically loads, mutates, and saves. Four convenience functions compose `with_session` + `transition_session`. Closure errors abort without saving. All functions tested.
</done>

### Task 3: Refactor session_update MCP handler to use with_session

<files>
crates/assay-mcp/src/server.rs
</files>

<action>

Refactor the `session_update` tool method in `server.rs` to use `assay_core::work_session::with_session` instead of the manual load → mutate → save pattern.

The current implementation (from Phase 41) does:
```rust
let mut session = load_session(&assay_dir, &params.0.session_id)?;
// ... transition + gate_run_ids append ...
save_session(&assay_dir, &session)?;
```

Replace with:
```rust
let previous_phase = {
    // Peek at current phase before mutation
    let current = assay_core::work_session::load_session(&assay_dir, &params.0.session_id);
    match current {
        Ok(s) => s.phase,
        Err(e) => return Ok(domain_error(&e)),
    }
};

let session = match assay_core::work_session::with_session(
    &assay_dir,
    &params.0.session_id,
    |session| {
        assay_core::work_session::transition_session(
            session,
            params.0.phase,
            &params.0.trigger,
            params.0.notes.as_deref(),
        )?;
        for id in &params.0.gate_run_ids {
            if !session.gate_runs.contains(id) {
                session.gate_runs.push(id.clone());
            }
        }
        Ok(())
    },
) {
    Ok(s) => s,
    Err(e) => return Ok(domain_error(&e)),
};
```

Note: The double-load (peek + with_session) is needed because `session_update` returns `previous_phase` in the response. An alternative is to capture `previous_phase` inside the closure, but the double-load is simpler and the I/O cost is negligible for a single JSON file. Use whichever approach is cleaner — if you find a way to capture `previous_phase` from within the closure without complicating the signature, prefer that.

Verify all existing `session_update` tests still pass unchanged — behavior must be identical.

</action>

<verify>
`rtk cargo test -p assay-mcp -- session_update` — all existing session_update tests pass without modification. `rtk cargo clippy -p assay-mcp` — no warnings.
</verify>

<done>
`session_update` uses `with_session` internally. All existing tests pass unchanged. The pattern is proven for Phase 43 consumption.
</done>

</tasks>

<verification>
- [ ] `just build` compiles without errors
- [ ] `just test` — all new and existing tests pass
- [ ] `just lint` — no clippy warnings
- [ ] `just fmt-check` — formatting is clean
- [ ] `just ready` passes
- [ ] `hostname` crate added to workspace and assay-core
- [ ] `SessionsConfig` type exists with `stale_threshold` defaulting to 3600
- [ ] `Config` accepts optional `[sessions]` section, backward-compatible
- [ ] `with_session` atomically loads, mutates, and saves
- [ ] `with_session` does NOT save when closure returns error
- [ ] `start_session`, `record_gate_result`, `complete_session`, `abandon_session` work correctly
- [ ] `session_update` MCP handler refactored to use `with_session`
- [ ] All existing session MCP tests pass unchanged
</verification>

<success_criteria>
1. Internal API surface exists: `with_session`, `start_session`, `record_gate_result`, `complete_session`, `abandon_session` are public functions in `assay_core::work_session`
2. `SessionsConfig` type with configurable `stale_threshold` is integrated into `Config`
3. `session_update` MCP handler uses `with_session`, proving the pattern works
4. `hostname` dependency wired for use in PLAN-02's recovery notes
</success_criteria>

<output>
After completion, create `.planning/phases/pending/42-session-recovery/42-01-SUMMARY.md`
</output>
