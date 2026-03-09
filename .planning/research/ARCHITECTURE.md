# Architecture Research -- v0.3.0

**Date:** 2026-03-08
**Scope:** v0.3.0 orchestration foundation -- git worktree lifecycle, Claude Code subprocess management, session record persistence, diff/context assembly for gate evaluation, and TUI gate results viewer.

---

## Existing Architecture Baseline (post-v0.2.0)

### Workspace Structure

```
assay-cli ──> assay-core ──> assay-types
assay-tui ──> assay-core ──> assay-types
assay-mcp ──> assay-core ──> assay-types
```

### Crate Inventory (v0.2.0)

| Crate | Purpose | Key Exports | Modules |
|-------|---------|-------------|---------|
| assay-types | Serializable DTOs | `Spec`, `Criterion`, `GateKind`, `GateResult`, `Config`, `GatesConfig`, `GuardConfig`, `GateRunRecord`, `AgentSession`, `Enforcement`, `Confidence`, `EvaluatorRole`, `FeatureSpec`, `GatesSpec`, `TeamCheckpoint`, `DiagnosticsReport` | `checkpoint`, `context`, `criterion`, `enforcement`, `feature_spec`, `gate`, `gate_run`, `gates_spec`, `schema_registry`, `session` |
| assay-core | Domain logic | `config::load()`, `spec::{load,scan,validate}`, `gate::{evaluate,evaluate_all,session::*}`, `history::{save,load,list}`, `init::init()`, `context::{diagnose,discover_sessions,estimate_tokens,parse_session}`, `checkpoint::{extract_team_state,save_checkpoint,load_latest_checkpoint}`, `guard::{start_guard,stop_guard,guard_status}` | `config/`, `spec/`, `gate/`, `history/`, `init.rs`, `context/`, `checkpoint/`, `guard/`, `review/` (stub), `workflow/` (stub), `error.rs` |
| assay-mcp | MCP server | `AssayServer` with 8 tools: `spec_list`, `spec_get`, `gate_run`, `gate_report`, `gate_finalize`, `gate_history`, `context_diagnose`, `estimate_tokens` | `server.rs`, `lib.rs` |
| assay-cli | CLI binary | Clap subcommands: `init`, `spec {list,show}`, `gate {run,history,report}`, `mcp serve`, `context {diagnose,list,estimate}`, `checkpoint {save,list,show}`, `guard {start,stop,status}` | `main.rs` (single file, ~76K) |
| assay-tui | TUI binary | Skeleton event loop (placeholder with quit handler) | `main.rs` |

### Key Design Patterns (Carried Forward)

1. **Free functions, not methods.** Core logic is `pub fn evaluate(...)`, not `impl GateEvaluator`. Functional style throughout.
2. **Module-as-directory.** Non-trivial modules use `mod.rs` + submodules (e.g., `gate/mod.rs` + `gate/session.rs`, `guard/mod.rs` + `guard/{daemon,pid,watcher,thresholds,circuit_breaker,config}.rs`).
3. **`deny_unknown_fields` on persisted DTOs.** Spec, Config, GatesSpec, GateRunRecord all reject unknown keys.
4. **Schema registry via `inventory`.** Types self-register for JSON Schema generation.
5. **Non-exhaustive error enum.** `AssayError` with 20+ variants, `#[non_exhaustive]`.
6. **Atomic file writes.** `tempfile::NamedTempFile` + `persist()` for all disk writes (history, checkpoints).
7. **Sync core, async surfaces.** Core modules are synchronous. MCP handlers use `spawn_blocking` to bridge. Guard daemon uses tokio for async event loop.
8. **In-memory session state in MCP.** `Arc<Mutex<HashMap<String, AgentSession>>>` for active gate sessions.

### Current Error Variants

The `AssayError` enum has variants covering: `Io`, `ConfigParse`, `ConfigValidation`, `AlreadyInitialized`, `SpecParse`, `SpecValidation`, `SpecScan`, `GateExecution`, `SpecNotFound`, `FeatureSpecParse`, `FeatureSpecValidation`, `GatesSpecParse`, `GatesSpecValidation`, `SessionNotFound`, `InvalidCriterion`, `SessionError`, `SessionDirNotFound`, `SessionFileNotFound`, `SessionParse`, `CheckpointWrite`, `CheckpointRead`, `GuardAlreadyRunning`, `GuardNotRunning`, `GuardCircuitBreakerTripped`.

### Current Dependencies (assay-core/Cargo.toml)

```toml
assay-types, chrono, serde, serde_json, tempfile, dirs, regex-lite,
thiserror, toml, notify, tokio, tracing, tracing-appender
# unix-only: libc
```

### Workspace Dependencies Available

```toml
serde, serde_json, schemars, rmcp, tokio, clap, ratatui, crossterm,
tracing, tracing-subscriber, chrono, toml, thiserror, color-eyre,
inventory, insta, jsonschema, libc, tempfile, anyhow, serial_test,
dirs, regex-lite, notify, tracing-appender
```

---

## New Component 1: Worktree Module (`assay_core::worktree`)

### Purpose

Manage the git worktree lifecycle for isolated agent work: create worktrees from branches, track their state, and clean them up after merge or abandonment.

### Where It Lives

**`crates/assay-core/src/worktree/mod.rs`** with submodules.

Rationale: Worktree management is domain logic (creating/tracking/cleaning git worktrees), not a serializable DTO (not assay-types), and not presentation (not CLI/TUI/MCP). Both the future orchestrator and the CLI need worktree operations -- core is the shared layer.

### Type Requirements (assay-types)

```rust
// NEW in assay-types/src/worktree.rs

/// State of a managed worktree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeState {
    /// Worktree created, agent not yet started.
    Pending,
    /// Agent is actively working in this worktree.
    Active,
    /// Agent finished, awaiting gate evaluation.
    AwaitingGate,
    /// Gate passed, ready for merge.
    ReadyToMerge,
    /// Merged back to target branch.
    Merged,
    /// Abandoned (gate failed, user cancelled, etc.).
    Abandoned,
}

/// Record of a managed worktree.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeRecord {
    /// Unique identifier for this worktree (derived from spec + timestamp).
    pub id: String,
    /// Spec name this worktree is implementing.
    pub spec_name: String,
    /// Branch name created for this worktree.
    pub branch_name: String,
    /// Absolute path to the worktree directory.
    pub path: String,
    /// Source branch/ref the worktree was created from.
    pub base_ref: String,
    /// Current lifecycle state.
    pub state: WorktreeState,
    /// When this worktree was created.
    pub created_at: DateTime<Utc>,
    /// When the state last changed.
    pub updated_at: DateTime<Utc>,
}
```

### Module API (assay-core::worktree)

```rust
/// Create a new git worktree for a spec.
///
/// Creates a branch `assay/<spec-name>-<short-id>` from `base_ref`,
/// then `git worktree add` at the configured location.
pub fn create(
    repo_root: &Path,
    spec_name: &str,
    base_ref: &str,
    worktree_dir: &Path,
) -> Result<WorktreeRecord>

/// List all Assay-managed worktrees.
///
/// Reads `.assay/worktrees/` records and cross-references with
/// `git worktree list` to detect stale entries.
pub fn list(assay_dir: &Path) -> Result<Vec<WorktreeRecord>>

/// Update the state of a worktree record.
pub fn update_state(
    assay_dir: &Path,
    worktree_id: &str,
    new_state: WorktreeState,
) -> Result<WorktreeRecord>

/// Remove a worktree and clean up its branch.
///
/// Runs `git worktree remove` and `git branch -d`, then deletes
/// the record file.
pub fn remove(repo_root: &Path, assay_dir: &Path, worktree_id: &str) -> Result<()>

/// Prune stale worktree records (worktree dir no longer exists).
pub fn prune(repo_root: &Path, assay_dir: &Path) -> Result<usize>
```

### Storage Design

```
.assay/
  worktrees/                    # NEW directory
    <worktree-id>.json          # One record per worktree
```

Follows the same atomic-write pattern as `history/` (tempfile + persist).

### Git Interaction Pattern

The worktree module shells out to `git` via `std::process::Command`, matching the pattern in `gate/mod.rs` for command execution. No git library dependency (libgit2/gitoxide) -- keeps the dependency footprint small and avoids FFI complexity.

```rust
// Example: creating a worktree
Command::new("git")
    .args(["worktree", "add", "-b", &branch_name, worktree_path, base_ref])
    .current_dir(repo_root)
    .output()
```

### New Error Variants

```rust
/// Git command failed (worktree create/remove/list).
#[error("git operation `{operation}` failed in `{repo_root}`: {message}")]
GitOperation {
    operation: String,
    repo_root: PathBuf,
    message: String,
}

/// Worktree record not found.
#[error("worktree `{id}` not found")]
WorktreeNotFound { id: String }

/// Worktree state transition invalid.
#[error("worktree `{id}` cannot transition from {from:?} to {to:?}")]
WorktreeInvalidTransition {
    id: String,
    from: WorktreeState,
    to: WorktreeState,
}
```

### Integration Points

| Component | Change Type | Details |
|-----------|-------------|---------|
| assay-types | New module `worktree.rs` | `WorktreeState`, `WorktreeRecord` types |
| assay-types | `lib.rs` | Add `pub mod worktree;` + re-exports |
| assay-core | New module `worktree/mod.rs` | `create`, `list`, `update_state`, `remove`, `prune` |
| assay-core | `lib.rs` | Add `pub mod worktree;` |
| assay-core | `error.rs` | Add `GitOperation`, `WorktreeNotFound`, `WorktreeInvalidTransition` |
| assay-types | `lib.rs` (Config) | Add optional `[worktrees]` config section |

### Config Extension

```rust
// NEW in assay-types lib.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeConfig {
    /// Base directory for worktrees, relative to repo root.
    /// Defaults to `.assay/worktrees-data/`.
    #[serde(default = "default_worktree_dir")]
    pub dir: String,
    /// Branch prefix for worktree branches. Defaults to `assay/`.
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}
```

The `Config` struct gains: `pub worktrees: Option<WorktreeConfig>`.

---

## New Component 2: Launcher Module (`assay_core::launcher`)

### Purpose

Manage Claude Code subprocess lifecycle: spawn an agent in a worktree with a spec prompt, monitor its status, and capture its exit.

### Where It Lives

**`crates/assay-core/src/launcher/mod.rs`**

Rationale: Process management is domain logic that both CLI (single-session launch) and the future orchestrator (multi-session management) will use.

### Type Requirements (assay-types)

```rust
// NEW in assay-types/src/launcher.rs

/// Configuration for launching an agent subprocess.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaunchConfig {
    /// Command to invoke the agent (e.g., "claude").
    #[serde(default = "default_agent_command")]
    pub command: String,
    /// Additional CLI arguments to pass.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables to set.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    /// Maximum duration in seconds before timeout. None = no timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// Status of a launched agent process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProcessStatus {
    /// Process is running.
    Running,
    /// Process exited with the given code.
    Exited { code: Option<i32> },
    /// Process was killed (timeout or signal).
    Killed { reason: String },
    /// Process failed to start.
    Failed { error: String },
}

/// Record of a launched agent process.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LaunchRecord {
    /// Links to the worktree this agent is working in.
    pub worktree_id: String,
    /// Spec the agent is implementing.
    pub spec_name: String,
    /// PID of the agent process (if started successfully).
    pub pid: Option<u32>,
    /// Current process status.
    pub status: AgentProcessStatus,
    /// When the agent was launched.
    pub launched_at: DateTime<Utc>,
    /// When the agent exited (if finished).
    pub exited_at: Option<DateTime<Utc>>,
}
```

### Module API (assay-core::launcher)

```rust
/// Launch a Claude Code agent in a worktree.
///
/// Spawns `claude --dangerously-skip-permissions -p <prompt>`
/// (or configured command) in the worktree directory. The prompt
/// is assembled from the spec and CLAUDE.md context.
pub fn launch(
    worktree_path: &Path,
    spec_name: &str,
    prompt: &str,
    config: &LaunchConfig,
) -> Result<LaunchRecord>

/// Check the status of a launched agent.
///
/// Polls the PID to determine if the process is still running.
pub fn check_status(record: &LaunchRecord) -> AgentProcessStatus

/// Kill a running agent process.
pub fn kill(record: &LaunchRecord) -> Result<()>

/// Build the agent prompt from a spec and optional context.
///
/// Assembles the spec criteria, relevant context from the
/// feature spec, and any gate results from prior runs.
pub fn build_prompt(
    spec: &Spec,
    feature_spec: Option<&FeatureSpec>,
    prior_results: Option<&GateRunRecord>,
) -> String
```

### Subprocess Strategy

The launcher uses `std::process::Command` with these considerations:

1. **Stdout/stderr handling:** Piped to files in `.assay/sessions/<worktree-id>/` for post-mortem analysis. Not captured in memory (agent sessions can run for hours).
2. **Process group:** The agent is spawned in its own process group (`setsid` on Unix) so it can be killed cleanly with all children.
3. **Non-blocking monitoring:** `check_status` uses `kill(pid, 0)` to probe liveness without blocking, same pattern as `guard/pid.rs`.

### Sync vs Async Decision

The launcher module should be **async** (unlike most of assay-core which is sync). Rationale:
- Process monitoring involves polling/waiting which is naturally async
- The guard daemon already establishes the precedent for async in assay-core (`guard::daemon`)
- The future orchestrator will need to manage multiple concurrent agents

Functions that do one-shot work (build_prompt, kill) remain sync. Functions that manage lifecycle (launch with timeout monitoring) use async.

### New Error Variants

```rust
/// Agent process failed to start.
#[error("launching agent in `{worktree}`: {message}")]
LaunchFailed {
    worktree: PathBuf,
    message: String,
}

/// Agent process timed out.
#[error("agent in `{worktree}` timed out after {timeout_secs}s")]
AgentTimeout {
    worktree: PathBuf,
    timeout_secs: u64,
}
```

### Integration Points

| Component | Change Type | Details |
|-----------|-------------|---------|
| assay-types | New module `launcher.rs` | `LaunchConfig`, `AgentProcessStatus`, `LaunchRecord` |
| assay-core | New module `launcher/mod.rs` | `launch`, `check_status`, `kill`, `build_prompt` |
| assay-core | `error.rs` | Add `LaunchFailed`, `AgentTimeout` |
| assay-types | `lib.rs` (Config) | Add optional `[agent]` config section with `LaunchConfig` |

---

## New Component 3: Session Module (`assay_core::session`)

### Purpose

Persist session records that track the full lifecycle of a spec-work-gate cycle: from worktree creation through agent launch, gate evaluation, to merge or abandonment.

### Disambiguation

There are currently two "session" concepts in the codebase:
- **`assay_types::session::AgentSession`** -- In-memory gate evaluation session (accumulates criterion results). Used by MCP `gate_run`/`gate_report`/`gate_finalize` flow. Stored in `Arc<Mutex<HashMap>>` in the MCP server.
- **`assay_core::context`** -- Claude Code session JSONL parsing (reads `~/.claude/projects/*/` session files for token diagnostics).

The new module introduces a **third concept**:
- **`assay_core::session`** -- Persistent orchestration session record. Tracks the full lifecycle of one spec implementation attempt (worktree + agent + gates + merge).

### Naming Decision

To avoid confusion, the orchestration session should use a distinct name. Options:
1. `assay_core::session` with type `OrcSession` -- terse but unclear
2. `assay_core::run` with type `RunSession` -- conflicts with `gate_run`
3. **`assay_core::session` with type `WorkSession`** -- clear, distinguishes from `AgentSession` (gate evaluation) and Claude Code sessions (diagnostics)

Recommendation: **`WorkSession`** -- it represents a unit of work against a spec.

### Type Requirements (assay-types)

```rust
// NEW in assay-types/src/work_session.rs

/// A persistent record of one spec implementation attempt.
///
/// Tracks the full lifecycle: worktree creation → agent launch →
/// gate evaluation → merge/abandon.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkSession {
    /// Unique session identifier.
    pub id: String,
    /// Spec being implemented.
    pub spec_name: String,
    /// Worktree ID (links to WorktreeRecord).
    pub worktree_id: String,
    /// Current lifecycle phase.
    pub phase: SessionPhase,
    /// Gate run IDs produced during this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gate_runs: Vec<String>,
    /// When this session was created.
    pub created_at: DateTime<Utc>,
    /// When this session ended (merged, abandoned, or failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Outcome summary (populated on completion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<SessionOutcome>,
}

/// Lifecycle phase of a work session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SessionPhase {
    /// Setting up worktree and environment.
    Setup,
    /// Agent is implementing the spec.
    Working,
    /// Running gate evaluation.
    Gating,
    /// Gates passed, awaiting merge approval.
    AwaitingMerge,
    /// Session completed (see outcome).
    Completed,
}

/// How a session ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SessionOutcome {
    /// Successfully merged to target branch.
    Merged { target_branch: String },
    /// Gates failed and session was abandoned.
    GateFailed { gate_run_id: String },
    /// User cancelled the session.
    Cancelled,
    /// Agent timed out.
    TimedOut,
    /// Agent process errored.
    Errored { message: String },
}
```

### Module API (assay-core::session)

```rust
/// Create a new work session.
pub fn create(
    assay_dir: &Path,
    spec_name: &str,
    worktree_id: &str,
) -> Result<WorkSession>

/// Update the phase of a work session.
pub fn update_phase(
    assay_dir: &Path,
    session_id: &str,
    phase: SessionPhase,
) -> Result<WorkSession>

/// Complete a work session with an outcome.
pub fn complete(
    assay_dir: &Path,
    session_id: &str,
    outcome: SessionOutcome,
) -> Result<WorkSession>

/// Record a gate run ID against a session.
pub fn add_gate_run(
    assay_dir: &Path,
    session_id: &str,
    gate_run_id: &str,
) -> Result<()>

/// Load a work session by ID.
pub fn load(assay_dir: &Path, session_id: &str) -> Result<WorkSession>

/// List all work sessions, optionally filtered by phase.
pub fn list(
    assay_dir: &Path,
    filter_phase: Option<SessionPhase>,
) -> Result<Vec<WorkSession>>

/// List active sessions (Setup, Working, Gating, AwaitingMerge).
pub fn list_active(assay_dir: &Path) -> Result<Vec<WorkSession>>
```

### Storage Design

```
.assay/
  sessions/                     # NEW directory
    <session-id>.json           # One file per work session
```

Same atomic-write pattern as history and worktree records.

### Relationship to Existing Sessions

```
WorkSession (orchestration lifecycle)
  ├── worktree_id → WorktreeRecord (git worktree)
  ├── gate_runs[] → GateRunRecord (gate results, via history module)
  └── during Gating phase:
        └── AgentSession (in-memory, MCP server)
              └── finalized → GateRunRecord → added to gate_runs[]
```

### Integration Points

| Component | Change Type | Details |
|-----------|-------------|---------|
| assay-types | New module `work_session.rs` | `WorkSession`, `SessionPhase`, `SessionOutcome` |
| assay-core | New module `session/mod.rs` | `create`, `update_phase`, `complete`, `load`, `list` |
| assay-core | `lib.rs` | Add `pub mod session;` |

**Note:** This creates a name collision with `assay_core::gate::session` (the existing gate evaluation session submodule). The gate session submodule should be referenced as `gate::session` and the new top-level module as `session`. No rename needed -- Rust's module paths disambiguate.

---

## New Component 4: Diff/Context Module (`assay_core::diff`)

### Purpose

Assemble diff and context information for gate evaluation. When an agent finishes working in a worktree, the gate evaluator needs to understand what changed relative to the base branch. This module collects diffs, changed file lists, and relevant context.

### Where It Lives

**`crates/assay-core/src/diff/mod.rs`**

### Type Requirements (assay-types)

```rust
// NEW in assay-types/src/diff.rs

/// Summary of changes in a worktree relative to its base.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffSummary {
    /// Base ref (branch/commit) the diff is against.
    pub base_ref: String,
    /// Head commit in the worktree.
    pub head_ref: String,
    /// Files added.
    pub added: Vec<String>,
    /// Files modified.
    pub modified: Vec<String>,
    /// Files deleted.
    pub deleted: Vec<String>,
    /// Total lines added across all files.
    pub lines_added: usize,
    /// Total lines deleted across all files.
    pub lines_deleted: usize,
}

/// A file-level diff with context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileDiff {
    /// File path relative to repo root.
    pub path: String,
    /// Diff status (added, modified, deleted, renamed).
    pub status: DiffStatus,
    /// Unified diff content (truncated if too large).
    pub diff: String,
    /// Whether the diff was truncated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

/// Diff status for a single file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
}
```

### Module API (assay-core::diff)

```rust
/// Compute a diff summary between base_ref and HEAD in a worktree.
pub fn summary(worktree_path: &Path, base_ref: &str) -> Result<DiffSummary>

/// Get file-level diffs with context lines.
///
/// `max_diff_bytes` caps the total diff output to prevent
/// unbounded memory usage from large changesets.
pub fn file_diffs(
    worktree_path: &Path,
    base_ref: &str,
    context_lines: usize,
    max_diff_bytes: usize,
) -> Result<Vec<FileDiff>>

/// Assemble a context document for gate evaluation.
///
/// Combines the spec, diff summary, and file diffs into a
/// structured document that an agent evaluator can assess.
pub fn assemble_gate_context(
    spec: &Spec,
    diff_summary: &DiffSummary,
    file_diffs: &[FileDiff],
) -> String
```

### Git Interaction

All diff operations shell out to `git diff` and `git diff --stat`, consistent with the worktree module's approach. The `--no-color` flag is always passed.

```rust
// Example: getting diff summary
Command::new("git")
    .args(["diff", "--stat", "--numstat", "--no-color", base_ref])
    .current_dir(worktree_path)
    .output()
```

### Size Limits

Gate context fed to agent evaluators must be bounded. Defaults:
- `max_diff_bytes`: 256 KB (configurable)
- Per-file diff truncation: 64 KB (matches existing `MAX_OUTPUT_BYTES` in `gate/mod.rs`)
- Total context document: 512 KB

### Integration Points

| Component | Change Type | Details |
|-----------|-------------|---------|
| assay-types | New module `diff.rs` | `DiffSummary`, `FileDiff`, `DiffStatus` |
| assay-core | New module `diff/mod.rs` | `summary`, `file_diffs`, `assemble_gate_context` |
| assay-core | `lib.rs` | Add `pub mod diff;` |

---

## New Component 5: TUI Gate Results Viewer

### Purpose

Display gate run results in the TUI. This is the first functional screen beyond the placeholder, laying groundwork for the orchestration dashboard.

### Current TUI State

The TUI (`crates/assay-tui/src/main.rs`) is a 42-line skeleton:
- Initializes ratatui terminal
- Renders "Assay TUI" centered text
- Handles `q` to quit
- Has a panic hook that restores terminal

### Architecture for TUI Screens

The TUI needs a screen/view abstraction before adding gate results. Recommended structure:

```
crates/assay-tui/src/
  main.rs              # Entry point, terminal setup, event loop
  app.rs               # App state, screen routing
  screens/
    mod.rs             # Screen trait/enum
    home.rs            # Landing screen (replaces current placeholder)
    gate_results.rs    # Gate run results viewer
  widgets/
    mod.rs
    criterion_row.rs   # Single criterion result row
    enforcement_badge.rs  # Required/Advisory indicator
    summary_bar.rs     # Pass/fail/skip summary
```

### Screen Trait

```rust
/// A renderable screen in the TUI.
pub trait Screen {
    /// Handle a key event, returning a possible screen transition.
    fn handle_key(&mut self, key: KeyEvent) -> Option<ScreenAction>;
    /// Render this screen to a frame.
    fn render(&self, frame: &mut Frame, area: Rect);
}

pub enum ScreenAction {
    Navigate(ScreenId),
    Quit,
}

pub enum ScreenId {
    Home,
    GateResults { spec_name: String },
}
```

### Gate Results Screen

The gate results screen loads a `GateRunRecord` (via `assay_core::history`) and renders:

1. **Header:** Spec name, run ID, timestamp, overall pass/fail
2. **Summary bar:** Passed/Failed/Skipped counts with enforcement breakdown
3. **Criteria table:** Scrollable list of criterion results with:
   - Name, enforcement level, pass/fail, duration
   - Expandable detail showing stdout/stderr/evidence
4. **Footer:** Keybindings (j/k scroll, Enter expand, q back)

### Data Loading

The TUI loads data from assay-core synchronously on screen entry. For the gate results viewer, this is a single `history::load()` call which reads one JSON file -- fast enough to not need async.

### New Dependencies

No new workspace dependencies needed. `ratatui` and `crossterm` are already available.

### Integration Points

| Component | Change Type | Details |
|-----------|-------------|---------|
| assay-tui | Refactor `main.rs` | Extract event loop, add screen routing |
| assay-tui | New `app.rs` | App state, current screen, screen transitions |
| assay-tui | New `screens/` | Screen trait, home screen, gate results screen |
| assay-tui | New `widgets/` | Reusable rendering components |
| assay-tui | `Cargo.toml` | Add `assay-core` dependency (currently only depends on ratatui/crossterm/color-eyre) |

**Critical:** The TUI currently does not depend on assay-core. Adding this dependency is required for the gate results viewer to load data.

---

## Data Flow: v0.3.0 Orchestration Pipeline

### Single-Spec Launch Flow

```
User → assay launch <spec-name>
  │
  ├── 1. config::load(root) → Config
  ├── 2. spec::load_by_name(spec_name) → Spec
  │
  ├── 3. worktree::create(repo_root, spec_name, "main", worktree_dir)
  │       └── git worktree add -b assay/<spec>-<id> <path> main
  │       └── Persists WorktreeRecord to .assay/worktrees/<id>.json
  │
  ├── 4. session::create(assay_dir, spec_name, worktree_id)
  │       └── Persists WorkSession { phase: Setup } to .assay/sessions/<id>.json
  │
  ├── 5. launcher::build_prompt(spec, feature_spec, None)
  │       └── Assembles prompt from spec criteria + context
  │
  ├── 6. launcher::launch(worktree_path, spec_name, prompt, launch_config)
  │       └── Spawns: claude --dangerously-skip-permissions -p "<prompt>"
  │       └── session::update_phase(Working)
  │       └── worktree::update_state(Active)
  │
  ├── 7. [Agent works in worktree...]
  │       └── MCP gate_run / gate_report / gate_finalize as before
  │
  ├── 8. [Agent exits or timeout]
  │       └── launcher::check_status() → Exited
  │       └── session::update_phase(Gating)
  │
  ├── 9. diff::summary(worktree_path, base_ref) → DiffSummary
  │       └── diff::file_diffs(worktree_path, base_ref, 3, 256KB) → Vec<FileDiff>
  │
  ├── 10. gate::evaluate_all(spec, worktree_path, timeout, config_timeout)
  │        └── history::save() → GateRunRecord
  │        └── session::add_gate_run(gate_run_id)
  │
  └── 11. If gates pass:
           ├── session::update_phase(AwaitingMerge)
           ├── worktree::update_state(ReadyToMerge)
           └── [Future: merge flow]
          If gates fail:
           ├── session::complete(GateFailed)
           └── worktree::update_state(Abandoned)
```

### TUI Gate Results Flow

```
User → assay tui
  └── Home screen: list specs + recent gate runs
        └── Select a spec → GateResults screen
              └── history::list(assay_dir, spec_name) → run IDs
                    └── Select run → history::load() → GateRunRecord
                          └── Render criteria table with evidence
```

---

## Build Order

The dependency chain dictates build order: types → core → surfaces.

### Phase A: Type Foundation

1. Add `worktree.rs` to assay-types (`WorktreeState`, `WorktreeRecord`)
2. Add `work_session.rs` to assay-types (`WorkSession`, `SessionPhase`, `SessionOutcome`)
3. Add `diff.rs` to assay-types (`DiffSummary`, `FileDiff`, `DiffStatus`)
4. Add `launcher.rs` to assay-types (`LaunchConfig`, `AgentProcessStatus`, `LaunchRecord`)
5. Add `WorktreeConfig` to Config, update schema snapshots
6. Wire all re-exports in `assay-types/src/lib.rs`
7. `just ready`

### Phase B: Core Modules (can partially parallelize)

8. Implement `assay_core::worktree` (create, list, update_state, remove, prune)
9. Implement `assay_core::diff` (summary, file_diffs, assemble_gate_context)
10. Implement `assay_core::launcher` (launch, check_status, kill, build_prompt)
11. Implement `assay_core::session` (create, update_phase, complete, load, list)
12. Add new error variants to `AssayError`
13. Wire modules in `assay-core/src/lib.rs`
14. `just ready`

**Parallelization:** Modules 8-11 are independent of each other at the core level. They can be built in any order. However, integration testing will need them composed.

### Phase C: TUI Gate Results

15. Refactor TUI: extract event loop, add app state
16. Implement Screen trait and screen routing
17. Add `assay-core` dependency to assay-tui
18. Implement gate results screen with criterion table
19. `just ready`

### Phase D: CLI Launch Command

20. Add `assay launch <spec>` subcommand
21. Wire worktree → launcher → session flow
22. Add `assay session {list,show}` subcommands
23. Add `assay worktree {list,remove,prune}` subcommands
24. `just ready`

### Phase E: MCP Extensions

25. Add worktree/session status tools to MCP server
26. Add diff summary tool for agent context
27. `just ready`

---

## Changes to Existing Patterns

### Pattern: CLI single-file monolith

The CLI is currently a single 76K `main.rs`. Adding launch/session/worktree subcommands will push it past maintainability limits. v0.3.0 should extract subcommand handlers into separate modules:

```
crates/assay-cli/src/
  main.rs           # Clap struct + dispatch only
  commands/
    mod.rs
    init.rs
    spec.rs
    gate.rs
    mcp.rs
    context.rs
    checkpoint.rs
    guard.rs
    launch.rs       # NEW
    session.rs      # NEW
    worktree.rs     # NEW
```

This is a refactor prerequisite, not a feature. It should happen early in v0.3.0 to avoid making the monolith worse.

### Pattern: Sync core

The launcher module introduces async into assay-core beyond the guard daemon. This establishes a clearer boundary:

- **Sync:** `spec`, `config`, `gate` (evaluation), `history`, `init`, `worktree`, `diff`, `session` (persistence)
- **Async:** `guard` (daemon event loop), `launcher` (process lifecycle with timeout monitoring)

The async modules use tokio, which is already a workspace dependency.

### Pattern: In-memory state (MCP sessions)

The MCP server holds `AgentSession`s in `Arc<Mutex<HashMap>>`. This works for gate evaluation sessions (short-lived, single-server). WorkSessions are persisted to disk and don't need in-memory state in the MCP server -- they're loaded on demand from `.assay/sessions/`.

### Pattern: .assay/ directory layout

The `.assay/` directory gains two new subdirectories:

```
.assay/
  config.toml
  .gitignore          # Must be updated to include new dirs
  specs/
  results/            # Existing: gate run history
  checkpoints/        # Existing: team state checkpoints
  worktrees/          # NEW: worktree records (JSON)
  sessions/           # NEW: work session records (JSON)
  backups/            # Existing: pruning backups
```

The `init.rs` module must be updated to create `worktrees/` and `sessions/` directories, and the `.gitignore` template must include them.

---

## Risk Assessment

### Git subprocess reliability (Medium Risk)

Shelling out to `git` for worktree operations introduces failure modes: git not installed, wrong version, repo in bad state, concurrent git operations. Mitigation: validate `git` availability on startup, use `--porcelain` output formats for parsing, handle lock contention gracefully.

### Agent process lifecycle (Medium Risk)

Managing long-running subprocesses (Claude Code can run for hours) requires careful handling of: orphaned processes after parent crash, PID reuse, signal handling. The guard daemon's PID management (`guard/pid.rs`) provides a proven pattern to follow.

### TUI state complexity (Low Risk)

The TUI starts with read-only views (gate results). No write operations, no concurrent data access. Complexity grows later with orchestration controls, but for v0.3.0 this is straightforward.

### Name collision: `session` (Low Risk)

Three session concepts is confusing. Mitigated by distinct type names (`AgentSession` for gate evaluation, `WorkSession` for orchestration lifecycle, and the context module's `SessionInfo` for Claude Code JSONL diagnostics). Module paths disambiguate at the code level.

### CLI refactor scope (Medium Risk)

Extracting a 76K monolith into modules is labor-intensive but mechanically safe (move functions, fix visibility, update paths). Risk is in missed cross-references or subtly broken behavior. Full test suite (`just ready`) catches most issues.

---

## Quality Gate Checklist

- [x] Integration points clearly identified (per-component tables)
- [x] New vs modified components explicit (5 new modules, CLI refactor, TUI overhaul)
- [x] Build order considers existing dependencies (types → core → TUI → CLI → MCP)
- [x] Data flow changes documented (orchestration pipeline, TUI data loading)
- [x] Existing patterns analyzed for compatibility (sync/async, storage, state management)
- [x] Backward compatibility analyzed (all changes are additive, new Config fields are optional)
- [x] Risk assessment for each area

---

*Research completed: 2026-03-08*
