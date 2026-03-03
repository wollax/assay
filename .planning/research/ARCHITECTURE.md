# Architecture Research -- v0.2.0

**Date:** 2026-03-02
**Scope:** v0.2.0 feature integration -- run history persistence, required/advisory gate enforcement, agent gate recording (`gate_report`), and hardening of existing codebase (20 open issues).

---

## Existing Architecture Baseline (post-v0.1.0)

### Workspace Structure

```
assay-cli ──> assay-mcp ──> assay-core ──> assay-types
assay-tui ──────────────> assay-core ──> assay-types
```

### Crate Inventory (v0.1.0)

| Crate | Purpose | Key Exports | Lines (approx) |
|-------|---------|-------------|-----------------|
| assay-types | Serializable DTOs | `Spec`, `Criterion`, `GateKind`, `GateResult`, `Config`, `GatesConfig`, `Gate`, `Review`, `Workflow` | ~130 src |
| assay-core | Domain logic | `config::load()`, `spec::{load, scan, validate}`, `gate::{evaluate, evaluate_all, evaluate_file_exists, resolve_timeout}`, `init::init()`, `error::AssayError` | ~720 src |
| assay-mcp | MCP server | `serve()`, `AssayServer` with 3 tools: `spec_list`, `spec_get`, `gate_run` | ~350 src |
| assay-cli | CLI binary | Clap subcommands: `init`, `spec {list,show}`, `gate run`, `mcp serve` | ~830 src |
| assay-tui | TUI binary | Skeleton event loop (placeholder) | ~42 src |

### Key Types and Their Relationships

```
Spec
  ├── name: String
  ├── description: String
  └── criteria: Vec<Criterion>

Criterion
  ├── name: String
  ├── description: String
  ├── cmd: Option<String>          // None = descriptive-only (skipped during eval)
  └── timeout: Option<u64>         // Per-criterion timeout override

GateKind (tagged enum)
  ├── Command { cmd: String }
  ├── AlwaysPass
  └── FileExists { path: String }

GateResult
  ├── passed: bool
  ├── kind: GateKind
  ├── stdout: String               // skip_serializing_if empty
  ├── stderr: String               // skip_serializing_if empty
  ├── exit_code: Option<i32>
  ├── duration_ms: u64
  ├── timestamp: DateTime<Utc>
  ├── truncated: bool
  └── original_bytes: Option<u64>
```

### Gate Evaluation Flow (Current)

```
evaluate_all(spec, working_dir, cli_timeout, config_timeout)
  └── for each criterion in spec.criteria:
        ├── if cmd is None → skip (increment skipped counter)
        └── if cmd is Some → evaluate(criterion, working_dir, timeout)
              └── evaluate_command(cmd, working_dir, timeout)
                    └── sh -c <cmd> → GateResult { passed, stdout, stderr, ... }

Returns → GateRunSummary { spec_name, results: Vec<CriterionResult>, passed, failed, skipped, total_duration_ms }
```

### Core Module Functions (assay-core)

| Module | Public Functions | Notes |
|--------|-----------------|-------|
| `config` | `load(root)`, `from_str(s)`, `validate(config)` | Loads `.assay/config.toml` |
| `spec` | `load(path)`, `from_str(s)`, `validate(spec)`, `scan(dir)` | `ScanResult { specs, errors }` |
| `gate` | `evaluate(criterion, dir, timeout)`, `evaluate_all(spec, dir, cli_t, cfg_t)`, `evaluate_file_exists(path, dir)`, `resolve_timeout(cli, crit, cfg)` | All synchronous |
| `init` | `init(root, options)` | Creates `.assay/` scaffold |
| `error` | `AssayError` (7 variants), `Result<T>` | `#[non_exhaustive]` |
| `review` | (empty stub) | `//! Reviews evaluate completed work...` |
| `workflow` | (empty stub) | `//! Workflows combine specs, gates...` |

### MCP Server Architecture

```rust
AssayServer { tool_router: ToolRouter<Self> }

Tools:
  spec_list()  → Vec<SpecListEntry>          // name, description, criteria_count
  spec_get(name) → Spec as JSON
  gate_run(name, include_evidence) → GateRunResponse
                                     // passed, failed, skipped, total_duration_ms
                                     // criteria: Vec<CriterionSummary>
```

All MCP tools load config + spec on each call (stateless). Gate evaluation uses `spawn_blocking` to bridge sync core into async rmcp handlers.

### Important Design Patterns

1. **Free functions, not methods.** Core logic is `pub fn evaluate(...)`, not `impl GateEvaluator`. Functional style.
2. **Stateless evaluation.** No persistent state anywhere. Each `gate_run` is fire-and-forget.
3. **`deny_unknown_fields` on DTOs.** `Spec`, `Criterion`, `Config`, `GatesConfig` all reject unknown TOML keys.
4. **Schema registry via `inventory`.** Types self-register for JSON Schema generation.
5. **Non-exhaustive error enum.** New variants can be added without breaking downstream.

---

## Feature 1: Required/Advisory Gate Enforcement

### What Changes

Currently, `evaluate_all()` treats all criteria equally -- a failure is a failure. The new `enforcement` field on `Criterion` allows criteria to be `Required` (default, current behavior) or `Advisory` (failure is reported but does not block).

### Type Changes (assay-types)

```rust
// NEW enum in assay-types/src/criterion.rs
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub enum Enforcement {
    #[default]
    Required,
    Advisory,
}

// MODIFIED: Criterion gains a new field
pub struct Criterion {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,
    // NEW field:
    #[serde(default)]  // Default = Required, omit from TOML when Required
    pub enforcement: Enforcement,
}
```

**Serde compatibility:** Using `#[serde(default)]` means existing spec files without `enforcement` will deserialize with `Enforcement::Required`, which is backward-compatible. The `deny_unknown_fields` attribute on `Criterion` means this field must be added to the struct before any TOML file can include it -- but that's the right order anyway.

**Schema impact:** `Enforcement` needs `inventory::submit!` registration. The `criterion` schema changes (new optional field). Snapshot tests in `assay-types/tests/schema_snapshots.rs` will need updating.

### Logic Changes (assay-core/gate)

The `GateRunSummary` struct needs a new counter:

```rust
pub struct GateRunSummary {
    pub spec_name: String,
    pub results: Vec<CriterionResult>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub advisory_failed: usize,  // NEW: advisory criteria that failed
    pub total_duration_ms: u64,
}
```

And `CriterionResult` needs to carry the enforcement level:

```rust
pub struct CriterionResult {
    pub criterion_name: String,
    pub enforcement: Enforcement,  // NEW: so consumers know if failure blocks
    pub result: Option<GateResult>,
}
```

The `evaluate_all()` function changes:

```rust
// In the evaluation loop, after getting gate_result:
match (gate_result.passed, criterion.enforcement) {
    (true, _) => passed += 1,
    (false, Enforcement::Required) => failed += 1,
    (false, Enforcement::Advisory) => advisory_failed += 1,
}
```

**Impact on CLI:** `print_gate_summary()` needs to display advisory failures distinctly (e.g., yellow warning count). The exit code should only be 1 when `failed > 0` (required failures), not for advisory failures.

**Impact on MCP:** `GateRunResponse` and `CriterionSummary` need the advisory count and per-criterion enforcement field. The `format_gate_response()` function updates.

**Confidence:** HIGH. This is a straightforward enum addition. The `#[serde(default)]` ensures backward compatibility. The main risk is forgetting to update all the places that check `passed/failed` counts.

### Integration Points

| Component | File | Change Type |
|-----------|------|-------------|
| assay-types | `criterion.rs` | Add `Enforcement` enum, add field to `Criterion` |
| assay-types | `lib.rs` | Re-export `Enforcement` |
| assay-core | `gate/mod.rs` | Modify `evaluate_all()`, `GateRunSummary`, `CriterionResult` |
| assay-cli | `main.rs` | Modify `stream_criterion()`, `print_gate_summary()`, `StreamCounters` |
| assay-mcp | `server.rs` | Modify `GateRunResponse`, `CriterionSummary`, `format_gate_response()` |
| assay-types | `tests/schema_snapshots.rs` | Update schema snapshots |

---

## Feature 2: Run History Persistence

### Where It Lives

Run history is domain logic that persists evaluation results. It belongs in **assay-core as a new module**: `assay_core::history`.

**Rationale:**
- History is not a DTO (not assay-types)
- History is not transport/presentation (not assay-mcp or assay-cli)
- History reads/writes files in `.assay/results/` -- this is domain-level I/O, same layer as `config::load()` and `spec::load()`
- Both CLI and MCP need to record and query history -- core is the shared layer

### Storage Design

```
.assay/
  config.toml
  specs/
  results/              # NEW: run history directory
    <spec-name>/        # One subdirectory per spec
      <timestamp>.json  # One file per run
```

Each run file is a serialized `GateRunRecord`:

```rust
// NEW type in assay-types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateRunRecord {
    /// Spec name that was evaluated.
    pub spec_name: String,
    /// Who triggered this run.
    pub trigger: RunTrigger,
    /// The full gate run summary.
    pub summary: GateRunSummary,  // Existing type, moved to assay-types or kept in core
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// Optional run identifier (UUID or monotonic).
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RunTrigger {
    Cli,
    Mcp { tool: String },
    Agent { agent_id: Option<String> },
}
```

**Key Decision: GateRunSummary location.**

Currently `GateRunSummary` lives in `assay-core::gate` with `#[derive(Serialize)]` only (no `Deserialize`). For history, it must be round-trippable. Two options:

1. **Move to assay-types.** Makes it a shared DTO. But it currently has a doc comment saying "computed summary type, not persisted DTO. Lives in assay-core." This was a deliberate v0.1 design choice.

2. **Keep in assay-core, add Deserialize.** Add `Deserialize` + `JsonSchema` to `GateRunSummary` and `CriterionResult`. The history module in core can use them directly. The MCP crate already depends on assay-core, so it can access the types.

**Recommendation: Option 2.** Keep `GateRunSummary` in core, add `Deserialize + JsonSchema`. The `GateRunRecord` wrapper goes in assay-types since it's the persisted DTO that both CLI and MCP consume. The `summary` field inside the record references the core type -- but since `GateRunSummary` lives in `assay-core` and `GateRunRecord` would need to go in a crate that assay-core depends on (assay-types), this creates a circular dependency.

**Revised recommendation: Put `GateRunRecord` in assay-core::history, not assay-types.** The history module owns both the persistence logic and the record type. Consumers (CLI, MCP) already depend on assay-core and can access `assay_core::history::GateRunRecord`. This avoids the circular dependency.

Alternatively, move `GateRunSummary` and `CriterionResult` to assay-types (making them shared DTOs). This is the cleaner long-term approach since they're already serializable and used across crate boundaries.

**Final recommendation: Move `GateRunSummary` and `CriterionResult` to assay-types.** Put `GateRunRecord` (the persistence wrapper) in assay-types too. Add `Deserialize + JsonSchema` to both. The history persistence logic (`save()`, `list()`, `load()`) stays in `assay-core::history`. This maintains the architecture: types in assay-types, logic in assay-core.

### History Module API (assay-core::history)

```rust
// assay_core::history

/// Save a gate run result to the history directory.
pub fn save(root: &Path, record: &GateRunRecord) -> Result<PathBuf>

/// List all run records for a spec, most recent first.
pub fn list(root: &Path, spec_name: &str) -> Result<Vec<GateRunRecord>>

/// List the N most recent runs for a spec.
pub fn list_recent(root: &Path, spec_name: &str, limit: usize) -> Result<Vec<GateRunRecord>>

/// Load a specific run record by ID.
pub fn load(root: &Path, spec_name: &str, run_id: &str) -> Result<GateRunRecord>

/// Get the most recent run for a spec.
pub fn latest(root: &Path, spec_name: &str) -> Result<Option<GateRunRecord>>
```

### File Naming Convention

Filename: `{ISO8601_timestamp}_{short_id}.json`

Example: `2026-03-02T14-30-00Z_a1b2c3.json`

Using timestamp prefix ensures `ls` gives chronological order. The short ID (first 6 chars of a UUID) ensures uniqueness for concurrent runs.

### `.gitignore` Alignment

The `init` module already generates `.assay/.gitignore` with `results/` as an ignored directory. This is forward-compatible -- history files will be gitignored by default.

```
# From init.rs render_gitignore()
# Results from gate evaluations
results/
```

### Integration Points

| Component | File | Change Type |
|-----------|------|-------------|
| assay-types | `lib.rs` | Add `GateRunSummary`, `CriterionResult`, `GateRunRecord`, `RunTrigger` |
| assay-core | `lib.rs` | Add `pub mod history;` |
| assay-core | `history/mod.rs` | NEW: `save()`, `list()`, `list_recent()`, `load()`, `latest()` |
| assay-core | `gate/mod.rs` | Remove `GateRunSummary` and `CriterionResult` (moved to types) |
| assay-core | `error.rs` | Add `HistoryIo` and `HistoryParse` variants to `AssayError` |
| assay-cli | `main.rs` | After `evaluate_all()`, call `history::save()` |
| assay-mcp | `server.rs` | After `gate_run`, call `history::save()` |

**Confidence:** HIGH. File-based JSON persistence is straightforward. The main complexity is the type relocation from core to types.

---

## Feature 3: Agent Gate Recording (`gate_report` MCP Tool)

### What It Does

The `gate_report` tool allows agents to report gate results they evaluated themselves (outside Assay's command execution). This is the "agent-as-evaluator" pattern: the agent runs a test, observes the outcome, and reports back to Assay for recording.

### Where It Lives

- **Tool handler:** `assay-mcp/src/server.rs` -- new `#[tool]` method on `AssayServer`
- **Types:** `assay-types` -- `AgentGateReport` parameter type
- **Persistence:** Uses `assay_core::history::save()` -- same persistence as `gate_run`

### New Types (assay-types)

```rust
/// An agent-reported gate evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentGateReport {
    /// The spec this report is for.
    pub spec_name: String,
    /// The criterion name being reported on.
    pub criterion_name: String,
    /// Whether the criterion passed.
    pub passed: bool,
    /// Agent-provided evidence/reasoning.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub evidence: String,
    /// Optional agent identifier.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent_id: Option<String>,
}
```

### MCP Tool Implementation

```rust
// In server.rs, within #[tool_router] impl AssayServer:

#[derive(Deserialize, JsonSchema)]
struct GateReportParams {
    #[schemars(description = "Spec name the report is for")]
    spec_name: String,
    #[schemars(description = "Criterion name being reported")]
    criterion_name: String,
    #[schemars(description = "Whether the criterion passed")]
    passed: bool,
    #[schemars(description = "Agent-provided evidence or reasoning")]
    #[serde(default)]
    evidence: String,
}

#[tool(description = "Report a gate evaluation result performed by the agent. \
    Use this when you have evaluated a criterion yourself (e.g., reviewed code, \
    checked documentation quality) and want to record the result.")]
async fn gate_report(&self, params: Parameters<GateReportParams>) -> Result<CallToolResult, McpError> {
    // 1. Load config + spec (validate spec/criterion exist)
    // 2. Build GateRunRecord with RunTrigger::Agent
    // 3. Save via history::save()
    // 4. Return confirmation
}
```

### Data Flow

```
Agent calls gate_report(spec_name, criterion_name, passed, evidence)
  └── MCP handler validates spec + criterion exist
        └── Constructs GateResult with GateKind::AgentReport  // NEW GateKind variant
              └── Wraps in GateRunRecord { trigger: RunTrigger::Agent }
                    └── history::save(root, record)
                          └── Writes .assay/results/<spec>/<timestamp>.json
```

This means `gate_report` results appear in the same history store as `gate_run` results. They can be queried the same way.

### New GateKind Variant

```rust
// In assay-types/src/gate.rs
pub enum GateKind {
    Command { cmd: String },
    AlwaysPass,
    FileExists { path: String },
    AgentReport,  // NEW: result reported by an agent, not executed by Assay
}
```

### History Query MCP Tool

A companion `gate_history` tool lets agents query past results:

```rust
#[derive(Deserialize, JsonSchema)]
struct GateHistoryParams {
    #[schemars(description = "Spec name to query history for")]
    spec_name: String,
    #[schemars(description = "Maximum number of recent runs to return (default: 5)")]
    #[serde(default = "default_history_limit")]
    limit: usize,
}

#[tool(description = "Query gate run history for a spec. Returns recent run results \
    including pass/fail status, timestamps, and who triggered each run.")]
async fn gate_history(&self, params: Parameters<GateHistoryParams>) -> Result<CallToolResult, McpError> {
    // Delegate to history::list_recent()
}
```

### Integration Points

| Component | File | Change Type |
|-----------|------|-------------|
| assay-types | `gate.rs` | Add `GateKind::AgentReport` variant |
| assay-mcp | `server.rs` | Add `gate_report` tool + `gate_history` tool |
| assay-mcp | `server.rs` | Add `GateReportParams`, `GateHistoryParams` structs |
| assay-core | `gate/mod.rs` | No change to evaluation logic (agent bypass) |
| assay-core | `history/mod.rs` | Used by gate_report for persistence |

**Confidence:** HIGH. The MCP tool pattern is well-established from `gate_run`. The `gate_report` tool is simpler since it doesn't need `spawn_blocking`.

---

## Feature 4: Hardening (Open Issues)

### Issue Inventory and Architectural Grouping

The 20 open issues cluster into 5 architectural groups:

#### Group A: MCP Server Hardening (7 issues)

| # | Title | Effort | Dependencies |
|---|-------|--------|--------------|
| 35 | MCP tool handlers have zero direct tests | Medium | None |
| 34 | Tool descriptions have minor inaccuracies | Small | None |
| 33 | SpecNotFound variant declared but never constructed | Small | None |
| 32 | Response structs lack field-level documentation | Small | None |
| 31 | gate_run has no timeout parameter for agents | Small | None |
| 30 | Failure reason only checks stderr, misses stdout-only errors | Small | None |
| 38 | spec_list silently discards scan errors | Small | None |

**Architectural impact:** These are localized to `assay-mcp/src/server.rs`. Issue #31 (timeout param) modifies `GateRunParams` -- this should be coordinated with v0.2 MCP changes to avoid churn. Issue #33 (SpecNotFound) may be resolved naturally when `gate_report` validates spec existence.

#### Group B: Core Robustness (3 issues)

| # | Title | Effort | Dependencies |
|---|-------|--------|--------------|
| 37 | resolve_working_dir does not validate path exists | Small | None |
| 36 | Unnecessary clone intermediaries in gate_run | Small | None |
| 17 | Establish error type hierarchy in assay-core | Medium | None |

**Architectural impact:** Issue #17 is already resolved -- the error hierarchy exists (`AssayError` with 7 variants). May need to be closed. Issue #37 affects `server.rs::resolve_working_dir()` and `main.rs::load_gate_context()`. Issue #36 is a refactor of `gate_run` handler.

#### Group C: CLI Improvements (1 issue)

| # | Title | Effort | Dependencies |
|---|-------|--------|--------------|
| 13 | CLI main() should return Result for error propagation | Medium | None |

**Architectural impact:** Changes `main()` from `#[tokio::main] async fn main()` to returning `Result`. Replaces `process::exit(1)` calls with `?` propagation. This is a cross-cutting refactor of `main.rs`.

#### Group D: TUI Fix (1 issue)

| # | Title | Effort | Dependencies |
|---|-------|--------|--------------|
| 12 | Use ratatui::try_init() instead of panicking init() | Small | None |

**Architectural impact:** Single-line change in `assay-tui/src/main.rs`.

#### Group E: Build/CI Hardening (3 issues)

| # | Title | Effort | Dependencies |
|---|-------|--------|--------------|
| 16 | Tighten deny.toml source controls from warn to deny | Small | None |
| 15 | Tighten deny.toml multiple-versions from warn to deny | Small | Depends on dep updates |
| 14 | CI plugin validation should check schema not just JSON syntax | Medium | None |

**Architectural impact:** None on Rust code. These are CI/tooling configuration changes.

#### Group F: Stale/Resolved Issues (5 issues)

| # | Title | Status | Notes |
|---|-------|--------|-------|
| 23 | spike.rs: Anchor forward reference | Stale | spike.rs was replaced in v0.1 |
| 22 | lib.rs: Update module doc | Stale | Likely resolved during v0.1 |
| 21 | lib.rs: Fix doc comment | Stale | Likely resolved during v0.1 |
| 20 | spike.rs: Add tracing for termination | Stale | spike.rs was replaced |
| 19 | spike.rs: Remove double-logged error | Stale | spike.rs was replaced |

**Recommendation:** Close issues #19-23 as stale. They reference `spike.rs` which was replaced by the production `server.rs` during v0.1.0.

### Hardening Build Order

Issues should be resolved in this order to minimize churn:

1. **Close stale issues** (#19-23) -- zero code changes
2. **Group E** (CI hardening) -- independent of feature work
3. **Group D** (TUI fix) -- one-line change, independent
4. **Group B** (core robustness) -- before feature work touches core
5. **Group A** (MCP hardening) -- coordinate with v0.2 MCP changes
6. **Group C** (CLI refactor) -- after feature work stabilizes CLI

---

## Data Flow: Complete v0.2 Pipeline

### Agent calls gate_run (existing + history)

```
Agent → MCP gate_run(spec_name, include_evidence)
  └── handler loads config + spec
        └── spawn_blocking → evaluate_all(spec, dir, None, cfg_timeout)
              └── Returns GateRunSummary
                    ├── format_gate_response() → GateRunResponse → JSON to agent
                    └── NEW: wrap in GateRunRecord { trigger: Mcp { tool: "gate_run" } }
                          └── history::save(root, record)
                                └── .assay/results/<spec>/<timestamp>.json
```

### Agent calls gate_report (new)

```
Agent → MCP gate_report(spec_name, criterion_name, passed, evidence)
  └── handler loads config + spec (validates both exist)
        └── Constructs single-criterion GateRunRecord
              ├── GateResult { passed, kind: AgentReport, ... }
              ├── trigger: RunTrigger::Agent
              └── history::save(root, record)
                    └── .assay/results/<spec>/<timestamp>.json
  └── Returns confirmation JSON to agent
```

### Agent queries gate_history (new)

```
Agent → MCP gate_history(spec_name, limit)
  └── handler calls history::list_recent(root, spec_name, limit)
        └── Reads .assay/results/<spec>/*.json
              └── Deserializes, sorts by timestamp desc, returns first N
  └── Returns Vec<GateRunRecord> as JSON to agent
```

### CLI runs gates (existing + history)

```
User → assay gate run <spec>
  └── load_gate_context() → (root, config, working_dir, timeout)
        └── for each criterion: stream_criterion() → evaluate()
              └── Prints live results to terminal
        └── NEW: Build GateRunSummary from counters
              └── Wrap in GateRunRecord { trigger: Cli }
                    └── history::save(root, record)
```

**Note:** The CLI currently uses `stream_criterion()` for individual criterion evaluation, not `evaluate_all()`. To persist history, the CLI needs to either:
- Switch to using `evaluate_all()` and then format results (losing streaming display), or
- Build a `GateRunSummary` from the `StreamCounters` after all criteria are evaluated

**Recommendation:** Build `GateRunSummary` post-hoc from the individual results collected during streaming. The CLI can accumulate `CriterionResult` entries alongside the streaming display, then save the complete record at the end. This preserves the streaming UX.

---

## New Components Summary

### New Modules

| Module | Location | Purpose |
|--------|----------|---------|
| `assay_core::history` | `crates/assay-core/src/history/mod.rs` | Run history persistence: save, list, load |

### New Types in assay-types

| Type | Location | Purpose |
|------|----------|---------|
| `Enforcement` | `criterion.rs` | Required/Advisory enum for gate criteria |
| `GateRunSummary` | `lib.rs` (moved from core) | Aggregate evaluation results (now round-trippable) |
| `CriterionResult` | `lib.rs` (moved from core) | Single criterion result with enforcement |
| `GateRunRecord` | `lib.rs` (new) | Persisted run record with trigger + summary |
| `RunTrigger` | `lib.rs` (new) | Who triggered the run: Cli, Mcp, Agent |

### New GateKind Variant

| Variant | Location | Purpose |
|---------|----------|---------|
| `AgentReport` | `gate.rs` | Results reported by agents, not executed by Assay |

### New MCP Tools

| Tool | Purpose | Parameters |
|------|---------|------------|
| `gate_report` | Record agent-evaluated gate results | spec_name, criterion_name, passed, evidence |
| `gate_history` | Query past gate run results | spec_name, limit |

### New Error Variants

| Variant | Purpose |
|---------|---------|
| `HistoryIo` | File I/O errors in history operations |
| `HistoryParse` | Deserialization errors reading history files |
| `CriterionNotFound` | gate_report references nonexistent criterion |

### New Dependencies

None expected. All required crates (`chrono`, `serde_json`, `uuid`) are either already workspace deps or can be avoided (timestamp + random suffix instead of UUID).

---

## Suggested Build Order

The build order respects the dependency chain: types before core before surfaces.

### Phase 1: Hardening Prerequisite

Resolve open issues that would cause churn if done after feature work:

1. Close stale issues (#19-23)
2. Fix #33 (SpecNotFound construction) -- will be needed by `gate_report` validation
3. Fix #30 (failure reason from stdout) -- before changing response format
4. Fix #37 (validate working_dir exists) -- before adding more paths
5. Fix #12 (TUI try_init) -- independent, quick win
6. Fix #31 (gate_run timeout param) -- before adding more MCP params

### Phase 2: Type Foundation

Add new types that features depend on:

1. Add `Enforcement` enum to `assay-types/src/criterion.rs`
2. Add `enforcement` field to `Criterion`
3. Move `GateRunSummary` and `CriterionResult` from `assay-core::gate` to `assay-types`
4. Add `Deserialize`, `JsonSchema` derives to moved types
5. Add `enforcement: Enforcement` field to `CriterionResult`
6. Add `advisory_failed` counter to `GateRunSummary`
7. Add `GateKind::AgentReport` variant
8. Add `GateRunRecord` and `RunTrigger` types
9. Update schema snapshots
10. Run `just ready`

### Phase 3: Core Logic

Implement core changes:

1. Update `gate::evaluate_all()` for enforcement-aware counting
2. Implement `assay_core::history` module (save, list, load, latest)
3. Add new error variants to `AssayError`
4. Wire history saving into gate evaluation flow (optional: core doesn't save automatically, surfaces do)
5. Run `just ready`

### Phase 4: MCP Surface

Add new MCP tools and update existing ones:

1. Update `gate_run` handler to save history
2. Update `GateRunResponse` / `CriterionSummary` for enforcement
3. Add `gate_report` tool
4. Add `gate_history` tool
5. Add `timeout` parameter to `gate_run` (issue #31)
6. Fix remaining MCP hardening issues (#32, #34, #35, #36, #38)
7. Run `just ready`

### Phase 5: CLI Surface

Update CLI for new features:

1. Update `stream_criterion()` to collect results for history
2. Save `GateRunRecord` after gate runs complete
3. Add `assay gate history <spec>` subcommand
4. Update `print_gate_summary()` for advisory counts
5. Fix #13 (CLI Result return type) if not disruptive
6. Run `just ready`

### Phase 6: CI/Build Hardening

1. Fix #14 (schema validation in CI)
2. Fix #15, #16 (deny.toml tightening)
3. Final `just ready`

---

## Risk Assessment

### Type Relocation (Medium Risk)

Moving `GateRunSummary` and `CriterionResult` from `assay-core::gate` to `assay-types` touches every consumer. The types are used in:
- `assay-core::gate::evaluate_all()` (return type)
- `assay-mcp::server::format_gate_response()` (input)
- `assay-mcp::server::tests` (test construction)
- `assay-cli::main.rs` (JSON serialization)

Mitigation: This is a compile-error-driven refactor. Move the types, fix imports until `just ready` passes.

### `deny_unknown_fields` Interaction (Low Risk)

`Criterion` has `#[serde(deny_unknown_fields)]`. Adding `enforcement` is safe -- it's a new field in the struct, not an unknown field in existing TOML. Existing TOML files that omit `enforcement` will get the default (`Required`). New TOML files that include `enforcement` will parse correctly.

### History File Growth (Low Risk)

Each run produces one JSON file (~1-5 KB). At 100 runs/day, that's ~500 KB/day. The `.gitignore` already excludes `results/`. For v0.2, no automatic pruning is needed. A `gate_history --prune` command can be added later.

### Backward Compatibility (Low Risk)

All changes are additive:
- New `Enforcement` field has a default value
- New `GateKind::AgentReport` variant is additive to a non-exhaustive-equivalent tagged enum
- New MCP tools don't affect existing tools
- History files are new (no migration needed)

The only potential breaking change is moving `GateRunSummary` from `assay_core::gate` to `assay_types`. Any external consumers importing `assay_core::gate::GateRunSummary` would need to update imports. Since this is a pre-1.0 workspace with no external consumers, this is acceptable.

---

## Quality Gate Checklist

- [x] Integration points clearly identified (per-component tables for each feature)
- [x] New vs modified components explicit (new modules, moved types, modified functions)
- [x] Build order considers existing dependencies (types -> core -> MCP -> CLI)
- [x] Data flow changes documented (gate_run + save, gate_report, gate_history)
- [x] Open issues grouped architecturally with resolution order
- [x] Backward compatibility analyzed
- [x] Risk assessment for each feature area

---

*Research completed: 2026-03-02*
