# Requirements

## Active

### R076 — LinearBackend
- Class: core-capability
- Status: validated
- Description: `assay_backends::linear::LinearBackend` implements `StateBackend`. `push_session_event` creates a Linear issue on first call (title = run_id, description = session list) and appends a comment on subsequent calls with the full `OrchestratorStatus` JSON. `read_run_state` fetches the latest comment and deserializes it back. API key from `LINEAR_API_KEY` env var. `capabilities()` returns messaging:false, gossip_manifest:false, annotations:true, checkpoints:false.
- Why it matters: Teams using Linear for project tracking get run observability without any extra tooling — session transitions appear as issue comments in the same place where work is planned.
- Source: user
- Primary owning slice: M011/S02
- Supporting slices: M011/S01
- Validation: S02 — LinearBackend implements all 7 StateBackend methods; push_session_event creates issue on first call and comments on subsequent; read_run_state deserializes latest comment; annotate_run posts [assay:manifest] tagged comment; capabilities()=D164 flags; 8 mockito contract tests pass; backend_from_config dispatches Linear→LinearBackend; just ready green with 1501 tests
- Notes: Requires `LINEAR_API_KEY`. Uses reqwest::blocking (D168 — supersedes D161 scoped async runtime). Real API validation is UAT only.

### R077 — GitHubBackend
- Class: core-capability
- Status: validated
- Description: `assay_backends::github::GitHubBackend` implements `StateBackend`. `push_session_event` shells out to `gh issue create` (first call) or `gh issue comment` (subsequent calls). `read_run_state` shells out to `gh issue view --json body`. `capabilities()` returns messaging:false, gossip_manifest:false, annotations:false, checkpoints:false.
- Why it matters: Open-source or CI-first teams using GitHub Issues get run observability integrated directly into their repo's issue tracker, with zero extra tooling beyond the `gh` CLI they already use for PRs.
- Source: user
- Primary owning slice: M011/S03
- Supporting slices: M011/S01
- Validation: S03 — `GitHubBackend` implements all 7 `StateBackend` methods; `push_session_event` creates issue on first call and comments on subsequent via `--body-file -` stdin pipe; `read_run_state` deserializes latest comment (falls back to issue body if no comments); `capabilities()` returns `CapabilitySet::none()` (all-false); 8 mock-subprocess contract tests pass; `backend_from_config()` dispatches `GitHub → GitHubBackend`; `just ready` green with 1501 tests. Real `gh` CLI validation is UAT only.
- Notes: Requires `gh` CLI installed and authenticated. Follows D008 (CLI-first), D065 (gh-first), D077 (--json for stable output) conventions. Messaging capability is false — GitHub Issues have no inbox/outbox semantics. Body passed via `--body-file -` with `Stdio::piped()` to avoid ARG_MAX and shell quoting issues.

### R078 — SshSyncBackend
- Class: core-capability
- Status: active
- Description: `assay_backends::ssh::SshSyncBackend` implements all 7 `StateBackend` methods by shelling out to `scp` to push/pull files from a remote host. `CapabilitySet::all()` returned — the remote host mirrors the local filesystem layout. Config: `host`, `remote_assay_dir`, optional `user`, optional `port`.
- Why it matters: Smelt workers running on remote machines can push state back to the controller via SSH — the same transport smelt already uses — without SCP being managed outside Assay. Encapsulates the existing smelt SCP pattern inside the trait.
- Source: user
- Primary owning slice: M011/S04
- Supporting slices: M011/S01
- Validation: unmapped
- Notes: Uses `std::process::Command::arg()` chaining (never shell string interpolation) to prevent injection. All capabilities true because the remote mirrors local filesystem semantics. Real multi-machine validation is UAT only.

### R079 — assay-backends crate and backend factory function
- Class: core-capability
- Status: validated
- Description: New `assay-backends` leaf crate with `linear`, `github`, `ssh` feature flags. `StateBackendConfig` gains `Linear`, `GitHub`, `Ssh` named variants (schema-snapshot-locked). `backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>` factory function resolves any variant to the appropriate backend. CLI/MCP construction sites use the factory fn instead of hardcoded `LocalFsBackend::new(...)` at manifest-dispatch call sites.
- Why it matters: The factory function is the bridge between the declarative `RunManifest.state_backend` config and the runtime `Arc<dyn StateBackend>` used by `OrchestratorConfig`. Without it, CLI/MCP callers must duplicate dispatch logic and import each backend directly.
- Source: inferred
- Primary owning slice: M011/S01
- Supporting slices: M011/S04
- Validation: S01 — `assay-backends` crate compiles; `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` variants with schema snapshots committed; `backend_from_config()` dispatches all 5 variants (LocalFs→LocalFsBackend, others→NoopBackend stubs); serde round-trip tests pass for all variants; `just ready` green with 1497 tests. CLI/MCP wiring deferred to S04.
- Notes: `assay-backends` depends on `assay-core` + `assay-types` (not vice versa, consistent with D003 dep-graph direction). Factory fn lives in `assay_backends::factory`. Feature flags gate each backend's deps — `reqwest` only in the binary when `linear` or `github` features are enabled. CLI/MCP construction site wiring (replacing hardcoded `LocalFsBackend::new`) is S04 work.

### R034 — OrchestratorMode selection
- Class: core-capability
- Status: validated
- Description: `RunManifest` has a top-level `mode` field (`dag` | `mesh` | `gossip`, default `dag`). The orchestration entry point dispatches to the appropriate executor based on mode. DAG mode preserves all existing behavior; Mesh and Gossip modes ignore `depends_on` with a warning.
- Why it matters: Mode selection is the user-facing contract that determines coordination pattern — it must be stable, schema-locked, and backward-compatible before Mesh/Gossip executors are built
- Source: user
- Primary owning slice: M004/S01
- Supporting slices: M004/S02, M004/S03
- Validation: S01 — OrchestratorMode enum with schema snapshot locked; mode field on RunManifest with serde(default) backward-compatible; CLI and MCP dispatch routing exercised by unit tests; all 1222+ tests pass; just ready green
- Notes: Schema snapshot updated and committed. Existing manifests without `mode` default to `dag`. Mesh/Gossip executors are stubs (full implementations in S02/S03).

### R035 — Mesh mode execution
- Class: core-capability
- Status: validated
- Description: In Mesh mode, all sessions launch in parallel (no dependency tiers). Each session receives a roster prompt layer listing peer session names and their inbox paths, enabling agents to know who else is running.
- Why it matters: Mesh semantics require awareness of peers — the roster is the minimal contract that gives agents the information needed to decide whether to message peers
- Source: user
- Primary owning slice: M004/S02
- Supporting slices: none
- Validation: S02 — test_mesh_mode_completed_not_dead proves parallel launch with roster PromptLayer injection; all sessions start without DAG ordering; depends_on emits warn and is ignored; state.json persists correct membership states; schema snapshots locked; just ready green
- Notes: Roster injected as PromptLayer (kind: System, priority: -5). Roster format includes "Outbox: <path>" as machine-parseable line (D058).

### R036 — Mesh peer messaging
- Class: core-capability
- Status: validated
- Description: Sessions in Mesh mode can write message files to their outbox directory (`.assay/orchestrator/<run_id>/mesh/<name>/outbox/`). The orchestrator polls outboxes and routes messages to target sessions' inbox directories. SWIM-inspired membership tracks alive/suspect/dead states via heartbeat files.
- Why it matters: Peer messaging is the distinctive behavior of Mesh mode — without it, Mesh is just parallel DAG without deps, which is less useful
- Source: user
- Primary owning slice: M004/S02
- Supporting slices: none
- Validation: S02 — test_mesh_mode_message_routing proves outbox→inbox routing with real filesystem ops; messages_routed counter accurate; MeshMemberState Completed vs Dead distinguishes normal exit from crash; routing thread polls every 50ms, exits when active_count==0. Note: Suspect state (heartbeat-based) is defined but unreachable until S04 adds heartbeat polling.
- Notes: Message routing is file-based (no sockets/channels). Heartbeat polling intervals and suspect/dead timeouts are configurable in MeshConfig. Suspect transitions deferred to S04.

### R037 — Gossip mode execution
- Class: core-capability
- Status: validated
- Description: In Gossip mode, all sessions launch in parallel (no dependency tiers). A coordinator thread watches for session completions and updates a knowledge manifest (`.assay/orchestrator/<run_id>/gossip/knowledge.json`) with gate results, pass/fail summary, and changed files from each completed session.
- Why it matters: Gossip enables cross-pollination of findings without explicit inter-session communication — the coordinator synthesizes on behalf of all sessions
- Source: user
- Primary owning slice: M004/S03
- Supporting slices: none
- Validation: S03 — test_gossip_mode_knowledge_manifest proves 3 mock sessions → knowledge.json with 3 entries, gossip_status.sessions_synthesized == 3; coordinator thread uses mpsc channel with drain loop to guarantee all completions captured; GossipStatus, KnowledgeEntry, KnowledgeManifest schema snapshots locked; just ready green
- Notes: Sessions run fully independently; knowledge manifest is updated post-completion by the coordinator. No mid-run injection needed — the manifest path is injected at launch as a readable path.

### R038 — Gossip knowledge manifest injection
- Class: core-capability
- Status: validated
- Description: At session launch in Gossip mode, each session's prompt layers include the knowledge manifest path. As sessions complete, the coordinator atomically updates the manifest so still-running sessions can read it at any point during their execution.
- Why it matters: The manifest injection closes the loop — agents can read what other agents have already done, enabling genuine cross-pollination of results
- Source: user
- Primary owning slice: M004/S03
- Supporting slices: none
- Validation: S03 — test_gossip_mode_manifest_path_in_prompt_layer proves each session receives a "gossip-knowledge-manifest" PromptLayer with a "Knowledge manifest: <path>" line under the run's orchestrator directory; atomic knowledge.json writes via tempfile+rename+sync_all; manifest path predictable as <run_dir>/gossip/knowledge.json
- Notes: Knowledge manifest is a JSON file with a stable schema (schema snapshot locked). Session reads it by path at any point during execution — no push mechanism needed.

### R001 — AgentSession persistence to disk
- Class: core-capability
- Status: validated
- Description: GateEvalContext (renamed from AgentSession) persists to disk via write-through cache, surviving MCP server restarts without losing active evaluation sessions
- Why it matters: In-memory sessions are lost on crash/restart, blocking reliable orchestration
- Source: inferred
- Primary owning slice: M001/S01
- Supporting slices: none
- Validation: S01 — persistence round-trip test, MCP write-through compilation, disk fallback code path
- Notes: Validated by S01. Full MCP-protocol-level restart test deferred to S07.

### R002 — Session vocabulary cleanup
- Class: quality-attribute
- Status: validated
- Description: AgentSession renamed to GateEvalContext across assay-types and assay-mcp; Smelt concepts renamed: manifest → RunManifest, runner → RunExecutor
- Why it matters: Five "session" concepts cause confusion; clean vocabulary before adding more types
- Source: inferred
- Primary owning slice: M001/S01
- Supporting slices: none
- Validation: S01 — zero grep matches for AgentSession, schema snapshot updated, all tests pass
- Notes: GateEvalContext rename complete. RunManifest/RunExecutor will be created with correct names in S06 (no rename needed).

### R003 — Harness crate exists
- Class: core-capability
- Status: validated
- Description: `assay-harness` crate exists as a leaf in the workspace dependency graph, depending on assay-core and assay-types
- Why it matters: Harness implementations depend on core, not vice versa — preserves clean dep graph
- Source: user
- Primary owning slice: M001/S02
- Supporting slices: none
- Validation: S02 — `cargo build -p assay-harness` compiles with correct dependency edges, workspace dep entry in root Cargo.toml
- Notes: Validated by S02. Crate has module stubs for prompt, settings, claude (filled by S03/S04).

### R004 — HarnessProfile type
- Class: core-capability
- Status: validated
- Description: `HarnessProfile` type in assay-types describes a complete agent configuration: prompt template, settings, and hook definitions
- Why it matters: The profile is the input contract for all harness adapters
- Source: user
- Primary owning slice: M001/S02
- Supporting slices: M001/S03
- Validation: S02 — 6 types with full derives, deny_unknown_fields, inventory registration, schema snapshots locked, re-exported from assay-types
- Notes: Validated by S02. Type contract locked by schema snapshots. S03 will use these types for prompt builder and settings merger.

### R005 — Layered prompt builder
- Class: core-capability
- Status: validated
- Description: Layered prompt builder assembles system prompts from composable layers: project conventions (always) → spec criteria (when spec provided)
- Why it matters: Agents need structured prompts that compose project context with spec requirements
- Source: user
- Primary owning slice: M001/S03
- Supporting slices: none
- Validation: S03 — `build_prompt()` implemented with 7 unit tests covering priority ordering, empty-layer filtering, stability, and mixed kinds
- Notes: Pure function, no side effects. Validated by S03.

### R006 — Layered settings merger
- Class: core-capability
- Status: validated
- Description: Layered settings merger combines project config base settings with spec-specific overrides (permissions, model, tool access)
- Why it matters: Different specs may need different agent permissions and tool access
- Source: user
- Primary owning slice: M001/S03
- Supporting slices: none
- Validation: S03 — `merge_settings()` implemented with 6 unit tests covering overlay (Option), replace (Vec), and preservation semantics
- Notes: Pure function. Replace semantics for Vec fields (D012). Validated by S03.

### R007 — Hook contract definitions
- Class: core-capability
- Status: validated
- Description: Hook contract definitions in assay-types declare lifecycle events (pre-tool, post-tool, stop) that harness adapters translate to harness-specific formats
- Why it matters: Hooks are how gates integrate with agent lifecycles
- Source: user
- Primary owning slice: M001/S03
- Supporting slices: M001/S04
- Validation: S03 — 4 tests validate HookContract/HookEvent construction and JSON round-trip for PreTool, PostTool, Stop events including realistic HarnessProfile
- Notes: Types in assay-types (validated by S03). Adapter translation to Claude Code format completed in S04.

### R008 — Claude Code adapter
- Class: core-capability
- Status: validated
- Description: Claude Code adapter generates CLAUDE.md content, .mcp.json, settings overrides, and hooks.json from a HarnessProfile
- Why it matters: Claude Code is the primary target harness — this is the first concrete adapter
- Source: user
- Primary owning slice: M001/S04
- Supporting slices: none
- Validation: S04 — generate_config() produces valid Claude Code artifacts locked by 12 insta snapshots; write_config() verified by tempfile tests; build_cli_args() verified by snapshot and unit tests. 27 total tests pass.
- Notes: Validated by S04. Runtime invocation deferred to S07.

### R009 — Callback-based control inversion
- Class: constraint
- Status: validated
- Description: Agent invocation uses callback-based control inversion (closures passed to core orchestration functions), not trait objects
- Why it matters: Preserves the zero-trait codebase convention
- Source: user
- Primary owning slice: M001/S04
- Supporting slices: M001/S06
- Validation: S04 — all three adapter functions (generate_config, write_config, build_cli_args) are plain functions, not trait methods. Zero traits in codebase confirmed.
- Notes: Validated by S04. Pattern continues in S06.

### R010 — Worktree orphan detection
- Class: quality-attribute
- Status: validated
- Description: Orphan detection identifies worktrees with no active WorkSession linked
- Why it matters: Worktrees leak disk and git refs without lifecycle tracking
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: S05 — `detect_orphans()` verified by 4 unit tests covering no-session, active-session, terminal-session, and missing-session classification
- Notes: Validated by S05. Returns Vec<WorktreeInfo> for actionable cleanup targeting.

### R011 — Worktree collision prevention
- Class: quality-attribute
- Status: validated
- Description: Collision prevention rejects worktree creation when spec already has an active worktree with an in-progress session
- Why it matters: Two worktrees for the same spec causes merge conflicts and wasted work
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: S05 — collision check in `create()` verified by 3 unit tests: active-session rejected with WorktreeCollision, terminal-session allowed, no-existing-worktree succeeds
- Notes: Validated by S05. WorktreeCollision error includes spec_slug and existing_path for actionable diagnosis.

### R012 — WorktreeMetadata session linkage
- Class: core-capability
- Status: validated
- Description: WorktreeMetadata includes `session_id: Option<String>` for session linkage
- Why it matters: Connects worktree lifecycle to session lifecycle for orphan detection
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: S05 — session_id field with serde(default, skip_serializing_if), deny_unknown_fields, schema snapshot, round-trip test with/without session_id and legacy JSON backward compatibility
- Notes: Validated by S05. Field is metadata-only (not on WorktreeInfo).

### R013 — Worktree tech debt resolution
- Class: quality-attribute
- Status: validated
- Description: 15 worktree tech debt issues resolved (error chain, base_dir type, detect_main conflation, dirty error advice, env var docs, MCP cleanup --all, deny_unknown_fields, prune failure, 3 missing tests, to_string_lossy, field duplication, schema registry, usize serialization)
- Why it matters: Tech debt compounds; cleaning up before adding harness integration prevents fragile foundations
- Source: execution
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: S05 — zero eprintln in worktree.rs, zero detect_main_worktree references, schema snapshots for WorktreeInfo/WorktreeStatus, 3 edge-case tests (corrupt metadata, git exclude, prune warning), just ready passes
- Notes: Validated by S05. Items fixed or explicitly deferred with rationale (worktree_cleanup_all → M002, field duplication → post-stabilization).

### R014 — RunManifest type
- Class: core-capability
- Status: validated
- Description: `RunManifest` type in assay-types represents a declarative description of work using `[[sessions]]` TOML array format
- Why it matters: The manifest is the entry point for the entire pipeline
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: S06 — RunManifest and ManifestSession types with full derives, deny_unknown_fields, inventory registration, schema snapshots locked, round-trip TOML tests pass
- Notes: Validated by S06. Forward-compatible for multi-agent via `[[sessions]]` array.

### R015 — Manifest parsing and validation
- Class: core-capability
- Status: validated
- Description: Single-session manifest parsing and validation from TOML files, with actionable error messages for malformed input
- Why it matters: Users author manifests by hand — errors must be helpful
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: S06 — from_str/validate/load functions with 13 tests covering round-trip, unknown fields, missing fields, empty sessions, caret-pointer errors, and file loading
- Notes: Validated by S06. ManifestParse errors include caret-pointer display; ManifestValidation collects all errors for single-pass fix.

### R016 — Manifest forward compatibility
- Class: quality-attribute
- Status: validated
- Description: RunManifest schema is forward-compatible for multi-agent extension (uses `[[sessions]]` array even for single-session)
- Why it matters: Avoids breaking change when M002 adds multi-session support
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: S06 — all test fixtures use `[[sessions]]` array syntax even for single-session; type enforces Vec<ManifestSession>
- Notes: Validated by S06. Design constraint verified by type system and test coverage.

### R017 — Single-agent end-to-end pipeline
- Class: primary-user-loop
- Status: validated
- Description: Single-agent pipeline executes the full flow: RunManifest → worktree create → agent launch (via harness) → gate evaluate → merge propose
- Why it matters: This is the core value loop — the reason assay exists
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: S07 — run_session() orchestrates 6-stage pipeline with 10 unit tests covering success and failure paths per stage; CLI and MCP entry points compile and pass tests; just ready green
- Notes: Validated by S07. Full runtime verification with real Claude Code is manual UAT.

### R018 — Pipeline as MCP tool
- Class: core-capability
- Status: validated
- Description: Pipeline is exposed as an MCP tool or composable MCP tool sequence that agents can invoke
- Why it matters: Agents need to trigger the pipeline programmatically
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: S07 — run_manifest MCP tool registered in router with manifest_path and timeout_secs params, schema generates correctly, spawn_blocking wrapping verified, missing-manifest error handling tested (5 tests)
- Notes: Validated by S07. Single run_manifest tool wraps full pipeline.

### R019 — Pipeline structured errors
- Class: failure-visibility
- Status: validated
- Description: Pipeline failures at any stage produce structured errors with the stage that failed and recovery guidance
- Why it matters: Agents need to know what failed and how to recover
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: S07 — PipelineError struct carries stage (PipelineStage enum), message, recovery guidance, and elapsed time. Tests verify stage-tagged errors for SpecLoad, WorktreeCreate, and AgentLaunch failure paths.
- Notes: Validated by S07. Recovery strings provide actionable fix guidance per stage.

## Active

### R020 — Multi-agent orchestration
- Class: core-capability
- Status: validated
- Description: OrchestratorSession, DAG executor, parallel sessions with dependency ordering
- Why it matters: Enables parallel agent work on independent specs
- Source: user
- Primary owning slice: M002/S02
- Supporting slices: M002/S01, M002/S06
- Validation: S06 — End-to-end integration tests prove 3-session DAG with dependencies executes in correct order, failure propagation skips dependents while continuing independent sessions, all successful branches merge into base. CLI routes multi-session manifests to orchestrator. MCP tool routes correctly. 3 integration tests with real git repos, 8 CLI tests, 11 MCP tests.
- Notes: Validated by S06. Real agent invocation is manual UAT.

### R021 — Orchestration MCP tools
- Class: core-capability
- Status: validated
- Description: `orchestrate_*` MCP tools (additive, no changes to existing tools)
- Why it matters: Programmatic access to multi-agent orchestration
- Source: user
- Primary owning slice: M002/S06
- Supporting slices: none
- Validation: S06 — orchestrate_run and orchestrate_status registered in router (22 total tools), schema tests pass, param deserialization tests pass, missing-manifest and missing-run-id error handling tested, orchestrate_status reads persisted state correctly. 13 total tests (11 unit + 2 integration).
- Notes: Validated by S06. Additive tools, no modification to existing 20 tools.

### R022 — Harness orchestration layer
- Class: core-capability
- Status: validated
- Description: Scope enforcement, multi-agent prompt generation
- Why it matters: Multi-agent needs coordinated prompting and scope boundaries
- Source: user
- Primary owning slice: M002/S05
- Supporting slices: M002/S06
- Validation: S05 — check_scope() detects out-of-scope and shared-file violations via globset patterns (9 tests). generate_scope_prompt() produces multi-agent awareness markdown injected as PromptLayer. CLI dispatches generate/install/update/diff for all three adapters (11 tests). ScopeViolation types with schema snapshots (2 tests). 22 total new tests. just ready passes.
- Notes: Validated by S05. Advisory enforcement only — runtime blocking deferred.

### R023 — MergeRunner with sequential merge
- Class: core-capability
- Status: validated
- Description: Sequential merge runner that merges completed session branches in topological order using `git merge --no-ff`
- Why it matters: Multiple agents produce branches that must be merged in dependency order
- Source: user
- Primary owning slice: M002/S03
- Supporting slices: M002/S06
- Validation: S03 — merge_completed_sessions() merges branches in topological order with CompletionTime/FileOverlap strategies, closure-based conflict handler, pre-flight validation, and structured MergeReport. 21 new tests (7 merge execution + 8 ordering + 6 merge runner) with real git repos. 10 schema snapshots locked. just ready passes.
- Notes: Validated by S03. S06 wires into orchestrator post-execution phase. AI conflict resolution deferred to M003 (R026).

### R024 — Additional harness adapters
- Class: differentiator
- Status: validated
- Description: Codex and OpenCode harness adapters
- Why it matters: Multi-harness support broadens adoption
- Source: user
- Primary owning slice: M002/S04
- Supporting slices: M002/S05
- Validation: S04 — Codex adapter generates .codex/config.toml (TOML) + AGENTS.md with 12 tests and 9 snapshots. OpenCode adapter generates opencode.json (JSON with $schema) + AGENTS.md with 10 tests and 9 snapshots. Both share build_prompt() for instructions, follow identical structure to Claude adapter. 49 total harness tests, 30 snapshots, just ready green.
- Notes: Validated by S04. Pulled to M002 per D028. CLI dispatch wired in S05.

### R025 — SessionCore type unification
- Class: quality-attribute
- Status: deferred
- Description: SessionCore struct composition for type unification across session concepts
- Why it matters: Reduces confusion from 5+ "session" types
- Source: inferred
- Primary owning slice: none (deferred indefinitely per D042)
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred per D042. Only 3 fields overlap across candidates. `#[serde(flatten)]` incompatible with `deny_unknown_fields`. Cost/benefit unfavorable. Revisit if a fourth session type emerges.

### R026 — AI conflict resolution
- Class: differentiator
- Status: validated
- Description: AI-powered conflict resolution via evaluator when merge conflicts arise
- Why it matters: Enables fully autonomous merge flows
- Source: user
- Primary owning slice: M003/S01
- Supporting slices: M003/S02
- Validation: S01 — two-phase merge_execute() + resolve_conflict() sync subprocess + merge runner lifecycle wiring + CLI --conflict-resolution auto flag + MCP conflict_resolution parameter. Integration tests with real git repos prove conflicting branches → live conflicted tree → scripted handler → Merged with valid merge commit (2-parent history). Real Claude invocation is manual UAT.
- Notes: Core infrastructure complete. S02 adds audit trail (R029) and validation command (R028).

### R027 — OpenTelemetry instrumentation
- Class: quality-attribute
- Status: active
- Description: OTel tracing spans and metrics across pipeline stages, session lifecycle, merge phases, and harness generation
- Why it matters: Observability is critical for debugging multi-agent orchestration at scale — which session is slow, where merges fail, harness generation latency
- Source: user
- Primary owning slice: M009/S03
- Supporting slices: M009/S01, M009/S02, M009/S04, M009/S05
- Validation: unmapped
- Notes: Deferred since M002. Now activated for M009. Scoped to tracing only (metrics deferred to R067). Covers pipeline + orchestration span instrumentation, JSON file + OTLP export, and eprintln→tracing migration.

### R028 — Post-resolution validation
- Class: quality-attribute
- Status: validated
- Description: After AI resolves a conflict, run a configurable validation command (e.g., `cargo check`) before accepting the merge commit
- Why it matters: Without validation, AI resolution is a trust-me black box that can introduce subtle semantic errors
- Source: M003 research
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: S02 — run_validation_command() with rollback proven by unit tests (success/failure/not_found); validation_command: "sh -c 'exit 1'" causes Skip + empty resolutions in integration test; git reset --hard HEAD~1 on failure; just ready passes
- Notes: Validation command is optional, configurable in ConflictResolutionConfig. Non-zero exit rejects the resolution and aborts the merge.

### R029 — Conflict resolution audit trail
- Class: failure-visibility
- Status: validated
- Description: Record original conflict markers, resolved content, and resolver output in MergeReport for every resolved conflict
- Why it matters: Critical for debugging when AI resolutions introduce subtle bugs — without an audit trail, the resolution is opaque
- Source: M003 research
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: S02 — MergeReport.resolutions[0] populated with session_name, original_contents (with markers), resolved_contents (clean), resolver_stdout in test_merge_resolutions_audit_trail integration test; persisted to merge_report.json; surfaced via orchestrate_status MCP tool returning { status, merge_report } wrapper; just ready passes
- Notes: Recorded as Vec<ConflictResolution> on MergeReport. Viewable via CLI --json and orchestrate_status MCP tool.

### R039 — Milestone concept
- Class: core-capability
- Status: validated
- Description: Milestones are first-class project organization units stored as TOML files in `.assay/milestones/`. A milestone has a name, description, ordered list of chunk references (spec slugs), status (draft/in_progress/verify/complete), optional depends_on other milestones, and optional PR settings (base_branch, branch_prefix).
- Why it matters: Without a milestone layer, Assay is a gate runner — with it, Assay becomes a development cycle manager that tracks progress across related chunks of work
- Source: user
- Primary owning slice: M005/S01
- Supporting slices: M005/S02, M005/S03, M005/S04
- Validation: S01 — Milestone, ChunkRef, MilestoneStatus types in assay-types with TOML round-trip tests and schema snapshots locked; milestone_list and milestone_get MCP tools registered; assay milestone list CLI subcommand functional; just ready green
- Notes: Milestone files live in `.assay/milestones/<slug>.toml`. Chunk references are spec slugs that must exist in `.assay/specs/`.

### R040 — Chunk-as-spec
- Class: core-capability
- Status: validated
- Description: Specs gain backward-compatible `milestone` and `order` metadata fields. A chunk IS a spec — it has gates.toml criteria, can be run independently via `gate run`, and also belongs to a milestone with an explicit ordering. Existing specs without these fields continue to work unchanged.
- Why it matters: Reusing the existing spec format avoids a parallel system — milestone membership is a metadata overlay, not a separate entity type
- Source: user
- Primary owning slice: M005/S01
- Supporting slices: M005/S02
- Validation: S01 — GatesSpec extended with serde(default, skip_serializing_if) fields; gates_spec_rejects_unknown_fields still passes; 3 new backward-compat tests pass; 1293 workspace tests green
- Notes: Fields added to GatesSpec: `milestone: Option<String>` and `order: Option<u32>`. Fully backward-compatible.

### R041 — Milestone file I/O
- Class: core-capability
- Status: validated
- Description: `assay-core` provides `milestone_load()`, `milestone_save()`, and `milestone_scan()` for TOML-based milestone persistence under `.assay/milestones/`. Atomic writes (tempfile-rename), validation on load, clear errors on malformed files.
- Why it matters: Reliable milestone persistence is the foundation for all cycle state tracking, wizard output, and PR workflow
- Source: inferred
- Primary owning slice: M005/S01
- Supporting slices: M005/S02, M005/S03, M005/S04
- Validation: S01 — milestone_load, milestone_save, milestone_scan implemented with atomic NamedTempFile+sync_all+persist; 5 integration tests in crates/assay-core/tests/milestone_io.rs all pass; AssayError::Io carries path and operation label on every failure
- Notes: Same atomic write pattern established in history.rs and work_session.rs.

### R042 — Guided authoring wizard
- Class: primary-user-loop
- Status: validated
- Description: `assay plan` is an interactive CLI wizard that asks structured questions (goal description, success criteria per chunk, verification commands) and generates a complete milestone TOML + chunk specs (gates.toml + optional spec.toml) from the answers. Also available as a `milestone_create` / `spec_create` MCP tool pair for agent-driven authoring.
- Why it matters: A beginning developer cannot write gate criteria from scratch — the wizard is the primary entry point that makes Assay accessible without prior knowledge of the spec format
- Source: user
- Primary owning slice: M005/S03
- Supporting slices: M005/S01
- Validation: S03 — create_from_inputs integration tests prove atomic milestone TOML + per-chunk gates.toml creation, milestone/order metadata on specs, slug collision rejection, and spec-patches-milestone behavior; MCP milestone_create and spec_create tool tests prove programmatic authoring; assay plan non-TTY guard proven by unit test; interactive TTY path is UAT-only (see S03-UAT.md); all 1320+ workspace tests green; just ready green
- Notes: Wizard asks: milestone goal, chunk breakdown (1-7 chunks), success criteria per chunk (as descriptions; cmd fields require manual editing). Generates valid milestone TOML + gates.toml per chunk. Criteria are text-only in the current implementation — runnable commands not collected by the wizard (known limitation D076).

### R043 — Development cycle state machine
- Class: core-capability
- Status: validated
- Description: Milestones track development phases: draft → in_progress → verify → complete. Transitions are guarded: in_progress requires at least one chunk; verify requires all chunks' required gates to pass; complete requires the milestone to have been in verify state. Invalid transitions return structured errors.
- Why it matters: The state machine is what turns Assay into a workflow engine — without it, milestones are just labeled buckets of specs
- Source: user
- Primary owning slice: M005/S02
- Supporting slices: M005/S01
- Validation: S02 — milestone_phase_transition enforces guarded transitions; cycle_advance evaluates gates before marking a chunk complete and auto-transitions to Verify when last chunk done; test_milestone_phase_transition_valid + test_milestone_phase_transition_invalid prove all guard conditions; all 1308 workspace tests green; just ready green
- Notes: State persisted in milestone TOML file. Transitions driven by cycle_advance MCP tool and assay milestone advance CLI command. Invalid transitions return AssayError::Io with descriptive from/to message.

### R044 — Cycle MCP tools
- Class: core-capability
- Status: validated
- Description: New MCP tools: `milestone_list` (list all milestones with status/progress), `milestone_get` (full milestone details including chunk statuses), `cycle_status` (current active milestone + active chunk + phase), `cycle_advance` (mark current chunk gates-verified, activate next chunk or advance milestone phase), `chunk_status` (gate pass/fail summary for a specific chunk).
- Why it matters: Agent-driven workflows require MCP tools to query and advance the development cycle — without them the agent has no way to know where it is or what comes next
- Source: user
- Primary owning slice: M005/S02
- Supporting slices: M005/S01
- Validation: S02 — cycle_status, cycle_advance, chunk_status registered in MCP router (3 presence tests pass); cycle_status returns null/CycleStatus JSON; cycle_advance wraps spawn_blocking and returns updated CycleStatus or domain_error; chunk_status reads history without triggering gate evaluation; milestone_list and milestone_get validated in S01; all tools additive
- Notes: cycle_advance rejects advancement when required gates fail, returning AssayError::Io with required_failed count and chunk slug. chunk_status returns { has_history: false } gracefully for new chunks.

### R045 — Gate-gated PR creation
- Class: primary-user-loop
- Status: validated
- Description: `assay pr create <milestone>` checks that all required gates in all milestone chunks pass, then creates a GitHub PR via `gh` CLI. PR is opened only when the gate check succeeds. PR number and URL are stored in the milestone file for tracking. Also available as a `pr_create` MCP tool.
- Why it matters: The PR is the delivery artifact — gate-gating it ensures only verified work ships, closing the quality loop between spec → implementation → PR
- Source: user
- Primary owning slice: M005/S04
- Supporting slices: M005/S01, M005/S02
- Validation: S04 — pr_check_milestone_gates + pr_create_if_gates_pass proven by 8 integration tests with mock gh binary; CLI proven by 2 unit tests; MCP pr_create tool proven by presence test; milestone TOML mutation confirmed; Verify→Complete transition confirmed; just ready green (1331 tests)
- Notes: Shells out to `gh pr create` (consistent with D008 git-CLI-first). Returns structured error when gates fail, listing which chunks have failing criteria.

### R046 — Branch-per-chunk naming
- Class: convention
- Status: validated
- Description: Worktree branches for chunk work follow the naming convention `assay/<milestone-slug>/<chunk-slug>`. The existing worktree system (D008) creates these branches. The PR command opens the PR from this branch to the configured base branch (default: main).
- Why it matters: Consistent branch naming makes the development history readable and enables the PR workflow to locate the correct branch without ambiguity
- Source: inferred
- Primary owning slice: M005/S04
- Supporting slices: M005/S01
- Validation: S04 — pr_create_if_gates_pass uses milestone.pr_base (default "main") as PR base; branch naming convention respected; no regression introduced
- Notes: Extends the existing `assay/<spec>` worktree branch convention (already used in M001-M004).

### R047 — Claude Code plugin upgrade
- Class: differentiator
- Status: validated
- Description: The Claude Code plugin gains new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`) and updated CLAUDE.md that describes the full guided workflow cycle. New hooks: Stop hook checks cycle status and reports incomplete chunks; PostToolUse reminder names active chunk. Plugin version bumped to 0.5.0.
- Why it matters: The plugin is the integration surface for Claude Code users — without upgraded skills the guided workflow is invisible inside Claude Code
- Source: user
- Primary owning slice: M005/S05
- Supporting slices: M005/S01, M005/S02, M005/S03, M005/S04
- Validation: S05 — 3 skill files with YAML frontmatter; CLAUDE.md ≤50 lines with skill/MCP tables; cycle-stop-check.sh passes bash -n with ≥11 exit-0 guards; hooks.json wired to cycle-stop-check.sh (stop-gate-check.sh removed); plugin.json 0.5.0; just ready green (1331+ tests). Decisions D084–D087 capture key patterns.
- Notes: PreCompact hook (milestone-checkpoint.sh) not implemented — Stop hook and PostToolUse provide sufficient cycle awareness. Interview-first pattern (D084) prevents orphan milestones. BLOCKING_CHUNKS in Stop hook reason (D087) enables immediate chunk targeting.

### R048 — Codex plugin (basic)
- Class: differentiator
- Status: validated
- Description: The Codex plugin gains a complete AGENTS.md workflow guide and four skills: gate-check, spec-show, cycle-status, and plan. These give Codex users the same spec-driven workflow experience as Claude Code users.
- Why it matters: Plugin parity at launch — both major agent platforms get a working plugin in M005
- Source: user
- Primary owning slice: M005/S06
- Supporting slices: M005/S01, M005/S02
- Validation: S06 — AGENTS.md (34 lines, ≤60 cap); 5 skill files (gate-check, spec-show, cycle-status, next-chunk, plan); all tool names correct; active:false handling confirmed in cycle-status and next-chunk; interview-first ordering confirmed in plan; cmd-editing note present; .gitkeep removed; 18/18 structural checks pass
- Notes: Delivered 5 skills (not 4 as originally planned — next-chunk was in must-haves and completed alongside plan). Flat .md file convention, not subdirectory SKILL.md.

### R049 — TUI project dashboard
- Class: primary-user-loop
- Status: validated
- Description: A real Ratatui TUI application (replacing the current stub) with a project dashboard showing: list of milestones with status indicators, chunk progress per milestone (X/N complete), gate status summary per chunk (pass/fail/pending), and keyboard navigation.
- Why it matters: The TUI is the preferred primary interface — a real dashboard is what makes Assay feel like a first-class development tool rather than a CLI
- Source: user
- Primary owning slice: M006/S01
- Supporting slices: none
- Validation: M006 complete — assay-tui binary produced (no collision with assay-cli); App::new() with Screen::NoProject guard (unit test); draw_dashboard() renders milestone name + status badge + chunk fraction from milestone_scan; handle_event() Up/Down/q/Enter tested; 6 spec_browser integration tests prove navigation; just ready green (1367 tests)
- Notes: Reads milestone and gate data from `.assay/` via assay-core. No agent spawning in M006.

### R050 — TUI interactive wizard
- Class: primary-user-loop
- Status: validated
- Description: The guided authoring wizard (R042) runs inside the TUI as an interactive form, allowing spec creation without dropping to the CLI.
- Why it matters: The TUI should be self-sufficient — requiring CLI for wizard defeats the purpose of a primary-surface TUI
- Source: user
- Primary owning slice: M006/S02
- Supporting slices: M006/S01
- Validation: M006 complete — WizardState state machine + draw_wizard popup (centered 64×14 popup, hardware cursor); App wiring (n→wizard→Dashboard); wizard_round_trip integration test drives synthetic KeyEvents through N=2 chunk flow → create_from_inputs → asserts milestone TOML + two gates.toml files in tempdir; App tests prove n-key opens wizard, Esc cancels, error on slug collision stays in wizard; 27 assay-tui tests + 1367 workspace tests all pass
- Notes: Pure WizardState in assay-tui (TUI concern); create_from_inputs from assay-core does the file I/O. Criteria are text-only (no cmd field, per D076).

### R051 — TUI spec browser
- Class: primary-user-loop
- Status: validated
- Description: The TUI allows navigation into any spec, displaying criteria, their descriptions, and the latest gate run result (pass/fail/pending) with evidence on demand.
- Why it matters: Developers need to inspect what's being verified — the spec browser gives visibility into the gate criteria without reading raw TOML
- Source: user
- Primary owning slice: M006/S03
- Supporting slices: M006/S01
- Validation: S03 — Dashboard→MilestoneDetail→ChunkDetail navigation with Esc chains; criteria table with ✓/✗/? icons joined from latest gate run via join_results; empty history renders as all Pending (not error); Legacy spec shows fallback message; 6 spec_browser integration tests all pass; just ready green
- Notes: Evidence drill-down (raw gate output per criterion) deferred to M007. Live refresh deferred to M007.

### R052 — TUI provider configuration
- Class: operability
- Status: validated
- Description: The TUI has a configuration screen for AI provider selection (Anthropic, OpenAI, Ollama) and model selection per phase (planning, execution, review). Settings persist to `.assay/config.toml`.
- Why it matters: Different providers and models have different cost/quality tradeoffs — users need to configure without editing TOML files
- Source: user
- Primary owning slice: M006/S04
- Supporting slices: M006/S05
- Validation: M006 complete — ProviderKind+ProviderConfig in assay-types with serde(default, skip_serializing_if) per D092; config_save (save fn) in assay-core::config using NamedTempFile atomic write; Screen::Settings full-screen view with ↑↓ provider selection and w save; 5 settings integration tests pass (open/close/navigate/save/restart-persistence); config_toml_roundtrip_without_provider proves backward compat; schema snapshots locked; status bar shows project name from Config; just ready green (1367 tests)
- Notes: Config.provider field uses Option<ProviderConfig> with serde(default) so existing config.toml files without provider section load without error (D092). Model-per-phase fields exist in ProviderConfig but UI shows only provider selection; model editing requires manual config.toml editing.

### R053 — TUI agent spawning
- Class: core-capability
- Status: validated
- Description: The TUI can spawn, monitor, and display output from AI agent sessions (assay pipeline runs) for the active chunk. Shows live status, gate results as they come in, and surfaces failures inline.
- Why it matters: The TUI becomes the primary development loop — spawning agents from the dashboard closes the cycle without leaving the TUI
- Source: user
- Primary owning slice: M007/S01
- Supporting slices: M006/S01
- Validation: S01 — channel-based event loop (TuiEvent dispatch), launch_agent_streaming (real subprocess pipes, line-by-line delivery), Screen::AgentRun with AgentRunStatus (Running/Done/Failed), r key handler spawning agent and transitioning to AgentRun, gate results refreshed on AgentDone; 6 integration tests (3 pipeline_streaming + 3 agent_run) prove mechanical correctness; all 30 assay-tui tests pass; just ready green. Real Claude invocation is UAT-only.
- Notes: Agent output streamed verbatim via mpsc channel. Gate results refresh synchronously in handle_agent_done via milestone_scan. Provider dispatch (Ollama/OpenAI) deferred to S02.

### R054 — Provider abstraction
- Class: core-capability
- Status: validated
- Description: Assay abstracts the AI provider layer so users can configure Anthropic (Claude), OpenAI (GPT), or Ollama (local) as the agent backend. Provider selection affects harness adapter choice and model configuration.
- Why it matters: Lock-in to a single provider limits adoption — provider abstraction enables beginners to use whichever AI tool they have access to
- Source: user
- Primary owning slice: M007/S02
- Supporting slices: M007/S01
- Validation: S02 — `provider_harness_writer` dispatches Anthropic/Ollama/OpenAI; 3 unit tests prove correct CLI args per provider; Settings screen model fields persist; real invocation is UAT-only
- Notes: Anthropic uses existing Claude Code adapter. Ollama uses `ollama run <model>`. OpenAI uses `openai api chat.completions.create --model <model>`. All three dispatched via closures (D001/D109).

### R055 — TUI MCP server management
- Class: operability
- Status: validated
- Description: The TUI has a panel for managing MCP server configuration. Shows configured servers from `.assay/mcp.json`, allows adding and deleting servers, and persists changes atomically to disk.
- Why it matters: Power users need to extend Assay's capabilities via MCP without editing JSON config files
- Source: user
- Primary owning slice: M007/S04
- Supporting slices: none
- Validation: S04 — Screen::McpPanel with add/delete/save keyboard UX; mcp_config_load/mcp_config_save with atomic NamedTempFile writes; 4 integration tests (load-empty, load-from-file, add-writes, delete-writes); name uniqueness validation; just ready green
- Notes: Static config management only (D110) — live MCP server connection/tool inspection deferred to M008+. Server list configured in `.assay/mcp.json`.

### R056 — TUI slash commands
- Class: primary-user-loop
- Status: validated
- Description: The TUI has a command input (triggered by `/`) that accepts slash commands: `/gate-check`, `/spec-show`, `/status`, `/next-chunk`, `/pr-create`. Commands execute against the active milestone/chunk context.
- Why it matters: Power users expect keyboard-driven control — slash commands provide a fast path for common operations without mouse navigation
- Source: user
- Primary owning slice: M007/S03
- Supporting slices: M007/S01
- Validation: S03 — 6 integration tests prove parse, dispatch, tab completion, overlay open/close, and command execution; `/` key opens from all non-wizard screens; synchronous dispatch to assay-core (D111); just ready green
- Notes: Same commands as the plugin skills, but executed locally in the TUI context. `/plan` not implemented (wizard is the TUI planning surface).

### R057 — OpenCode plugin
- Class: differentiator
- Status: validated
- Description: Full OpenCode plugin matching Claude Code plugin: AGENTS.md, skills (gate-check, spec-show, cycle-status, plan), and opencode.json configuration. Completes the three-platform plugin parity.
- Why it matters: OpenCode is the third major agent platform — parity across all three platforms maximizes reach
- Source: user
- Primary owning slice: M008/S03
- Supporting slices: none
- Validation: S03 — AGENTS.md (37 lines, ≤60 cap); 5 skill files (gate-check, spec-show, cycle-status, next-chunk, plan) copied verbatim from Codex plugin; all 10 MCP tool names verified present; flat .md file convention; interview-first pattern confirmed; both null guards confirmed; .gitkeep removed; opencode.json untouched; 22/22 structural checks pass
- Notes: OpenCode plugin scaffold already existed in `plugins/opencode/` (package.json, opencode.json, tsconfig.json). S03 filled in AGENTS.md + 5 skills. Content is identical to Codex plugin except the AGENTS.md title heading.

### R058 — Advanced PR workflow
- Class: primary-user-loop
- Status: validated
- Description: PR creation supports configurable labels, reviewers, and PR body templates. The TUI shows PR status (open/merged/closed, CI status, review status) for milestones with open PRs.
- Why it matters: The basic PR creation (R045) gets work out the door — labels, reviewers, and templates make it usable in team workflows
- Source: user
- Primary owning slice: M008/S02
- Supporting slices: M008/S01
- Validation: S01 — Milestone TOML extended with pr_labels/pr_reviewers/pr_body_template (backward-compatible); pr_create_if_gates_pass passes --label/--reviewer/--body to gh; CLI --label/--reviewer flags and MCP labels/reviewers params with extend semantics; 12 integration tests with mock gh binary; template rendering with 4 placeholders proven. S02 — PrStatusInfo type + pr_status_poll() calling gh pr view --json; TuiEvent::PrStatusUpdate + background polling thread (60s interval, initial-poll-no-delay); dashboard badge rendering (state icon + CI counts + review abbreviation); graceful degradation when gh missing; 8 core + 3 TUI integration tests.
- Notes: Extends the milestone TOML pr_settings field. PR status from `gh pr view --json`. Fully validated across S01+S02.

### R059 — Gate history analytics
- Class: failure-visibility
- Status: validated
- Description: Assay tracks and surfaces gate failure trends across runs — which criteria fail most often, which chunks require the most retries, milestone completion velocity. Accessible from TUI analytics panel and `assay history --analytics` CLI command.
- Why it matters: Identifying recurring failures helps developers improve their specs and find systemic quality issues before they reach PR
- Source: user
- Primary owning slice: M008/S04
- Supporting slices: M008/S05
- Validation: S04 — `compute_analytics()` aggregates failure frequency and milestone velocity; CLI with text tables and `--json`; 14 tests. S05 — TUI analytics screen with `a` key handler, draw_analytics renderer (failure frequency heatmap + velocity table), 6 integration tests; `just ready` green.
- Notes: Aggregates from existing `.assay/history/` records. No new storage format needed. Both CLI and TUI surfaces validated.

### R060 — Structured tracing foundation
- Class: quality-attribute
- Status: validated
- Description: Replace `eprintln!` across the workspace with `tracing::warn/info/debug`. Set up `tracing-subscriber` with layered configuration (fmt layer + optional OTel layer). All crates emit structured events via the `tracing` facade.
- Why it matters: `eprintln!` output is unstructured, unleveled, and invisible to any collection system. Structured tracing is the foundation for all observability features.
- Source: user
- Primary owning slice: M009/S01
- Supporting slices: none
- Validation: S01 — zero eprintln! in all 4 production crates (grep verified); init_tracing() with TracingConfig presets, EnvFilter, non-blocking stderr; 3 telemetry unit tests; cargo fmt/clippy/test all green; just ready passes
- Notes: D125 superseded by D131 (assay-tui gains tracing dep). 3 interactive eprint! calls preserved (D133). Guard daemon file logging deferred to S04.

### R061 — Pipeline span instrumentation
- Class: core-capability
- Status: validated
- Description: `#[instrument]` spans on pipeline stages: spec load, worktree create, agent launch, gate eval, merge propose. Each span carries stage name, spec slug, and timing.
- Why it matters: The pipeline is the core value loop — when a gate eval is slow or an agent launch fails, the user needs to see exactly where time was spent and what failed.
- Source: user
- Primary owning slice: M009/S02
- Supporting slices: M009/S01
- Validation: S02 — `#[instrument]` on 5 public pipeline functions; `info_span!` on 6 stage blocks; span names verified by 4 integration tests using tracing-test subscriber capture; info/warn events at all stage boundaries; all existing pipeline tests green
- Notes: Spans nest under `pipeline::run_session` and `pipeline::setup_session` root spans. Consistent with D007 (sync core). D135 (tracing-test), D136 (no-env-filter for cross-crate assertions).

### R062 — Orchestration span instrumentation
- Class: core-capability
- Status: validated
- Description: Spans on DAG/Mesh/Gossip executors: per-session spans nested under orchestration root span. Merge runner phases, conflict resolution, and session state transitions all traced.
- Why it matters: Multi-agent orchestration is where things go wrong invisibly — which session is blocking the DAG, which merge conflicted, which gossip coordinator stalled.
- Source: user
- Primary owning slice: M009/S03
- Supporting slices: M009/S01, M009/S02
- Validation: S03 — 5 integration tests in orchestrate_spans.rs asserting span names for DAG root+session, Mesh root, Gossip root, merge root via tracing-test subscriber capture. Cross-thread span parenting in std::thread::scope workers via Span::current() capture+clone+enter pattern. All existing orchestration tests pass with zero regressions.
- Notes: Cross-thread span parenting uses enter-guard pattern (D138). Span assertions use `{` suffix to prevent module-path false positives (D137). info!() events inside spans for tracing-test detectability (D139).

### R063 — JSON file trace export
- Class: core-capability
- Status: validated
- Description: Completed traces written to `.assay/traces/` as JSON files. CLI command `assay traces list` and `assay traces show <id>` for inspection without external tooling.
- Why it matters: Not every user has Jaeger/Grafana. JSON file export provides zero-dependency trace inspection for local development and debugging.
- Source: user
- Primary owning slice: M009/S04
- Supporting slices: M009/S01
- Validation: S04 — JsonFileLayer writes one JSON file per root span with correct parent-child tree, timing, and fields; atomic writes via NamedTempFile+persist; file pruning at 50 max; assay traces list prints table of traces; assay traces show <id> renders indented span tree; end-to-end write→read→render round-trip proven by integration test; just ready green
- Notes: Custom Layer implementation (not built-in JSON formatter) to capture span open/close lifecycle. Trace files written only for pipeline-running subcommands (Run/Gate/Context); Traces subcommand uses traces_dir: None to prevent self-tracing.

### R064 — OTLP trace export
- Class: core-capability
- Status: validated
- Description: Feature-flagged OTLP exporter (`--features telemetry`) sends spans to a collector (Jaeger/Tempo). Uses rt-tokio with existing runtime (D143). Configurable endpoint via `OTEL_EXPORTER_OTLP_ENDPOINT` env var.
- Why it matters: OTLP is the industry standard for distributed tracing. Teams using Jaeger/Grafana Tempo need native export without a sidecar.
- Source: user
- Primary owning slice: M009/S05
- Supporting slices: M009/S01
- Validation: S05 — telemetry feature flag on assay-core/assay-cli gates all OTel deps; init_tracing() adds OTLP layer when feature enabled and endpoint configured; graceful degradation with tracing::warn! on init failure; TracingGuard::drop() flushes via SdkTracerProvider::shutdown(); cargo tree confirms zero OTel deps in default build, 13 in telemetry build; 2 integration tests pass; just ready green. Real Jaeger validation is UAT.
- Notes: Uses http-proto + hyper-client transport (D144) to avoid reqwest version conflict. D127 superseded by D143 (no scoped runtime needed).

### R065 — Trace context propagation
- Class: quality-attribute
- Status: validated
- Description: Trace IDs propagated across subprocess boundaries (agent launch) via TRACEPARENT environment variable so child process spans can be correlated with parent orchestration traces.
- Why it matters: Without propagation, agent subprocess traces are disconnected islands. Propagation enables the full picture: orchestrator → session → agent → gate eval as one trace.
- Source: inferred
- Primary owning slice: M009/S05
- Supporting slices: M009/S02, M009/S03
- Validation: S05 — cfg-gated extract_traceparent() and inject_traceparent() helpers in pipeline.rs; both launch_agent() and launch_agent_streaming() inject TRACEPARENT when telemetry feature enabled and active span exists; integration test proves TRACEPARENT appears in subprocess env with valid W3C format 00-{32hex}-{16hex}-{2hex}; debug log when no active span; just ready green
- Notes: Standard W3C Trace Context via `TRACEPARENT` env var. Child processes that support OTel will pick it up automatically. Non-OTel children ignore it harmlessly. Thread-local span context captured before thread::spawn in streaming path.

### R071 — StateBackend trait and CapabilitySet
- Class: core-capability
- Status: validated
- Description: `assay_core::state_backend::StateBackend` trait with sync methods: `capabilities()`, `push_session_event()`, `read_run_state()`, `send_message()`, `poll_inbox()`, `annotate_run()`, `save_checkpoint_summary()`. `CapabilitySet` flags struct declares which optional methods a backend implements. `StateBackendConfig` enum in `assay-types` with `LocalFs` and `Custom` variants.
- Why it matters: The trait is the abstraction boundary — without it, every concrete backend (Linear, GitHub, SSH) has no stable interface to implement against
- Source: user
- Primary owning slice: M010/S01
- Supporting slices: M010/S02
- Validation: S01 — trait defined with 7 sync methods, object safety proven via `_assert_object_safe` compile guard, `CapabilitySet::all()`/`none()` constructors verified by 2 contract tests, `Box<dyn StateBackend>` construction proven by trait-object test, `StateBackendConfig` serde schema locked by JSON Schema snapshot, 1471 workspace tests pass, just ready green
- Notes: Deliberate, scoped exception to D001 (zero-trait convention). Documented as D149.

### R072 — LocalFsBackend: zero regression
- Class: quality-attribute
- Status: validated
- Description: `LocalFsBackend` implements `StateBackend` by wrapping all existing persistence code. All orchestrate integration, mesh, and gossip tests pass unchanged. `RunManifest` without `state_backend` field defaults to `LocalFsBackend` (backward-compatible deserialization).
- Why it matters: The abstraction must be invisible to existing users — no behavioral change, no test regression, no schema break
- Source: user
- Primary owning slice: M010/S02
- Supporting slices: none
- Validation: S02 — backward-compat round-trip tests (manifest without field → None, manifest with LocalFs → round-trip), 16/16 LocalFsBackend contract tests, 5+2+2+5+3=17 integration tests all pass unchanged, just ready green with 1481 tests. Schema split (orchestrate/non-orchestrate) covers both feature flag states.
- Notes: `RunManifest` confirmed no `deny_unknown_fields` — D092 pattern applied cleanly. Schema snapshot split (D159) handles feature-gated field without conflicts.

### R073 — Tier-2 event routing through backend
- Class: core-capability
- Status: validated
- Description: Orchestrator, mesh coordinator, gossip coordinator, and checkpoint persistence route Tier-2 events (session transitions, run phase, knowledge manifest notifications, checkpoint summaries) through `StateBackend` methods. Tier-1 (per-tick heartbeats, per-message mesh routing) stays file-backed inside `LocalFsBackend` — not exposed as a trait surface.
- Why it matters: This is the actual payoff — smelt workers can push Tier-2 events to a remote backend without SCP; the controller reads from the backend instead of the filesystem
- Source: user
- Primary owning slice: M010/S02
- Supporting slices: M010/S03
- Validation: S02 — zero `persist_state` references in `crates/assay-core/src/orchestrate/` (grep confirmed), all 11 callsites replaced by `config.backend.push_session_event()`, LocalFsBackend retains filesystem behavior, all integration tests pass.
- Notes: Tier-1 vs Tier-2 split confirmed: heartbeats and per-message file routing are LocalFsBackend implementation details only.

### R074 — CapabilitySet and graceful degradation
- Class: core-capability
- Status: validated
- Description: Orchestrator checks `backend.capabilities().supports_messaging` before mesh peer routing and `supports_gossip_manifest` before knowledge manifest PromptLayer injection. If a capability is absent, the orchestrator emits a `warn!` event and continues without that feature rather than failing.
- Why it matters: Not all backends support all operations. A LinearBackend that supports messaging but not sub-second heartbeats must degrade gracefully, not panic.
- Source: user
- Primary owning slice: M010/S03
- Supporting slices: M010/S02
- Validation: S03 — run_mesh() checks supports_messaging before spawning routing thread; run_gossip() checks supports_gossip_manifest before PromptLayer injection and manifest writes; NoopBackend test helper (CapabilitySet::none()) proves degradation paths; test_mesh_degrades_gracefully_without_messaging and test_gossip_degrades_gracefully_without_manifest both pass; all existing integration tests pass unchanged; just ready green with 1488 tests
- Notes: `NoopBackend` test helper (all capabilities false, all methods no-op) used to prove degradation paths in isolation. Both degradation paths emit `warn!` events.

### R075 — smelt-agent plugin
- Class: differentiator
- Status: validated
- Description: `plugins/smelt-agent/` directory with `AGENTS.md` system prompt and skills: `run-dispatch.md` (how to read a RunManifest, configure a backend, dispatch a run), `backend-status.md` (how to query `read_run_state`, interpret `OrchestratorStatus`), `peer-message.md` (how to use `send_message`/`poll_inbox` for agent-to-agent coordination across machines).
- Why it matters: Smelt workers run as AI agents — they need a purpose-built prompt that teaches them the backend-aware API surface and coordination patterns
- Source: user
- Primary owning slice: M010/S04
- Supporting slices: M010/S02
- Validation: S04 — plugins/smelt-agent/AGENTS.md (27 lines, ≤60 cap) with skill table, MCP tool table, and workflow overview; skills/run-dispatch.md, skills/backend-status.md, skills/peer-message.md all exist with valid YAML frontmatter; MCP tool names (orchestrate_run, orchestrate_status, run_manifest) verified against server.rs; just ready green
- Notes: Plugin follows the same format as `plugins/claude-code/` and `plugins/codex/`. Skills document the MCP tool signatures stabilised in S02.

## Deferred

### R066 — TUI trace viewer
- Class: primary-user-loop
- Status: deferred
- Description: TUI screen showing recent orchestration traces with span tree, timing, and status. Accessible via a key from Dashboard.
- Why it matters: The TUI is the primary surface — trace inspection should eventually be available without leaving the TUI.
- Source: user
- Primary owning slice: none (deferred to future milestone)
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred per user decision. Depends on R063 (JSON file export) for data source.

### R067 — OTel metrics
- Class: quality-attribute
- Status: deferred
- Description: OTel counters (sessions launched, gates evaluated, merges attempted) and histograms (gate eval latency, agent run duration) alongside tracing.
- Why it matters: Metrics enable dashboards, alerting, and trend analysis that tracing alone cannot provide efficiently.
- Source: user
- Primary owning slice: none (deferred to future milestone)
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred per user decision. Tracing provides the most diagnostic value first.

## Out of Scope

### R030 — Trait objects for adapter dispatch
- Class: anti-feature
- Status: out-of-scope
- Description: Using trait objects or dyn dispatch for harness adapters
- Why it matters: Prevents scope creep toward framework patterns; zero-trait convention is load-bearing
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Closures/callbacks are the pattern (D006)

### R031 — Modifying existing MCP tools
- Class: anti-feature
- Status: out-of-scope
- Description: Adding optional parameters or changing signatures of existing 18 MCP tools
- Why it matters: Preserves backward compatibility for existing consumers
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: New tools are additive only

### R032 — SQLite for session storage
- Class: anti-feature
- Status: out-of-scope
- Description: Using SQLite for session or worktree persistence
- Why it matters: JSON files are sufficient for single-project scope; SQLite adds dependency and migration burden
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Consistent with existing JSON-per-record pattern

### R033 — tmux-based agent orchestration
- Class: anti-feature
- Status: out-of-scope
- Description: Using tmux for agent session management
- Why it matters: `--print` mode is simpler and more reliable for evaluation; tmux is for interactive sessions
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: agtx uses tmux for interactive; Assay doesn't need it

## Traceability

| ID | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|
| R001 | core-capability | validated | M001/S01 | none | S01 |
| R002 | quality-attribute | validated | M001/S01 | none | S01 |
| R003 | core-capability | validated | M001/S02 | none | S02 |
| R004 | core-capability | validated | M001/S02 | M001/S03 | S02 |
| R005 | core-capability | validated | M001/S03 | none | S03 |
| R006 | core-capability | validated | M001/S03 | none | S03 |
| R007 | core-capability | validated | M001/S03 | M001/S04 | S03 |
| R008 | core-capability | validated | M001/S04 | none | S04 |
| R009 | constraint | validated | M001/S04 | M001/S06 | S04 |
| R010 | quality-attribute | validated | M001/S05 | none | S05 |
| R011 | quality-attribute | validated | M001/S05 | none | S05 |
| R012 | core-capability | validated | M001/S05 | none | S05 |
| R013 | quality-attribute | validated | M001/S05 | none | S05 |
| R014 | core-capability | validated | M001/S06 | none | S06 |
| R015 | core-capability | validated | M001/S06 | none | S06 |
| R016 | quality-attribute | validated | M001/S06 | none | S06 |
| R017 | primary-user-loop | validated | M001/S07 | none | S07 |
| R018 | core-capability | validated | M001/S07 | none | S07 |
| R019 | failure-visibility | validated | M001/S07 | none | S07 |
| R020 | core-capability | validated | M002/S02 | M002/S01, M002/S06 | S06 |
| R021 | core-capability | validated | M002/S06 | none | S06 |
| R022 | core-capability | validated | M002/S05 | M002/S06 | S05 |
| R023 | core-capability | validated | M002/S03 | M002/S06 | S03 |
| R024 | differentiator | validated | M002/S04 | M002/S05 | S04 |
| R025 | quality-attribute | deferred | none | none | unmapped |
| R026 | differentiator | validated | M003/S01 | M003/S02 | S01 |
| R027 | quality-attribute | validated | M009/S03 | M009/S01–S05 | S01–S05 |
| R030 | anti-feature | out-of-scope | none | none | n/a |
| R031 | anti-feature | out-of-scope | none | none | n/a |
| R032 | anti-feature | out-of-scope | none | none | n/a |
| R028 | quality-attribute | validated | M003/S02 | none | S02 |
| R029 | failure-visibility | validated | M003/S02 | none | S02 |
| R033 | anti-feature | out-of-scope | none | none | n/a |
| R034 | core-capability | validated | M004/S01 | M004/S02, M004/S03 | S01 |
| R035 | core-capability | validated | M004/S02 | none | S02 |
| R036 | core-capability | validated | M004/S02 | none | S02 |
| R037 | core-capability | validated | M004/S03 | none | S03 |
| R038 | core-capability | validated | M004/S03 | none | S03 |
| R039 | core-capability | validated | M005/S01 | M005/S02, M005/S03, M005/S04 | S01 |
| R040 | core-capability | validated | M005/S01 | M005/S02 | S01 |
| R041 | core-capability | validated | M005/S01 | M005/S02, M005/S03, M005/S04 | S01 |
| R042 | primary-user-loop | validated | M005/S03 | M005/S01 | S03 |
| R043 | core-capability | validated | M005/S02 | M005/S01 | S02 |
| R044 | core-capability | validated | M005/S02 | M005/S01 | S02 |
| R045 | primary-user-loop | validated | M005/S04 | M005/S01, M005/S02 | S04 |
| R046 | convention | validated | M005/S04 | M005/S01 | S04 |
| R047 | differentiator | validated | M005/S05 | M005/S01–S04 | S05 |
| R048 | differentiator | validated | M005/S06 | M005/S01, M005/S02 | S06 |
| R049 | primary-user-loop | validated | M006/S01 | none | S01 |
| R050 | primary-user-loop | validated | M006/S02 | M006/S01 | S02 |
| R051 | primary-user-loop | validated | M006/S03 | M006/S01 | S03 |
| R052 | operability | validated | M006/S04 | M006/S05 | S04, S05 |
| R053 | core-capability | validated | M007/S01 | M006/S01 | S01 |
| R054 | core-capability | validated | M007/S02 | M007/S01 | S02 |
| R055 | operability | validated | M007/S04 | none | S04 |
| R056 | primary-user-loop | validated | M007/S03 | M007/S01 | S03 |
| R057 | differentiator | validated | M008/S03 | none | S03 |
| R058 | primary-user-loop | validated | M008/S02 | M008/S01 | S01, S02 |
| R059 | failure-visibility | validated | M008/S04 | M008/S05 | S04, S05 |

| R060 | quality-attribute | validated | M009/S01 | none | S01 |
| R061 | core-capability | validated | M009/S02 | M009/S01 | S02 |
| R062 | core-capability | validated | M009/S03 | M009/S01, M009/S02 | S03 |
| R063 | core-capability | validated | M009/S04 | M009/S01 | S04 |
| R064 | core-capability | validated | M009/S05 | M009/S01 | S05 |
| R065 | quality-attribute | validated | M009/S05 | M009/S02, M009/S03 | S05 |
| R066 | primary-user-loop | deferred | none | none | unmapped |
| R067 | quality-attribute | deferred | none | none | unmapped |
| R071 | core-capability | validated | M010/S01 | M010/S02 | S01 |
| R072 | quality-attribute | validated | M010/S02 | none | S02 |
| R073 | core-capability | validated | M010/S02 | M010/S03 | S02 |
| R074 | core-capability | validated | M010/S03 | M010/S02 | S03 |
| R075 | differentiator | validated | M010/S04 | M010/S02 | S04 |
| R076 | core-capability | validated | M011/S02 | M011/S01 | S02 |
| R077 | core-capability | validated | M011/S03 | M011/S01 | S03 |
| R078 | core-capability | active | M011/S04 | M011/S01 | unmapped |
| R079 | core-capability | validated | M011/S01 | M011/S04 | S01 |

## Coverage Summary

- Active requirements: 1 (R078)
- Validated: 70 (R001–R029 except R025, R034–R065, R071–R077, R079)
- Unmapped active requirements: 0
- Deferred: 3 (R025, R066, R067)
- Out of scope: 4 (R030, R031, R032, R033)
- Unmapped active requirements: 0
