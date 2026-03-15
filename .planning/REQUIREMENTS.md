# Requirements: v0.5.0 Single-Agent Harness End-to-End

## Prerequisites

- [ ] **PREREQ-01**: AgentSession (gate evaluation context) persists to disk via write-through cache, surviving MCP server restarts without losing active evaluation sessions
- [ ] **PREREQ-02**: AgentSession renamed to GateEvalContext across assay-types and assay-mcp, with Smelt concepts renamed: manifest → RunManifest, runner → RunExecutor

## Harness

- [ ] **HARNESS-01**: `assay-harness` crate exists as a leaf in the workspace dependency graph, depending on assay-core and assay-types
- [ ] **HARNESS-02**: `HarnessProfile` type in assay-types describes a complete agent configuration: prompt template, settings, and hook definitions
- [ ] **HARNESS-03**: Layered prompt builder assembles system prompts from composable layers: project conventions (always) → spec criteria (when spec provided)
- [ ] **HARNESS-04**: Layered settings merger combines project config base settings with spec-specific overrides (permissions, model, tool access)
- [ ] **HARNESS-05**: Hook contract definitions in assay-types declare lifecycle events (pre-tool, post-tool, stop) that harness adapters translate to harness-specific formats
- [ ] **HARNESS-06**: Claude Code adapter generates CLAUDE.md content, .mcp.json, settings overrides, and hooks.json from a HarnessProfile
- [ ] **HARNESS-07**: Agent invocation uses callback-based control inversion (closures passed to core orchestration functions), not trait objects

## Worktree Enhancements

- [ ] **WTREE-01**: Orphan detection identifies worktrees with no active WorkSession linked
- [ ] **WTREE-02**: Collision prevention rejects worktree creation when spec already has an active worktree with an in-progress session
- [ ] **WTREE-03**: WorktreeMetadata includes `session_id: Option<String>` for session linkage
- [ ] **WTREE-04**: 15 worktree tech debt issues resolved (error chain, base_dir type, detect_main conflation, dirty error advice, env var docs, MCP cleanup --all, deny_unknown_fields, prune failure, 3 missing tests, to_string_lossy, field duplication, schema registry, usize serialization)

## Manifest

- [ ] **MANIFEST-01**: `RunManifest` type in assay-types represents a declarative description of work using `[[sessions]]` TOML array format
- [ ] **MANIFEST-02**: Single-session manifest parsing and validation from TOML files, with actionable error messages for malformed input
- [ ] **MANIFEST-03**: RunManifest schema is forward-compatible for multi-agent extension (uses `[[sessions]]` array even for single-session)

## End-to-End Pipeline

- [ ] **E2E-01**: Single-agent pipeline executes the full flow: RunManifest → worktree create → agent launch (via harness) → gate evaluate → merge propose
- [ ] **E2E-02**: Pipeline is exposed as an MCP tool or composable MCP tool sequence that agents can invoke
- [ ] **E2E-03**: Pipeline failures at any stage produce structured errors with the stage that failed and recovery guidance

---

## Future Requirements (deferred to v0.6.0+)

- [ ] Multi-agent orchestration: OrchestratorSession, DAG executor, parallel sessions
- [ ] `orchestrate_*` MCP tools (additive, no changes to existing tools)
- [ ] Harness orchestration layer: scope enforcement, multi-agent prompt generation
- [ ] MergeRunner with sequential merge and AI conflict resolution
- [ ] Codex and OpenCode harness adapters
- [ ] SessionCore struct composition for type unification

## Out of Scope

- Trait objects for adapter dispatch — closures/callbacks are the pattern (zero-trait convention)
- Modifying existing MCP tools — new tools are additive
- Multi-session manifest execution — v0.6.0
- AI conflict resolution — v0.6.1
- Smelt infrastructure pivot — tracked on Smelt's roadmap

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PREREQ-01 | — | Pending |
| PREREQ-02 | — | Pending |
| HARNESS-01 | — | Pending |
| HARNESS-02 | — | Pending |
| HARNESS-03 | — | Pending |
| HARNESS-04 | — | Pending |
| HARNESS-05 | — | Pending |
| HARNESS-06 | — | Pending |
| HARNESS-07 | — | Pending |
| WTREE-01 | — | Pending |
| WTREE-02 | — | Pending |
| WTREE-03 | — | Pending |
| WTREE-04 | — | Pending |
| MANIFEST-01 | — | Pending |
| MANIFEST-02 | — | Pending |
| MANIFEST-03 | — | Pending |
| E2E-01 | — | Pending |
| E2E-02 | — | Pending |
| E2E-03 | — | Pending |
