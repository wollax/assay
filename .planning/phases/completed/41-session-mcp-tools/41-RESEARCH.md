# Phase 41: Session MCP Tools — Research

## WorkSession Type Model

All types live in `crates/assay-types/src/work_session.rs` and are re-exported from `assay_types`:

### `WorkSession` (the main struct)
```rust
pub struct WorkSession {
    pub id: String,                         // ULID string (26 chars)
    pub spec_name: String,
    pub worktree_path: PathBuf,
    pub phase: SessionPhase,
    pub created_at: DateTime<Utc>,
    pub transitions: Vec<PhaseTransition>,
    pub agent: AgentInvocation,
    pub gate_runs: Vec<String>,             // serde: skip when empty
    pub assay_version: String,
}
```

- Derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`
- No `deny_unknown_fields` — intentionally forward-compatible
- Registered in schema registry as `"work-session"`

### `SessionPhase` (enum, snake_case serde)
```
Created → AgentRunning → GateEvaluated → Completed
Any non-terminal → Abandoned
```
- `can_transition_to(next)` validates transitions
- `is_terminal()` returns true for Completed/Abandoned
- Derives: `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema`
- Display impl outputs snake_case matching serde

### `PhaseTransition`
Fields: `from: SessionPhase`, `to: SessionPhase`, `timestamp: DateTime<Utc>`, `trigger: String`, `notes: Option<String>` (skip when None)

### `AgentInvocation`
Fields: `command: String`, `model: Option<String>` (skip when None)

## Persistence Layer API

All functions in `crates/assay-core/src/work_session.rs`, exported as `assay_core::work_session::*`:

### `create_work_session(spec_name, worktree_path, agent_command, agent_model) -> WorkSession`
- Pure function, no I/O — generates ULID, sets phase=Created, empty transitions/gate_runs
- Stamps `assay_version` from `CARGO_PKG_VERSION`
- Does NOT auto-persist — caller must call `save_session` separately

### `transition_session(session, next, trigger, notes) -> Result<()>`
- Validates via `can_transition_to`, returns `AssayError::WorkSessionTransition` on invalid
- Appends `PhaseTransition` entry, updates `session.phase`

### `save_session(assay_dir, session) -> Result<PathBuf>`
- Atomic write (tempfile + rename) to `.assay/sessions/<id>.json`
- Creates `sessions/` dir if needed
- Validates session ID via `history::validate_path_component` (rejects path traversal)

### `load_session(assay_dir, session_id) -> Result<WorkSession>`
- Returns `AssayError::WorkSessionNotFound` if file missing

### `list_sessions(assay_dir) -> Result<Vec<String>>`
- Returns sorted session IDs (ULID sort = chronological)
- Returns empty vec if sessions dir doesn't exist
- **Note**: Returns IDs only, not full sessions — filtering by spec_name/status requires loading each session

## Existing MCP Tool Patterns

### Tool Registration
- Tools are methods on `AssayServer` annotated with `#[tool(description = "...")]`
- Params via `Parameters<T>` wrapper where `T: Deserialize + JsonSchema`
- Return type: `Result<CallToolResult, McpError>`
- The `#[tool_router]` attribute on the `impl AssayServer` block auto-registers all `#[tool]` methods
- `#[tool_handler]` on `ServerHandler` impl wires tools to the MCP protocol

### Parameter Struct Pattern
```rust
#[derive(Deserialize, JsonSchema)]
pub struct FooParams {
    #[schemars(description = "...")]
    pub required_field: String,

    #[schemars(description = "...")]
    #[serde(default)]
    pub optional_field: Option<Type>,
}
```

### Response Pattern
- Response structs derive `Serialize` (private, not exported)
- Serialized to JSON string, wrapped in `CallToolResult::success(vec![Content::text(json)])`
- Warnings field: `#[serde(default, skip_serializing_if = "Vec::is_empty")] warnings: Vec<String>`

### Common Flow
1. `resolve_cwd()?` — get working directory
2. `load_config(&cwd)` — load `.assay/config.toml`, returns `Result<Config, CallToolResult>`
3. Domain operation (calling into `assay_core`)
4. On error: `return Ok(domain_error(&e))` — converts `AssayError` to `CallToolResult` with `isError: true`
5. Serialize response to JSON, return success

### Server Info / Instructions
The `get_info()` method includes an `instructions` string listing all tools. New tools need to be added here.

### Lib.rs Exports
`lib.rs` conditionally exports param structs under `#[cfg(any(test, feature = "testing"))]` for integration tests.

## Error Handling Patterns

### AssayError Variants for Sessions
- `WorkSessionTransition { session_id, from, to }` — invalid phase transition
- `WorkSessionNotFound { session_id }` — session file not on disk
- `Io { operation, path, source }` — filesystem failures
- `Json { operation, path, source }` — serialization failures

### Error Flow
1. `assay_core` functions return `Result<T, AssayError>`
2. MCP tools catch errors with `match ... { Err(e) => return Ok(domain_error(&e)) }`
3. `domain_error()` converts to `CallToolResult::error(vec![Content::text(err.to_string())])`
4. Protocol-level errors use `McpError::internal_error(msg, None)` (reserved for infrastructure)

## Integration Points

### Where New Tools Connect

1. **`session_create`**: Calls `assay_core::work_session::create_work_session()` then `save_session()`
   - Needs `cwd`, config (for `assay_dir`), spec validation (optional)
   - Returns session ID + initial state

2. **`session_update`**: Calls `load_session()`, `transition_session()`, optionally mutates `gate_runs`, then `save_session()`
   - Must re-persist after mutation
   - Gate run linking = push to `session.gate_runs` vec

3. **`session_list`**: Calls `list_sessions()` then `load_session()` for each (for filtering)
   - Filtering by `spec_name` and `status` requires loading session data
   - Performance consideration: could be slow with many sessions

### Existing State
- `AssayServer` currently holds `sessions: Arc<Mutex<HashMap<String, AgentSession>>>` for in-memory gate sessions
- WorkSessions are a separate concept — they are on-disk persistent state, not in-memory
- No overlap/conflict with existing `sessions` field

### Dependencies Available
- `assay-core` and `assay-types` already in `Cargo.toml`
- `chrono`, `serde`, `serde_json`, `schemars` all available
- No new crate dependencies needed

## Key Risks / Considerations

1. **list_sessions returns IDs only**: Filtering by `spec_name` or `status` requires loading every session file. For small session counts this is fine. Could add a warning if session count exceeds a threshold, or consider a limit param.

2. **No assay_dir helper**: Existing tools use `cwd.join(".assay")` inline. Session tools need the same pattern — `cwd.join(".assay")` is the `assay_dir` argument to persistence functions.

3. **`session_update` is a read-modify-write**: Load session, transition, optionally add gate_run IDs, save. No locking beyond file atomicity. Concurrent updates could race, but MCP tools are single-agent so this is acceptable.

4. **Spec validation on create**: The context doc notes `create_work_session` requires all fields. The MCP tool should decide whether to validate spec_name exists (call `load_spec_entry_with_diagnostics`) or trust the caller. Existing tools like `gate_run` validate spec existence — `session_create` should likely do the same for consistency.

5. **Worktree path**: `create_work_session` takes a `PathBuf` for worktree_path. The MCP tool needs to decide if this is required or can be derived (e.g., from worktree_list for the spec). Making it optional with auto-resolution from config would match `worktree_create`'s pattern.

6. **Token budget**: `session_list` returning full `WorkSession` objects could be large. Returning summaries (id, spec_name, phase, created_at) would be more token-efficient, consistent with `gate_history` which returns `GateHistoryEntry` summaries rather than full records.

7. **Whether to add `session_get`**: `session_list` with filters could cover single-session lookup, but a dedicated `session_get(id)` would be more ergonomic and token-efficient (avoids loading all sessions). The context doc leaves this to Claude's discretion.

8. **`assay_version` in `create_work_session`**: Currently uses `env!("CARGO_PKG_VERSION")` from `assay-core`'s Cargo.toml. When called from `assay-mcp`, this will be `assay-core`'s version, not `assay-mcp`'s. This is existing behavior and probably fine since they share workspace versioning.

## RESEARCH COMPLETE
