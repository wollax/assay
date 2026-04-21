# Architecture: Assay

Assay is a spec-driven development kit for agentic workflows, built in Rust. It provides quality gates, context management, and orchestration for AI coding agents. Seven crates form a layered architecture where types flow upward, domain logic lives in the middle, and presentation crates are thin wrappers.

---

## Crate Dependency Graph

```
assay-cli ──┐
assay-tui ──┤
            ├──> assay-mcp ──> assay-backends ──> assay-core ──> assay-harness ──> assay-types
            │                                  └───────────────────────────────────────┘
            └──> assay-core ──> assay-harness ──> assay-types
```

| Crate | Role | Key Dependencies |
|-------|------|------------------|
| `assay-types` | Shared serializable domain types | serde, schemars, inventory, chrono, semver |
| `assay-core` | Domain logic (specs, gates, reviews, workflows, context, merge) | assay-types, assay-harness, cupel, tokio, notify |
| `assay-harness` | Agent adapter implementations | assay-types |
| `assay-backends` | Remote state backend implementations (feature-gated) | assay-types, assay-core, reqwest |
| `assay-mcp` | MCP server with 29+ tool handlers and signal endpoint | assay-core, assay-backends, rmcp, axum |
| `assay-cli` | CLI binary (thin wrapper) | assay-core, clap, dialoguer |
| `assay-tui` | TUI binary (thin wrapper) | assay-core, ratatui, crossterm, color-eyre |

**Design rule:** Dependency arrows point inward. `assay-types` has zero internal dependencies. Presentation crates never contain business logic.

---

## Key Abstractions

### HarnessProvider

Object-safe trait for agent runtime adapters. Decouples the pipeline from any specific AI coding tool.

```
trait HarnessProvider: Send + Sync {
    fn write_harness(profile, working_dir) -> Result<Vec<String>>
    fn write_harness_streaming(profile, working_dir, prompt) -> Result<Vec<String>>
}
```

| Implementation | Crate | Target Agent |
|----------------|-------|-------------|
| `ClaudeProvider` | assay-harness | Claude Code |
| `CodexProvider` | assay-harness | OpenAI Codex |
| `OpenCodeProvider` | assay-harness | OpenCode |
| `NullProvider` | assay-types | Testing |

Each provider writes platform-specific configuration files (e.g., `.claude/settings.json`, `CLAUDE.md`) and returns the CLI arguments needed to launch the agent subprocess.

### StateBackend

Object-safe `Send + Sync` trait for orchestrator state persistence. Uses a capability-based design where backends advertise supported operations via `CapabilitySet`.

| Capability | Methods |
|-----------|---------|
| Always available | `push_session_event()`, `read_run_state()` |
| `supports_messaging` | `deliver_message()`, `drain_inbox()` |
| `supports_gossip_manifest` | `write_gossip_manifest()`, `read_gossip_manifest()` |
| `supports_checkpoints` | `write_checkpoint()` |
| `supports_peer_registry` | `register_peer()`, `list_peers()`, `unregister_peer()` |

| Implementation | Crate | Backing Store |
|----------------|-------|--------------|
| `LocalFsBackend` | assay-core | Filesystem (`.assay/orchestrator/`) |
| `NoopBackend` | assay-core | /dev/null (testing) |
| `GitHubBackend` | assay-backends | GitHub Issues API |
| `LinearBackend` | assay-backends | Linear project API |
| `SshBackend` | assay-backends | SSH remote filesystem |
| `SmeltBackend` | assay-backends | Smelt HTTP event API |

### SchemaRegistry

Compile-time auto-discovery registry using `inventory::submit!`. Every serializable domain type registers itself with a kebab-case name and a schema generator function. At runtime, `schema_registry::all_entries()` yields `SchemaEntry` items for JSON Schema generation.

This pattern eliminates manual registration lists. Adding a new type with the `inventory::submit!` macro is sufficient for it to appear in generated schemas.

### PipelineStage

Enum providing structured error context for the end-to-end pipeline. Each variant corresponds to a phase of execution, enabling precise error attribution.

| Variant | Phase |
|---------|-------|
| `SpecLoad` | Spec file parsing and validation |
| `WorktreeCreate` | Git worktree isolation |
| `HarnessConfig` | Agent configuration writing |
| `AgentLaunch` | Subprocess spawning |
| `GateEvaluate` | Quality gate evaluation |
| `MergeCheck` | Merge conflict detection |

`PipelineError` wraps `AssayError` with the active `PipelineStage` for contextual error reporting.

---

## Domain Model

### Core Types and Relationships

```
Config (project-level)
  ├── GatesConfig         — gate execution defaults (timeout, evaluator, retries)
  ├── GuardConfig         — daemon thresholds (soft/hard, poll interval, circuit breaker)
  ├── SessionsConfig      — staleness, max count, max age
  ├── WorktreeConfig      — worktree base directory
  ├── ProviderConfig      — AI model selection (planning, execution, review)
  └── WorkflowConfig      — solo dev loop (auto_isolate, protected_branches, uat)

Milestone
  └── ChunkRef[]          — slug, order, depends_on
       └── GatesSpec (gates.toml)
            ├── GateSection         — enforcement defaults
            ├── SpecPreconditions   — requires (commands, status checks)
            ├── Criterion[]         — name, description, kind, when, enforcement
            ├── extends             → parent GatesSpec (composition)
            └── include             → CriteriaLibrary[] (composition)

FeatureSpec (spec.toml, IEEE 830/29148)
  ├── Requirement[]        — REQ-AREA-NNN, obligation, priority
  └── AcceptanceCriterion[] — Gherkin, EARS, or Plain format

WorkSession (persistent lifecycle)
  ├── SessionPhase: Created → AgentRunning → GateEvaluated → Completed | Abandoned
  ├── AgentInvocation      — command, model
  ├── ToolCallSummary      — total, by_tool, error_count
  └── gate_runs[]          → GateRunRecord

GateEvalContext (crash-recoverable in-memory state)
  ├── command_results[]    → CriterionResult
  ├── agent_evaluations{}  → AgentEvaluation[]
  └── diff                 — captured git changes

GateRunRecord (immutable on-disk artifact)
  ├── GateRunSummary
  │    ├── CriterionResult[] → GateResult (pass/fail)
  │    └── EnforcementSummary
  └── DiffTruncation

ResolvedGate (post-composition)
  └── ResolvedCriterion[]  — criterion + CriterionSource (Own / Parent / Library)

RunManifest (orchestration)
  ├── ManifestSession[]    — spec, settings, hooks, prompt_layers, file_scope, depends_on
  └── OrchestratorMode     — Dag, Mesh, or Gossip
```

### Key Type Hierarchies

| Type | Variants | Purpose |
|------|----------|---------|
| `GateKind` | Command, AlwaysPass, FileExists, AgentReport, EventCount, NoToolErrors | How a criterion is evaluated |
| `When` | SessionEnd, AfterToolCalls{n}, OnEvent{event_type} | When a criterion fires |
| `Enforcement` | Required, Advisory | Whether failure blocks progression |
| `SpecStatus` | Draft → Proposed → Planned → InProgress → Verified → Deprecated | IEEE 830 lifecycle |
| `GateSpecStatus` | Draft → Ready → Approved → Verified | Workflow lifecycle |
| `MilestoneStatus` | Draft → InProgress → Verify → Complete | Milestone lifecycle |
| `SessionPhase` | Created → AgentRunning → GateEvaluated → Completed / Abandoned | Session state machine |
| `AgentEvent` | ToolCalled, ToolResult, TurnEnded, SessionStopped, TextDelta, TextBlock | Streaming event types |
| `StateBackendConfig` | LocalFs, Linear, GitHub, Ssh, Smelt, Custom | Backend selection |

---

## Data Flow

### Spec to Gate to Result to History

```
1. Config Loading
   .assay/config.toml → config::load() → Config

2. Spec Resolution
   RunManifest → ManifestSession.spec
     → spec::load_spec_entry_with_diagnostics() → SpecEntry (Legacy | Directory)
     → spec::compose::resolve() → ResolvedGate (if extends/include present)

3. Worktree Isolation
   worktree::create(spec_slug, base_branch) → WorktreeInfo
     → writes .assay/worktree.json metadata

4. Harness Configuration
   HarnessProvider::write_harness_streaming(profile, working_dir, prompt)
     → writes agent config files → returns Vec<String> CLI args

5. Agent Execution
   pipeline::launch_agent_streaming(cli_args) → subprocess
     → claude_stream parser → AgentEvent stream
     → pipeline_checkpoint consumes events, fires mid-session checkpoints

6. Gate Evaluation
   gate::evaluate_all(spec, working_dir, config)
     → per Criterion:
       Command   → spawn shell, check exit code
       FileExists → check path existence
       AgentReport → evaluator subprocess or manual gate_report
       EventCount → count matching events
       NoToolErrors → check tool error absence
     → CriterionResult[] → GateRunSummary

7. Result Persistence
   history::save(assay_dir, record)
     → .assay/history/<spec>/<run-id>.json
   gate::session::finalize() → GateRunRecord
   work_session::transition(GateEvaluated)

8. Auto-Promotion (optional)
   spec::auto_promote_on_pass() → updates gates.toml status to Verified
   FeatureSpec.auto_promote → spec::promote::promote_spec()

9. Merge Check
   merge::merge_check(base, head) → MergeCheck
   merge::merge_propose() → MergeProposal (push + PR)

10. PR Creation
    pr::create_pr(milestone) → invokes gh CLI → sets pr_number/pr_url
```

### Multi-Session Orchestration

The orchestrator extends the single-session pipeline to parallel execution:

1. `RunManifest` with mode (Dag/Mesh/Gossip) is loaded and validated
2. `orchestrate::dag::build_dag()` produces a validated DAG from session dependencies
3. `orchestrate::executor::run()` dispatches sessions in parallel respecting the DAG
4. Each session executes stages 2-8 of the single-session pipeline
5. `orchestrate::merge_runner::run()` merges results in computed order (CompletionTime or FileOverlap)
6. Optional AI-based conflict resolution via `conflict_resolver`
7. `OrchestratorStatus` is persisted via the active `StateBackend`

### Context Diagnostics Flow

```
context::discovery::find_session() → session JSONL path
  → context::parser::parse_entries() → Vec<SessionEntry>
  → context::diagnostics::analyze() → DiagnosticsReport
  → context::tokens::estimate() → TokenEstimate
  → context::budgeting → cupel pipeline for token budget allocation
  → context::pruning::prune() → PruneReport (dry-run or execute)
```

Six composable pruning strategies organized into three tiers:

| Tier | Strategies | Approach |
|------|-----------|----------|
| Gentle | ProgressCollapse, SystemReminderDedup | Remove redundancy |
| Standard | + MetadataStrip, StaleReads | Remove low-value content |
| Aggressive | + ThinkingBlocks, ToolOutputTrim | Remove verbose outputs |

---

## MCP API

The MCP server (`assay mcp serve`) exposes 29+ tools over JSON-RPC via stdio transport. A separate signal HTTP endpoint runs on a configurable port.

### Tool Catalog

| Tool | Category | Purpose |
|------|----------|---------|
| `spec_list` | Spec | Discover available specs |
| `spec_get` | Spec | Read a full spec definition |
| `spec_validate` | Spec | Static spec validation |
| `spec_create` | Spec | Create a chunk spec (gates.toml) |
| `spec_resolve` | Spec | Resolve effective criteria with source annotations |
| `gate_run` | Gate | Evaluate quality gate criteria |
| `gate_evaluate` | Gate | Headless evaluator subprocess |
| `gate_report` | Gate | Submit agent evaluation for a criterion |
| `gate_finalize` | Gate | Finalize session, persist as GateRunRecord |
| `gate_history` | Gate | Query past gate run results |
| `gate_wizard` | Gate | Create/edit gate spec with composability |
| `context_diagnose` | Context | Token usage and bloat diagnostics |
| `estimate_tokens` | Context | Current token usage and health |
| `worktree_create` | Worktree | Create isolated git worktree |
| `worktree_list` | Worktree | List active worktrees |
| `worktree_status` | Worktree | Check worktree status |
| `worktree_cleanup` | Worktree | Remove worktree and branch |
| `merge_check` | Merge | Read-only conflict detection between refs |
| `session_create` | Session | Create a work session |
| `session_get` | Session | Retrieve session details |
| `session_update` | Session | Transition session phase |
| `session_list` | Session | List sessions with filters |
| `milestone_list` | Milestone | List all milestones |
| `milestone_get` | Milestone | Get milestone details |
| `milestone_create` | Milestone | Create a milestone |
| `criteria_create` | Criteria | Create a criteria library |
| `criteria_list` | Criteria | List criteria libraries |
| `criteria_get` | Criteria | Get a criteria library |
| `pr_create` | Delivery | Create GitHub PR for milestone |
| `orchestrate_run` | Orchestrate | Launch orchestrated multi-session run |
| `orchestrate_status` | Orchestrate | Query orchestrator status |
| `poll_signals` | Signal | Poll cross-session signal inbox |
| `send_signal` | Signal | Send signal to a session |

### Signal Server

A separate HTTP endpoint (`POST /api/v1/signal`, `GET /api/v1/state`) served by axum on a configurable port (default 7432). Used for cross-job signaling in multi-session orchestration. The signal server runs alongside the MCP stdio transport within the same process.

---

## CLI Commands

Binary: `assay`

| Command | Subcommands | Purpose |
|---------|-------------|---------|
| `init` | -- | Initialize a new Assay project |
| `spec` | `list`, `show`, `review`, `validate`, `coverage` | Manage spec files |
| `gate` | `run`, `history` | Quality gate evaluation |
| `context` | `diagnose`, `list`, `prune`, `estimate` | Context window diagnostics |
| `worktree` | `create`, `list`, `status`, `cleanup` | Git worktree management |
| `run` | -- | Execute a manifest pipeline |
| `harness` | `generate`, `install`, `diff` | Agent harness configuration |
| `checkpoint` | `save`, `show`, `list` | Team state checkpointing |
| `manifest` | `generate` | Generate run manifests |
| `milestone` | `list`, `show`, `create`, `cycle` | Milestone management |
| `criteria` | `list`, `new` | Criteria library management |
| `plan` | `quick`, `full` | Guided authoring wizard |
| `history` | `analytics` | Gate run history analysis |
| `pr` | `create` | GitHub PR with gate evidence |
| `traces` | `list`, `show` | Trace file inspection |

Entry point: `#[tokio::main] async fn main()` dispatches through clap-derived `Cli` struct. Tracing is configured per-subcommand (MCP gets warn-only; pipeline commands get trace file export).

---

## Error Handling

### AssayError Hierarchy

`AssayError` is a `#[non_exhaustive]` enum with `thiserror::Error` derive, containing 30+ variants organized by concern:

| Category | Variants |
|----------|----------|
| I/O | `Io`, `Json` (both carry operation, path, source) |
| Parsing | `ConfigParse`, `SpecParse`, `GatesSpecParse`, `FeatureSpecParse`, `ManifestParse`, `LibraryParse`, `SessionParse` |
| Validation | `ConfigValidation`, `SpecValidation`, `GatesSpecValidation`, `FeatureSpecValidation`, `ManifestValidation` |
| Not-found | `SpecNotFound`, `SpecNotFoundDiagnostic` (with suggestions), `SessionNotFound`, `WorkSessionNotFound`, `GateEvalContextNotFound`, `LibraryNotFound`, `ParentGateNotFound` |
| Gate | `GateExecution`, `InvalidCriterion`, `Evaluator` |
| Worktree | `WorktreeGit`, `WorktreeGitFailed`, `WorktreeCollision`, `WorktreeExists`, `WorktreeNotFound`, `WorktreeDirty` |
| Merge | `MergeCheckRefError`, `MergeExecuteError`, `MergeRunnerError` |
| Session | `SessionError`, `SessionDirNotFound`, `SessionFileNotFound`, `WorkSessionTransition` |
| Guard | `GuardAlreadyRunning`, `GuardNotRunning`, `GuardCircuitBreakerTripped` |
| Orchestrate | `DagCycle`, `DagValidation` (feature-gated) |
| Context | `ContextBudget`, `ContextBudgetInvalid` |
| Workflow | `WorkflowViolation` |
| Init | `AlreadyInitialized` |
| Composition | `CycleDetected`, `InvalidSlug` |
| Checkpoint | `CheckpointWrite`, `CheckpointRead` |

### Related Error Types

- `EvaluatorError` -- Subprocess failures: Timeout, Crash, ParseError, NoStructuredOutput, NotInstalled, Io
- `PipelineError` -- Wraps `AssayError` with `PipelineStage` context
- `HarnessError` -- Agent adapter failures (defined in assay-types)

### Design Decisions

- **Batch validation**: All validation collects errors into `Vec<ConfigError>` / `Vec<SpecError>` / `Vec<ManifestError>` so users can fix everything in one pass.
- **Enriched not-found**: `SpecNotFoundDiagnostic` includes available specs, invalid matches, and suggestions.
- **TOML error formatting**: `format_toml_error()` shows the offending source line with a caret pointer, truncated to ~80 characters.
- **Convenience constructors**: `AssayError::io()` and `AssayError::json()` accept operation and path for consistent context.
- **Type alias**: `Result<T>` is aliased to `std::result::Result<T, AssayError>`.

---

## Configuration

### `.assay/config.toml`

Loaded by `assay_core::config::load()`. Validated with batch error collection.

| Section | Key Fields | Purpose |
|---------|-----------|---------|
| Top-level | `project_name` (required), `specs_dir` (default: `"specs/"`) | Project identity |
| `[gates]` | `default_timeout`, `working_dir`, `max_history`, `evaluator_model`, `evaluator_retries`, `evaluator_timeout`, `agent_eval_mode` | Gate execution defaults |
| `[guard]` | `soft_threshold`, `hard_threshold`, `soft_threshold_bytes`, `hard_threshold_bytes`, `poll_interval_secs`, `max_recoveries`, `recovery_window_secs` | Background daemon thresholds |
| `[sessions]` | `stale_threshold_secs`, `max_count`, `max_age_days` | Session lifecycle limits |
| `[worktree]` | `base_dir` | Git worktree base directory |
| `[provider]` | `provider` (anthropic/openai/ollama), `planning_model`, `execution_model`, `review_model` | AI model selection |
| `[workflow]` | `auto_isolate` (always/never/ask), `protected_branches`, `uat_enabled`, `strict_status` | Solo dev loop behavior |

### Spec Files

| Format | Path | Type | Purpose |
|--------|------|------|---------|
| Legacy flat | `.assay/specs/<name>.toml` | `Spec` | Backward-compatible single-file spec |
| Directory-based | `.assay/specs/<name>/gates.toml` | `GatesSpec` | Gate criteria with composition support |
| Feature spec | `.assay/specs/<name>/spec.toml` | `FeatureSpec` | IEEE 830/29148 requirements |
| Criteria library | `.assay/criteria/<name>.toml` | `CriteriaLibrary` | Reusable criterion collections |

### Run Manifests

TOML files with `[[sessions]]` array-of-tables. Loaded by `assay_core::manifest::load()`. Validated with the same batch pattern as config.

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `NO_COLOR` | Disable ANSI colors in CLI output |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Enable OTLP trace export |
| `RUST_LOG` | Standard tracing env filter |

---

## Testing Strategy

### Organization

- **Unit tests**: `#[cfg(test)] mod tests` blocks within source files covering serde roundtrips, validation rules, display formatting.
- **Integration tests**: `crates/*/tests/*.rs` files organized by domain.
- **Snapshot tests**: `insta` (v1.46 with JSON feature) for schema stability in `assay-types/tests/schema_snapshots.rs`.

### Key Test Suites

| Crate | Notable Test Files |
|-------|-------------------|
| assay-types | `schema_snapshots.rs`, `schema_roundtrip.rs`, `context_types.rs` |
| assay-core | `state_backend.rs`, `orchestrate_integration.rs`, `mesh_integration.rs`, `gossip_integration.rs`, `pipeline_streaming.rs`, `pipeline_auto_promote.rs`, `wizard.rs`, `analytics.rs`, `cycle.rs`, `milestone_io.rs`, `pr.rs` |
| assay-tui | `app_wizard.rs`, `wizard_round_trip.rs`, `gate_wizard_round_trip.rs`, `spec_browser.rs`, `slash_commands.rs`, `agent_run.rs`, `trace_viewer.rs`, `mcp_panel.rs` |
| assay-mcp | `mcp_handlers.rs`, `signal_server.rs` |
| assay-backends | `github_backend.rs`, `linear_backend.rs`, `ssh_backend.rs`, `smelt_backend.rs` |

### Common Patterns

| Pattern | Purpose |
|---------|---------|
| Serde roundtrip | Serialize to JSON/TOML, deserialize back, assert equality |
| Skip-serializing-if | Verify optional/empty fields omitted in output |
| Unknown field rejection | Verify `deny_unknown_fields` rejects extra keys |
| Backward compatibility | Old formats without new fields deserialize with defaults |
| Batch validation | Verify all errors collected in one pass |
| Temp directories | `tempfile::TempDir` for filesystem isolation |
| HTTP mocks | `mockito` for backend API tests |
| Serial execution | `serial_test` for tests with shared state |

### Test Utilities

| Crate | Purpose |
|-------|---------|
| `cupel-testing` | Context-engine test helpers |
| `tempfile` | Temporary directories |
| `mockito` | HTTP mock servers |
| `serial_test` | Sequential test execution |
| `tracing-test` | Trace output verification |
| `assert_cmd` / `predicates` | CLI integration testing |
