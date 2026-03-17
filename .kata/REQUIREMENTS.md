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
- Status: active
- Description: Orphan detection identifies worktrees with no active WorkSession linked
- Why it matters: Worktrees leak disk and git refs without lifecycle tracking
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: unmapped
- Notes: Part of worktree enhancement work

### R011 — Worktree collision prevention
- Class: quality-attribute
- Status: active
- Description: Collision prevention rejects worktree creation when spec already has an active worktree with an in-progress session
- Why it matters: Two worktrees for the same spec causes merge conflicts and wasted work
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: unmapped
- Notes: Guards against double-launch

### R012 — WorktreeMetadata session linkage
- Class: core-capability
- Status: active
- Description: WorktreeMetadata includes `session_id: Option<String>` for session linkage
- Why it matters: Connects worktree lifecycle to session lifecycle for orphan detection
- Source: inferred
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: unmapped
- Notes: Additive field on existing type

### R013 — Worktree tech debt resolution
- Class: quality-attribute
- Status: active
- Description: 15 worktree tech debt issues resolved (error chain, base_dir type, detect_main conflation, dirty error advice, env var docs, MCP cleanup --all, deny_unknown_fields, prune failure, 3 missing tests, to_string_lossy, field duplication, schema registry, usize serialization)
- Why it matters: Tech debt compounds; cleaning up before adding harness integration prevents fragile foundations
- Source: execution
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: unmapped
- Notes: 15 tracked issues from v0.4.x work

### R014 — RunManifest type
- Class: core-capability
- Status: active
- Description: `RunManifest` type in assay-types represents a declarative description of work using `[[sessions]]` TOML array format
- Why it matters: The manifest is the entry point for the entire pipeline
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: unmapped
- Notes: Forward-compatible for multi-agent via `[[sessions]]` array

### R015 — Manifest parsing and validation
- Class: core-capability
- Status: active
- Description: Single-session manifest parsing and validation from TOML files, with actionable error messages for malformed input
- Why it matters: Users author manifests by hand — errors must be helpful
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: unmapped
- Notes: TOML parsing, schema validation, clear error messages

### R016 — Manifest forward compatibility
- Class: quality-attribute
- Status: active
- Description: RunManifest schema is forward-compatible for multi-agent extension (uses `[[sessions]]` array even for single-session)
- Why it matters: Avoids breaking change when M002 adds multi-session support
- Source: user
- Primary owning slice: M001/S06
- Supporting slices: none
- Validation: unmapped
- Notes: Design constraint, not runtime behavior

### R017 — Single-agent end-to-end pipeline
- Class: primary-user-loop
- Status: active
- Description: Single-agent pipeline executes the full flow: RunManifest → worktree create → agent launch (via harness) → gate evaluate → merge propose
- Why it matters: This is the core value loop — the reason assay exists
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: unmapped
- Notes: Capstone slice, composes everything

### R018 — Pipeline as MCP tool
- Class: core-capability
- Status: active
- Description: Pipeline is exposed as an MCP tool or composable MCP tool sequence that agents can invoke
- Why it matters: Agents need to trigger the pipeline programmatically
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: unmapped
- Notes: Could be single tool or composed sequence of existing tools

### R019 — Pipeline structured errors
- Class: failure-visibility
- Status: active
- Description: Pipeline failures at any stage produce structured errors with the stage that failed and recovery guidance
- Why it matters: Agents need to know what failed and how to recover
- Source: user
- Primary owning slice: M001/S07
- Supporting slices: none
- Validation: unmapped
- Notes: Error types must include pipeline stage context

## Deferred

### R020 — Multi-agent orchestration
- Class: core-capability
- Status: deferred
- Description: OrchestratorSession, DAG executor, parallel sessions with dependency ordering
- Why it matters: Enables parallel agent work on independent specs
- Source: user
- Primary owning slice: M002 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred to M002 — requires single-agent foundation first

### R021 — Orchestration MCP tools
- Class: core-capability
- Status: deferred
- Description: `orchestrate_*` MCP tools (additive, no changes to existing tools)
- Why it matters: Programmatic access to multi-agent orchestration
- Source: user
- Primary owning slice: M002 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Additive tools, no modification to existing 18

### R022 — Harness orchestration layer
- Class: core-capability
- Status: deferred
- Description: Scope enforcement, multi-agent prompt generation
- Why it matters: Multi-agent needs coordinated prompting and scope boundaries
- Source: user
- Primary owning slice: M002 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Depends on single-agent harness being stable

### R023 — MergeRunner with sequential merge
- Class: core-capability
- Status: deferred
- Description: Sequential merge runner with AI conflict resolution
- Why it matters: Multiple agents produce branches that must be merged in dependency order
- Source: user
- Primary owning slice: M002 (provisional)
- Supporting slices: M003
- Validation: unmapped
- Notes: merge_check already exists; this adds automated execution

### R024 — Additional harness adapters
- Class: differentiator
- Status: deferred
- Description: Codex and OpenCode harness adapters
- Why it matters: Multi-harness support broadens adoption
- Source: user
- Primary owning slice: M003 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Claude Code adapter in M001 establishes the pattern

### R025 — SessionCore type unification
- Class: quality-attribute
- Status: deferred
- Description: SessionCore struct composition for type unification across session concepts
- Why it matters: Reduces confusion from 5+ "session" types
- Source: inferred
- Primary owning slice: M003 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred until API stabilizes through usage

### R026 — AI conflict resolution
- Class: differentiator
- Status: deferred
- Description: AI-powered conflict resolution via evaluator when merge conflicts arise
- Why it matters: Enables fully autonomous merge flows
- Source: user
- Primary owning slice: M003 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Requires MergeRunner foundation from M002

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
| R010 | quality-attribute | active | M001/S05 | none | unmapped |
| R011 | quality-attribute | active | M001/S05 | none | unmapped |
| R012 | core-capability | active | M001/S05 | none | unmapped |
| R013 | quality-attribute | active | M001/S05 | none | unmapped |
| R014 | core-capability | active | M001/S06 | none | unmapped |
| R015 | core-capability | active | M001/S06 | none | unmapped |
| R016 | quality-attribute | active | M001/S06 | none | unmapped |
| R017 | primary-user-loop | active | M001/S07 | none | unmapped |
| R018 | core-capability | active | M001/S07 | none | unmapped |
| R019 | failure-visibility | active | M001/S07 | none | unmapped |
| R020 | core-capability | deferred | M002 | none | unmapped |
| R021 | core-capability | deferred | M002 | none | unmapped |
| R022 | core-capability | deferred | M002 | none | unmapped |
| R023 | core-capability | deferred | M002 | M003 | unmapped |
| R024 | differentiator | deferred | M003 | none | unmapped |
| R025 | quality-attribute | deferred | M003 | none | unmapped |
| R026 | differentiator | deferred | M003 | none | unmapped |
| R030 | anti-feature | out-of-scope | none | none | n/a |
| R031 | anti-feature | out-of-scope | none | none | n/a |
| R032 | anti-feature | out-of-scope | none | none | n/a |
| R033 | anti-feature | out-of-scope | none | none | n/a |

## Coverage Summary

- Active requirements: 10
- Mapped to slices: 19
- Validated: 9
- Unmapped active requirements: 0
