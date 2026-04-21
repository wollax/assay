# Assay Crates -- Exhaustive Documentation Scan

Generated: 2026-04-21

---

## Technology Stack

| Category | Technology | Version | Purpose |
|---|---|---|---|
| Language | Rust | Edition 2024 | Primary language |
| Serialization | serde | 1.x | Serialize/Deserialize for all types |
| Serialization | serde_json | 1.x | JSON format support |
| Serialization | toml | 1.x | TOML spec/config format |
| Schema | schemars | 1.x (chrono04) | JSON Schema generation for all domain types |
| Schema | jsonschema | 0.43 | Schema validation (dev) |
| Schema | inventory | 0.3 | Compile-time schema registry via `inventory::submit!` |
| CLI | clap | 4.x (derive) | CLI argument parsing with derive macros |
| TUI | ratatui | 0.30 | Terminal user interface rendering |
| TUI | crossterm | 0.28 | Terminal event handling |
| MCP | rmcp | 0.17 (server, transport-io) | Model Context Protocol server implementation |
| HTTP | axum | 0.8 | Signal endpoint HTTP server |
| HTTP | tower / tower-http | 0.5 / 0.6 | HTTP middleware (rate limiting) |
| HTTP | reqwest | 0.13 (json, blocking) | HTTP client for Linear/GitHub/Smelt backends |
| Async | tokio | 1.x (full) | Async runtime |
| Observability | tracing | 0.1 | Structured logging |
| Observability | tracing-subscriber | 0.3 (fmt, env-filter, registry) | Tracing subscriber setup |
| Observability | tracing-appender | 0.2 | Non-blocking log file writer |
| Observability | opentelemetry | 0.31 (metrics) | OpenTelemetry SDK (optional telemetry feature) |
| Observability | opentelemetry-otlp | 0.31 | OTLP exporter (optional) |
| Observability | tracing-opentelemetry | 0.32 | Tracing-to-OTel bridge (optional) |
| Context | cupel | 1.2.0 | Context-engine pipeline (token budgeting) |
| Context | cupel-otel | 0.2.0 | Cupel OpenTelemetry integration (optional) |
| Time | chrono | 0.4 (serde) | DateTime handling |
| IDs | ulid | 1.2 (serde) | ULID-based session identifiers |
| Versioning | semver | 1.x (serde) | Semantic version handling |
| Errors | thiserror | 2.x | Derive Error trait |
| Errors | color-eyre | 0.6 | Enhanced error reports (TUI) |
| Errors | anyhow | 1.x | CLI error handling |
| Filesystem | tempfile | 3.x | Atomic file writes via temp+rename |
| Filesystem | dirs | 6.x | Platform directory resolution |
| Filesystem | notify | 7.x (macos_kqueue) | File system watcher (guard daemon) |
| Filesystem | globset | 0.4 | Glob pattern matching (scope enforcement) |
| System | hostname | 0.4 | Machine hostname detection |
| System | which | 8.x | Binary PATH resolution |
| System | libc | 0.2 | Unix signal handling (guard daemon) |
| Regex | regex-lite | 0.1 | Lightweight regex (context pruning, TUI) |
| Testing | insta | 1.46 (json) | Snapshot testing |
| Testing | serial_test | 3.x | Serial test execution |
| Testing | tracing-test | 0.2 | Tracing subscriber for tests |
| Testing | cupel-testing | 0.1.0 | Cupel test utilities |
| Testing | mockito | 1.x | HTTP mock server (backends) |
| Testing | assert_cmd / predicates | 2.x / 3.x | CLI integration testing |
| UI | dialoguer | 0.12.0 | Interactive CLI prompts |

---

## Architecture Pattern

### Layered Crate Dependency Graph

```
assay-cli ──┐
assay-tui ──┤
            ├──> assay-mcp ──> assay-backends ──> assay-core ──> assay-harness ──> assay-types
            │                                  └───────────────────────────────────────┘
            └──> assay-core ──> assay-harness ──> assay-types
```

**Dependency direction:** Types flow upward from `assay-types`. Domain logic lives in `assay-core`. Adapters (`assay-harness`, `assay-backends`) implement traits. Presentation crates (`assay-cli`, `assay-tui`, `assay-mcp`) are thin wrappers.

### Key Patterns

1. **Trait-based abstraction** -- `HarnessProvider` (agent adapters), `StateBackend` (persistence), `PruneStrategy` (context pruning). All are object-safe for dynamic dispatch.

2. **Inventory-based schema registry** -- Every serializable domain type registers itself via `inventory::submit!` with a kebab-case name and a schema generator function. The `schema_registry::all_entries()` iterator yields all registered schemas at runtime.

3. **Feature-gated orchestration** -- Multi-session orchestration types and logic are behind the `orchestrate` Cargo feature flag, keeping the single-session path lean.

4. **Pipeline architecture** -- The end-to-end flow is: `RunManifest` -> spec load -> worktree create -> harness config -> agent launch -> gate evaluate -> merge check. Each stage is a `PipelineStage` enum variant for structured error context.

5. **Crash-recoverable sessions** -- `GateEvalContext` provides in-memory state for resumable gate evaluation. `WorkSession` provides persistent lifecycle tracking with a linear state machine (`SessionPhase`).

6. **Atomic file I/O** -- All writes use `NamedTempFile` -> write -> sync -> persist pattern for crash safety.

7. **Serde-driven design** -- All types derive `Serialize`/`Deserialize`/`JsonSchema`. Unknown field rejection (`deny_unknown_fields`) on authored types, tolerance on runtime/evolution types.

---

## API Surface

### assay-types

**Core domain types (all `pub`):**
- `Spec`, `Gate`, `Review`, `Workflow`, `Config`
- `Criterion`, `CriterionKind` (AgentReport, EventCount, NoToolErrors), `When` (SessionEnd, AfterToolCalls, OnEvent)
- `GateKind` (Command, AlwaysPass, FileExists, AgentReport, EventCount, NoToolErrors)
- `GateResult`, `GateRunSummary`, `GateRunRecord`, `GateEvalOutcome`, `CriterionResult`, `DiffTruncation`
- `FeatureSpec`, `SpecStatus`, `Obligation`, `Priority`, `VerificationMethod`, `Requirement`, `AcceptanceCriterion`
- `GatesSpec`, `GateSpecStatus`, `GateCriterion` (alias for `Criterion`)
- `Enforcement`, `EnforcementSummary`, `GateSection`
- `GateEvalContext`, `AgentEvaluation`, `Confidence`, `EvaluatorRole`
- `WorkSession`, `SessionPhase`, `PhaseTransition`, `AgentInvocation`, `ToolCallSummary`
- `TeamCheckpoint`, `AgentState`, `AgentStatus`, `TaskState`, `TaskStatus`, `ContextHealthSnapshot`
- `SessionEntry`, `ContentBlock`, `UsageData`, `BloatCategory`, `DiagnosticsReport`, `SessionInfo`
- `TokenEstimate`, `ContextHealth`, `GrowthRate`
- `PruneStrategy`, `PrescriptionTier`, `PruneSummary`, `PruneReport`
- `Milestone`, `MilestoneStatus`, `ChunkRef`
- `MergeCheck`, `MergeConflict`, `ConflictType`, `FileChange`, `ChangeType`
- `MergeExecuteResult`, `ConflictScan`, `ConflictMarker`, `MarkerType`
- `MergeProposal`, `MergeProposeConfig`
- `HarnessProfile`, `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent`
- `ScopeViolation`, `ScopeViolationType`
- `RunManifest`, `ManifestSession`
- `EvaluatorOutput`, `EvaluatorCriterionResult`, `CriterionOutcome`, `EvaluatorSummary`
- `FormattedEvidence`
- `ValidationResult`, `Diagnostic`, `DiagnosticSummary`, `Severity`
- `ReviewReport`, `ReviewCheck`, `ReviewCheckKind`, `FailedCriterionSummary`, `GateDiagnostic`, `CheckpointPhase`
- `CoverageReport`
- `AgentEvent` (ToolCalled, ToolResult, TurnEnded, SessionStopped, TextDelta, TextBlock)
- `SignalRequest`, `PeerUpdate`, `GateSummary`, `PeerInfo`, `PollSignalsResult`, `AssayServerState`, `RunSummary`
- `StateBackendConfig` (LocalFs, Linear, GitHub, Ssh, Smelt, Custom)
- `WorktreeConfig`, `WorktreeMetadata`, `WorktreeInfo`, `WorktreeStatus`
- `ResolvedGate`, `ResolvedCriterion`, `CriterionSource` (Own, Parent, Library)
- `CriteriaLibrary`
- `SpecPreconditions`, `PreconditionStatus`, `RequireStatus`, `CommandStatus`
- `ProviderKind`, `ProviderConfig`, `GatesConfig`, `GuardConfig`, `SessionsConfig`, `WorkflowConfig`, `AutoIsolate`
- `HarnessProvider` trait, `NullProvider`, `HarnessError`
- `GateWizardInput`, `GateWizardOutput`, `CriteriaWizardInput`, `CriteriaWizardOutput`, `CriterionInput`
- `SchemaEntry`, `schema_registry::all_entries()`

**Feature-gated (`orchestrate`):**
- `OrchestratorStatus`, `OrchestratorPhase`, `OrchestratorMode` (Dag, Mesh, Gossip)
- `SessionStatus`, `SessionRunState`, `FailurePolicy`
- `MergePlan`, `MergePlanEntry`, `MergeStrategy`, `MergeReport`, `MergeSessionResult`, `MergeSessionStatus`
- `ConflictAction`, `ConflictResolution`, `ConflictResolutionConfig`, `ConflictFileContent`
- `MeshConfig`, `MeshStatus`, `MeshMemberStatus`, `MeshMemberState`
- `GossipConfig`, `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest`

### assay-core

**Public modules and key functions:**
- `config`: `load()`, `save()`, `from_str()`, `validate()`
- `spec`: `load()`, `load_gates()`, `save_gates()`, `load_feature_spec()`, `load_spec_entry()`, `load_spec_entry_with_diagnostics()`, `scan()`, `validate()`, `validate_gates_spec()`, `validate_feature_spec()`, `from_str()`, `spec_set_status()`, `auto_promote_on_pass()`, `effective_status()`
- `spec::compose`: gate composition with `extends`/`include` resolution
- `spec::coverage`: requirement-to-criteria coverage computation
- `spec::promote`: spec status promotion
- `spec::validate`: structural validation with diagnostics
- `gate`: gate evaluation, `resolve_enforcement()`, session management, evidence rendering
- `gate::session`: gate eval context lifecycle
- `gate::evidence`: formatted evidence for PR bodies
- `gate::render`: markdown rendering of gate results
- `review`: spec review with structural and agent-quality checks
- `workflow`: solo developer workflow orchestration
- `history`: gate run history persistence and analytics
- `init`: project initialization (`assay init`)
- `checkpoint`: team state extraction, persistence, config discovery
- `context`: JSONL discovery, parsing, diagnostics, token estimation, budgeting
- `context::pruning`: strategy-based session file pruning pipeline
- `guard`: background daemon with threshold-based pruning, circuit breaker
- `worktree`: git worktree lifecycle (create, list, status, cleanup)
- `work_session`: persistent session lifecycle management
- `evaluator`: Claude Code evaluator subprocess spawning/parsing
- `merge`: conflict detection (`merge_check`), conflict scanning, merge execution
- `pipeline`: end-to-end pipeline orchestrator
- `pipeline_checkpoint`: mid-session checkpoint driver consuming agent events
- `milestone`: milestone I/O (scan, load, save, cycle management)
- `pr`: GitHub PR creation with gate evidence
- `wizard`: guided authoring (milestones, specs, criteria libraries)
- `manifest`: run manifest loading and validation
- `telemetry`: centralized tracing subscriber initialization
- `error`: `AssayError`, `EvaluatorError`, `Result<T>`

**Feature-gated (`orchestrate`):**
- `orchestrate`: DAG construction, executor, merge runner, conflict resolver, gossip, mesh
- `state_backend`: `StateBackend` trait, `CapabilitySet`, `LocalFsBackend`, `NoopBackend`
- `manifest_gen`: manifest generation from milestones or all specs

### assay-backends

**Modules (all feature-gated):**
- `factory`: `backend_from_config()` dispatcher
- `github` (`github` feature): GitHub Issues-based state backend
- `linear` (`linear` feature): Linear project-based state backend
- `ssh` (`ssh` feature): SSH remote state backend
- `smelt` (`smelt` feature): Smelt HTTP event push backend

### assay-harness

**Modules:**
- `prompt`: prompt assembly from layered sources
- `settings`: settings merging and override resolution
- `claude`: Claude Code adapter (writes `.claude/settings.json`, `CLAUDE.md`)
- `codex`: Codex adapter
- `opencode`: OpenCode adapter
- `scope`: scope enforcement and multi-agent prompt generation
- `claude_stream`: Claude streaming NDJSON parser
- `provider`: built-in `HarnessProvider` implementations

### assay-mcp -- MCP Tool Handlers

29+ MCP tools exposed:

| Tool | Purpose |
|---|---|
| `spec_list` | Discover available specs |
| `spec_get` | Read a full spec definition |
| `spec_validate` | Static spec validation |
| `spec_create` | Create a chunk spec (gates.toml) |
| `spec_resolve` | Resolve effective criteria with source annotations |
| `gate_run` | Evaluate quality gate criteria |
| `gate_evaluate` | Headless evaluator subprocess |
| `gate_report` | Submit agent evaluation for a criterion |
| `gate_finalize` | Finalize session, persist as GateRunRecord |
| `gate_history` | Query past gate run results |
| `gate_wizard` | Create/edit gate spec with composability |
| `context_diagnose` | Token usage and bloat diagnostics |
| `estimate_tokens` | Current token usage and health |
| `worktree_create` | Create isolated git worktree |
| `worktree_list` | List active worktrees |
| `worktree_status` | Check worktree status |
| `worktree_cleanup` | Remove worktree and branch |
| `merge_check` | Read-only conflict detection between refs |
| `session_create` | Create a work session |
| `session_get` | Retrieve session details |
| `session_update` | Transition session phase |
| `session_list` | List sessions with filters |
| `milestone_list` | List all milestones |
| `milestone_get` | Get milestone details |
| `milestone_create` | Create a milestone |
| `criteria_create` | Create a criteria library |
| `criteria_list` | List criteria libraries |
| `criteria_get` | Get a criteria library |
| `pr_create` | Create GitHub PR for milestone |
| `orchestrate_run` | Launch orchestrated multi-session run |
| `orchestrate_status` | Query orchestrator status |
| `poll_signals` | Poll cross-session signal inbox |
| `send_signal` | Send signal to a session |

Also: `signal_server` module provides an HTTP endpoint (`POST /api/v1/signal`, `GET /api/v1/state`) for cross-job signaling.

### assay-cli -- CLI Commands

Binary: `assay`

| Command | Subcommands | Purpose |
|---|---|---|
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

### assay-tui

Binary: `assay-tui`

**Modules:**
- `app`: Main `App` struct with state, drawing, event handling
- `agent`: Agent run panel (streaming output display)
- `event`: `TuiEvent` enum (Key, Resize, AgentEvent, AgentDone, PrStatusUpdate)
- `wizard`: Milestone/spec creation wizard
- `gate_wizard`: Gate spec creation/editing wizard
- `slash`: Slash command system
- `mcp_panel`: MCP server status panel
- `trace_viewer`: Trace file span tree viewer

---

## Domain Model

### Core Relationships

```
Config (project-level)
  ├── GatesConfig (gate execution defaults)
  ├── GuardConfig (daemon thresholds)
  ├── SessionsConfig (staleness, eviction)
  ├── WorktreeConfig (base directory)
  ├── ProviderConfig (AI model selection)
  └── WorkflowConfig (solo dev loop settings)

Milestone
  └── ChunkRef[] (slug, order, depends_on)
       └── GatesSpec (gates.toml)
            ├── GateSection (enforcement defaults)
            ├── SpecPreconditions (requires, commands)
            ├── Criterion[] (name, description, cmd/path/kind, when, enforcement)
            ├── extends → parent GatesSpec
            └── include → CriteriaLibrary[]

FeatureSpec (spec.toml, IEEE 830/29148)
  ├── Requirement[] (REQ-AREA-NNN, obligation, priority)
  └── AcceptanceCriterion[] (Gherkin, EARS, Plain)

WorkSession (persistent lifecycle)
  ├── SessionPhase: Created → AgentRunning → GateEvaluated → Completed | Abandoned
  ├── AgentInvocation (command, model)
  ├── ToolCallSummary (total, by_tool, error_count)
  └── gate_runs[] → GateRunRecord

GateEvalContext (crash-recoverable in-memory)
  ├── command_results[] → CriterionResult
  ├── agent_evaluations{} → AgentEvaluation[]
  └── diff (captured git changes)

GateRunRecord (immutable on-disk artifact)
  ├── GateRunSummary
  │    ├── CriterionResult[] → GateResult
  │    └── EnforcementSummary
  └── DiffTruncation

ResolvedGate (post-composition)
  └── ResolvedCriterion[] (criterion + CriterionSource: Own/Parent/Library)

RunManifest
  ├── ManifestSession[] (spec, settings, hooks, prompt_layers, file_scope, depends_on)
  └── OrchestratorMode (Dag, Mesh, Gossip)
```

### Type Hierarchies

- **GateKind**: Command, AlwaysPass, FileExists, AgentReport, EventCount, NoToolErrors
- **CriterionKind**: AgentReport, EventCount, NoToolErrors (internally tagged with aliases for backward compat)
- **When**: SessionEnd (default), AfterToolCalls{n}, OnEvent{event_type}
- **Enforcement**: Required (default), Advisory
- **SpecStatus** (IEEE 830): Draft -> Proposed -> Planned -> InProgress -> Verified -> Deprecated
- **GateSpecStatus** (workflow): Draft -> Ready -> Approved -> Verified
- **MilestoneStatus**: Draft -> InProgress -> Verify -> Complete
- **SessionPhase**: Created -> AgentRunning -> GateEvaluated -> Completed | Abandoned
- **AgentEvent**: ToolCalled, ToolResult, TurnEnded, SessionStopped, TextDelta, TextBlock
- **StateBackendConfig**: LocalFs, Linear, GitHub, Ssh, Smelt, Custom

---

## Configuration

### Config File: `.assay/config.toml`

Loaded by `assay_core::config::load()`. Uses `Config` struct from `assay-types`.

**Top-level fields:**
- `project_name` (required, non-empty)
- `specs_dir` (default: `"specs/"`)
- `[gates]` -- `default_timeout`, `working_dir`, `max_history`, `evaluator_model`, `evaluator_retries`, `evaluator_timeout`, `agent_eval_mode`
- `[guard]` -- `soft_threshold`, `hard_threshold`, `soft_threshold_bytes`, `hard_threshold_bytes`, `poll_interval_secs`, `max_recoveries`, `recovery_window_secs`
- `[sessions]` -- `stale_threshold_secs` (alias: `stale_threshold`), `max_count`, `max_age_days`
- `[worktree]` -- `base_dir`
- `[provider]` -- `provider` (anthropic/openai/ollama), `planning_model`, `execution_model`, `review_model`
- `[workflow]` -- `auto_isolate` (always/never/ask), `protected_branches`, `uat_enabled`, `strict_status`

### Spec Files

- **Legacy flat**: `.assay/specs/<name>.toml` -- `Spec` struct
- **Directory-based**: `.assay/specs/<name>/gates.toml` -- `GatesSpec` struct
- **Feature spec**: `.assay/specs/<name>/spec.toml` -- `FeatureSpec` struct (IEEE 830)
- **Criteria libraries**: `.assay/criteria/<name>.toml` -- `CriteriaLibrary` struct

### Run Manifests

TOML files with `[[sessions]]` array-of-tables. Loaded by `assay_core::manifest::load()`.

### Environment Variables

- `NO_COLOR` -- Disable ANSI colors in CLI output
- `OTEL_EXPORTER_OTLP_ENDPOINT` -- Enable OTLP trace export
- Standard tracing env filter via `RUST_LOG`

---

## Error Handling

### Error Types

**`AssayError`** (`assay-core/src/error.rs`) -- `#[non_exhaustive]` enum with `thiserror::Error` derive. 30+ variants organized by concern:

- **I/O**: `Io`, `Json` (both carry operation, path, and source)
- **Parsing**: `ConfigParse`, `SpecParse`, `GatesSpecParse`, `FeatureSpecParse`, `ManifestParse`, `LibraryParse`, `SessionParse`
- **Validation**: `ConfigValidation`, `SpecValidation`, `GatesSpecValidation`, `FeatureSpecValidation`, `ManifestValidation`
- **Not-found**: `SpecNotFound`, `SpecNotFoundDiagnostic` (enriched with available/invalid/suggestion), `SessionNotFound`, `WorkSessionNotFound`, `GateEvalContextNotFound`, `LibraryNotFound`, `ParentGateNotFound`
- **Gate**: `GateExecution`, `InvalidCriterion`, `Evaluator`
- **Worktree**: `WorktreeGit`, `WorktreeGitFailed`, `WorktreeCollision`, `WorktreeExists`, `WorktreeNotFound`, `WorktreeDirty`
- **Merge**: `MergeCheckRefError`, `MergeExecuteError`, `MergeRunnerError`
- **Session**: `SessionError`, `SessionDirNotFound`, `SessionFileNotFound`, `WorkSessionTransition`
- **Guard**: `GuardAlreadyRunning`, `GuardNotRunning`, `GuardCircuitBreakerTripped`
- **Orchestrate** (feature-gated): `DagCycle`, `DagValidation`
- **Context**: `ContextBudget`, `ContextBudgetInvalid`
- **Workflow**: `WorkflowViolation`
- **Init**: `AlreadyInitialized`
- **Composition**: `CycleDetected`, `InvalidSlug`
- **Checkpoint**: `CheckpointWrite`, `CheckpointRead`

**`EvaluatorError`** -- Subprocess failures: `Timeout`, `Crash`, `ParseError`, `NoStructuredOutput`, `NotInstalled`, `Io`.

**`PipelineError`** -- Wraps `AssayError` with `PipelineStage` context.

**Convenience:** `AssayError::io()` and `AssayError::json()` constructors. `Result<T>` alias for `std::result::Result<T, AssayError>`.

### Validation Pattern

All validation uses batch collection: errors accumulate in a `Vec<ConfigError>` / `Vec<SpecError>` / `Vec<ManifestError>` and are returned together so users can fix everything in one pass.

### TOML Parse Error Formatting

`format_toml_error()` shows the offending source line with a caret pointer (`^`), truncated to ~80 characters when needed.

---

## Testing Patterns

### Test Organization

- **Unit tests**: `#[cfg(test)] mod tests` blocks within source files. Extensive coverage of serde roundtrips, validation rules, and display formatting.
- **Integration tests**: `crates/*/tests/*.rs` files. Key suites:
  - `assay-core/tests/`: `state_backend.rs`, `orchestrate_integration.rs`, `mesh_integration.rs`, `gossip_integration.rs`, `pipeline_streaming.rs`, `pipeline_auto_promote.rs`, `pipeline_spans.rs`, `merge_propose.rs`, `wizard.rs`, `wizard_gate.rs`, `wizard_criteria.rs`, `analytics.rs`, `cycle.rs`, `milestone_io.rs`, `pr.rs`, `config_provider.rs`, `integration_modes.rs`
  - `assay-tui/tests/`: `app_wizard.rs`, `wizard_round_trip.rs`, `gate_wizard_round_trip.rs`, `gate_wizard_app.rs`, `spec_browser.rs`, `slash_commands.rs`, `settings.rs`, `help_status.rs`, `analytics_screen.rs`, `pr_status_panel.rs`, `agent_run.rs`, `provider_dispatch.rs`, `trace_viewer.rs`, `mcp_panel.rs`
  - `assay-mcp/tests/`: `mcp_handlers.rs`, `signal_server.rs`
  - `assay-backends/tests/`: `github_backend.rs`, `linear_backend.rs`, `ssh_backend.rs`, `smelt_backend.rs`
  - `assay-types/tests/`: `schema_snapshots.rs`, `schema_roundtrip.rs`, `context_types.rs`

### Snapshot Testing

Uses `insta` (v1.46 with JSON feature) for schema snapshot testing in `assay-types/tests/schema_snapshots.rs`.

### Test Utilities

- `cupel-testing` for context-engine test helpers
- `tempfile` for temporary directories in filesystem tests
- `mockito` for HTTP mock servers in backend tests
- `serial_test` for tests that cannot run concurrently
- `tracing-test` for verifying trace output

### Common Test Patterns

1. **Serde roundtrip**: Serialize to JSON/TOML, deserialize back, assert equality
2. **Skip-serializing-if**: Verify optional/empty fields are omitted in serialized output
3. **Unknown field rejection**: Verify `deny_unknown_fields` rejects extra keys
4. **Backward compatibility**: Old JSON/TOML without new fields deserializes with defaults
5. **Display/to_string**: Verify human-readable output matches serde format
6. **Validation batch collection**: Verify all errors collected in one pass

---

## Entry Points

### assay-cli (`crates/assay-cli/src/main.rs`)

- Binary name: `assay`
- Entry: `#[tokio::main] async fn main()` -> `run()` -> `Cli::try_parse()` -> command dispatch
- Tracing: initialized per-subcommand via `tracing_config_for()` (MCP gets warn-only, pipeline commands get trace file export)
- Exit: returns process exit code via `std::process::exit()`

### assay-tui (`crates/assay-tui/src/main.rs`)

- Entry: `fn main()` -> `color_eyre::install()` -> `init_tracing()` -> `ratatui::init()` -> `run(terminal)`
- Event loop: background thread for crossterm events + optional PR status polling thread
- Architecture: `App::new()` loads state, `App::draw()` renders, `App::handle_event()` processes input

### assay-mcp (library crate, launched via `assay mcp serve`)

- Entry: `assay_mcp::serve()` -> `server::serve()` -> `rmcp` stdio transport
- Protocol: JSON-RPC over stdin/stdout
- Signal endpoint: separate axum HTTP server on configurable port

---

## Key Abstractions

### `HarnessProvider` trait (`assay-types/src/provider.rs`)

Object-safe trait for agent adapters. Two methods:
- `write_harness(profile, working_dir) -> Result<Vec<String>>` -- writes config, returns CLI args
- `write_harness_streaming(profile, working_dir, prompt) -> Result<Vec<String>>` -- full streaming command line

Implementations: `ClaudeProvider` (assay-harness/claude), `CodexProvider` (assay-harness/codex), `OpenCodeProvider` (assay-harness/opencode), `NullProvider` (testing).

### `StateBackend` trait (`assay-core/src/state_backend.rs`)

Object-safe, `Send + Sync` trait for orchestrator state persistence. Methods:
- `capabilities() -> CapabilitySet` -- advertises supported operations
- `push_session_event()`, `read_run_state()` -- always supported
- Messaging: `deliver_message()`, `drain_inbox()` -- requires `supports_messaging`
- Gossip: `write_gossip_manifest()`, `read_gossip_manifest()` -- requires `supports_gossip_manifest`
- Checkpoints: `write_checkpoint()` -- requires `supports_checkpoints`
- Signals: `register_peer()`, `list_peers()`, `unregister_peer()` -- requires `supports_peer_registry`

Implementations: `LocalFsBackend`, `NoopBackend` (core), `GitHubBackend`, `LinearBackend`, `SshBackend`, `SmeltBackend` (assay-backends).

### `SchemaEntry` / `schema_registry` (`assay-types/src/schema_registry.rs`)

Compile-time auto-discovery registry. Types register via `inventory::submit!`. Used by the `generate-schemas` example to emit JSON Schema files for all domain types.

### `PipelineStage` enum (`assay-core/src/pipeline.rs`)

Structured error context for pipeline failures: `SpecLoad`, `WorktreeCreate`, `HarnessConfig`, `AgentLaunch`, `GateEvaluate`, `MergeCheck`.

### Context Pruning Strategies (`assay-types/src/context.rs`, `assay-core/src/context/pruning/`)

Six composable strategies: `ProgressCollapse`, `SystemReminderDedup`, `MetadataStrip`, `StaleReads`, `ThinkingBlocks`, `ToolOutputTrim`. Organized into three tiers: `Gentle`, `Standard`, `Aggressive`.

---

## Internal Data Flow

### Single-Session Pipeline (Solo Developer)

```
1. Config Loading
   .assay/config.toml → config::load() → Config

2. Spec Resolution
   RunManifest → ManifestSession.spec → spec::load_spec_entry_with_diagnostics()
   → SpecEntry (Legacy | Directory)
   → spec::compose::resolve() → ResolvedGate (if extends/include present)

3. Worktree Isolation
   worktree::create(spec_slug, base_branch) → WorktreeInfo
   → writes .assay/worktree.json metadata

4. Harness Configuration
   HarnessProvider::write_harness_streaming(profile, working_dir, prompt)
   → writes .claude/settings.json + CLAUDE.md (or equivalent for Codex/OpenCode)
   → returns Vec<String> CLI args

5. Agent Execution
   pipeline::launch_agent_streaming(cli_args)
   → spawns subprocess with --output-format stream-json
   → claude_stream parser → AgentEvent stream
   → pipeline_checkpoint consumes events, fires mid-session checkpoints

6. Gate Evaluation
   gate::evaluate_all(spec, working_dir, config)
   → for each Criterion:
     - Command: spawn shell, check exit code → GateResult
     - FileExists: check path → GateResult
     - AgentReport: evaluator subprocess or manual gate_report → AgentEvaluation
     - EventCount: count matching events → GateResult
     - NoToolErrors: check for tool errors → GateResult
   → CriterionResult[] → GateRunSummary

7. Result Persistence
   history::save(assay_dir, record) → .assay/history/<spec>/<run-id>.json
   gate::session::finalize() → GateRunRecord
   work_session::transition(GateEvaluated) → updates session JSON

8. Auto-Promotion (optional)
   spec::auto_promote_on_pass() → updates gates.toml status to Verified
   FeatureSpec.auto_promote → spec::promote::promote_spec()

9. Merge Check
   merge::merge_check(base, head) → MergeCheck
   → or merge::merge_propose() → MergeProposal (push + PR)

10. PR Creation
    pr::create_pr(milestone) → uses gh CLI → sets pr_number/pr_url on Milestone
```

### Multi-Session Orchestration

```
RunManifest (mode: dag/mesh/gossip)
  → orchestrate::dag::build_dag() → validated DAG
  → orchestrate::executor::run() → parallel session dispatch
    per session: stages 2-8 above
  → orchestrate::merge_runner::run() → ordered merge sequence
    → ordering::plan() → MergePlan (CompletionTime or FileOverlap)
    → conflict_resolver (optional AI resolution)
  → OrchestratorStatus persisted via StateBackend
```

### Context Diagnostics Flow

```
context::discovery::find_session() → session JSONL path
  → context::parser::parse_entries() → Vec<SessionEntry>
  → context::diagnostics::analyze() → DiagnosticsReport
  → context::tokens::estimate() → TokenEstimate
  → context::budgeting → cupel pipeline for token budget allocation
  → context::pruning::prune() → PruneReport (dry-run or execute)
```
