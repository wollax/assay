# Requirements

## Active

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
- Status: active
- Description: AI-powered conflict resolution via evaluator when merge conflicts arise
- Why it matters: Enables fully autonomous merge flows
- Source: user
- Primary owning slice: M003/S01
- Supporting slices: M003/S02
- Validation: unmapped
- Notes: Two-phase merge lifecycle (D044), sync subprocess (D043). S01 builds the resolver; S02 adds audit trail and validation.

### R027 — OpenTelemetry instrumentation
- Class: quality-attribute
- Status: deferred
- Description: OTel tracing spans and metrics across pipeline stages, session lifecycle, merge phases, and harness generation
- Why it matters: Observability is critical for debugging multi-agent orchestration at scale — which session is slow, where merges fail, harness generation latency
- Source: user
- Primary owning slice: M004+ (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Cross-cutting concern — better as a dedicated pass after orchestration and harness surfaces stabilize. M002 should identify span boundaries but not wire them.

### R028 — Post-resolution validation
- Class: quality-attribute
- Status: active
- Description: After AI resolves a conflict, run a configurable validation command (e.g., `cargo check`) before accepting the merge commit
- Why it matters: Without validation, AI resolution is a trust-me black box that can introduce subtle semantic errors
- Source: M003 research
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: unmapped
- Notes: Validation command is optional, configurable in ConflictResolutionConfig. Non-zero exit rejects the resolution and aborts the merge.

### R029 — Conflict resolution audit trail
- Class: failure-visibility
- Status: active
- Description: Record original conflict markers, resolved content, and resolver output in MergeReport for every resolved conflict
- Why it matters: Critical for debugging when AI resolutions introduce subtle bugs — without an audit trail, the resolution is opaque
- Source: M003 research
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: unmapped
- Notes: Recorded as Vec<ConflictResolution> on MergeReport. Viewable via CLI --json and orchestrate_status MCP tool.

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
| R026 | differentiator | active | M003/S01 | M003/S02 | unmapped |
| R027 | quality-attribute | deferred | M004+ | none | unmapped |
| R030 | anti-feature | out-of-scope | none | none | n/a |
| R031 | anti-feature | out-of-scope | none | none | n/a |
| R032 | anti-feature | out-of-scope | none | none | n/a |
| R028 | quality-attribute | active | M003/S02 | none | unmapped |
| R029 | failure-visibility | active | M003/S02 | none | unmapped |
| R033 | anti-feature | out-of-scope | none | none | n/a |

## Coverage Summary

- Active requirements: 3 (R026, R028, R029)
- Mapped to slices: 3
- Validated: 24
- Deferred: 3 (R025, R027 — with rationale)
- Unmapped active requirements: 0
