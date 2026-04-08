# Requirements: v0.5.0 Single-Agent Harness End-to-End

## Prerequisites

- [x] **PREREQ-01**: GateEvalContext persists to disk via write-through cache (atomic rename), surviving MCP server restarts
- [x] **PREREQ-02**: AgentSession renamed to GateEvalContext across assay-types and assay-mcp; manifest → RunManifest, runner → RunExecutor

## Harness

- [x] **HARNESS-01**: `assay-harness` crate exists as a leaf in the workspace dependency graph
- [x] **HARNESS-02**: `HarnessProfile` type in assay-types describes complete agent configuration
- [x] **HARNESS-03**: Layered prompt builder assembles system prompts from composable layers
- [x] **HARNESS-04**: Layered settings merger combines project config with spec-specific overrides
- [x] **HARNESS-05**: Hook contract definitions declare lifecycle events (pre-tool, post-tool, stop)
- [x] **HARNESS-06**: Claude Code adapter generates CLAUDE.md, .mcp.json, settings, hooks.json
- [x] **HARNESS-07**: HarnessProvider trait with Claude/Codex/OpenCode adapters (exceeded original scope)

## Worktree Enhancements

- [x] **WTREE-01**: Orphan detection via `is_orphan` field on WorktreeInfo
- [x] **WTREE-02**: Collision prevention for duplicate active worktrees per spec
- [x] **WTREE-03**: WorktreeMetadata includes `session_id: Option<String>` for session linkage
- [~] **WTREE-04**: Worktree tech debt mostly resolved — `WorktreeConfig.base_dir` intentionally kept as `String` (schema-breaking change avoided)

## Manifest

- [x] **MANIFEST-01**: `RunManifest` type with `sessions: Vec<ManifestSession>` + `[[sessions]]` TOML array
- [x] **MANIFEST-02**: Single-session manifest parsing with TOML round-trip tests
- [x] **MANIFEST-03**: Forward-compatible for multi-agent (orchestration modes, depends_on, file_scope, shared_files)

## End-to-End Pipeline

- [x] **E2E-01**: Single-agent pipeline: RunManifest → worktree → harness → agent → gate → merge
- [x] **E2E-02**: Pipeline exposed as `run_manifest` MCP tool
- [x] **E2E-03**: Structured `PipelineError { stage, message, recovery }` errors

---

## v0.6.0+ Requirements (verified 2026-04-08)

- [x] Multi-agent orchestration: DAG executor + Mesh + Gossip parallel sessions
- [x] `orchestrate_run` + `orchestrate_status` MCP tools (additive)
- [x] Harness orchestration layer: scope enforcement (`check_scope`), multi-agent prompt generation
- [x] MergeRunner with sequential merge + AI conflict resolution (Claude subprocess, validation command)
- [x] Codex and OpenCode harness adapters (full implementations)
- [ ] SessionCore struct composition for type unification (deferred — cosmetic refactor)

## Out of Scope

- Trait objects for adapter dispatch — closures/callbacks are the pattern (zero-trait convention)
- Modifying existing MCP tools — new tools are additive
- Multi-session manifest execution — v0.6.0
- AI conflict resolution — v0.6.1
- Smelt infrastructure pivot — tracked on Smelt's roadmap

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PREREQ-01 | 52 | ✅ Done |
| PREREQ-02 | 51 | ✅ Done |
| HARNESS-01 | 55 | ✅ Done |
| HARNESS-02 | 55 | ✅ Done |
| HARNESS-03 | 56 | ✅ Done |
| HARNESS-04 | 56 | ✅ Done |
| HARNESS-05 | 57 | ✅ Done |
| HARNESS-06 | 57 | ✅ Done |
| HARNESS-07 | 57 | ✅ Done |
| WTREE-01 | 53 | ✅ Done |
| WTREE-02 | 53 | ✅ Done |
| WTREE-03 | 53 | ✅ Done |
| WTREE-04 | 54 | ⚠️ Partial (base_dir kept as String) |
| MANIFEST-01 | 58 | ✅ Done |
| MANIFEST-02 | 58 | ✅ Done |
| MANIFEST-03 | 58 | ✅ Done |
| E2E-01 | 59 | ✅ Done |
| E2E-02 | 59 | ✅ Done |
| E2E-03 | 59 | ✅ Done |
