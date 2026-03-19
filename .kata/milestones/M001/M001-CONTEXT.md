# M001: Single-Agent Harness End-to-End — Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

## Project Description

Assay is a spec-driven quality gate system for AI coding agents. M001 adds the harness layer: a complete pipeline from declarative manifest → worktree creation → agent launch (via Claude Code) → gate evaluation → merge proposal. This transforms Assay from a gate evaluation toolkit into a full agent orchestration primitive.

## Why This Milestone

Assay can evaluate code quality but cannot yet launch or manage the agents that produce the code. The harness closes this gap — the user writes a manifest declaring what spec to implement, and Assay handles the entire lifecycle. Without this, orchestration requires manual glue between worktree creation, agent invocation, and gate evaluation.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Write a TOML manifest declaring a spec to implement, and run the full pipeline via CLI or MCP tool
- See structured errors at each pipeline stage with recovery guidance if anything fails
- Have Claude Code launched in an isolated worktree with auto-generated CLAUDE.md, .mcp.json, settings, and hooks

### Entry point / environment

- Entry point: `assay run <manifest.toml>` CLI command and `run_manifest` MCP tool
- Environment: local dev (worktrees on local filesystem, git CLI, claude CLI)
- Live dependencies involved: `git` CLI, `claude` CLI (Claude Code)

## Completion Class

- Contract complete means: all types compile, all tests pass, `just ready` green, manifest parsing round-trips correctly
- Integration complete means: the pipeline can be exercised with a real spec, real worktree, and real Claude Code invocation against a test repo
- Operational complete means: pipeline handles Claude Code timeout, crash, and exit-code failures gracefully

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- A manifest file can drive the full pipeline: parse → worktree create → harness config generate → agent launch → gate evaluate → merge propose
- Pipeline failures at worktree, agent, and gate stages produce structured errors with recovery guidance
- The generated Claude Code harness config (CLAUDE.md, .mcp.json, hooks.json) is valid and functional

## Risks and Unknowns

- Claude Code `--print` mode JSON output format may have changed since research (March 2026) — validate early
- Hook contract (pre-tool, post-tool, stop) interaction with Claude Code's actual hooks.json format needs verification
- Process lifecycle management (timeout, kill, orphan cleanup) for long-running agents has sharp edges

## Existing Codebase / Prior Art

- `crates/assay-core/src/worktree.rs` (1061 lines) — worktree CRUD, already functional
- `crates/assay-core/src/work_session.rs` (1335 lines) — session lifecycle, fully implemented
- `crates/assay-core/src/merge.rs` (627 lines) — merge_check implemented
- `crates/assay-core/src/evaluator.rs` (1159 lines) — headless agent evaluation via claude --print
- `crates/assay-core/src/gate/mod.rs` — gate evaluation with subprocess pattern (template for harness)
- `crates/assay-mcp/src/server.rs` — 18 MCP tools, additive-only pattern
- `crates/assay-types/src/session.rs` — AgentSession (to be renamed GateEvalContext)
- `crates/assay-types/src/work_session.rs` — WorkSession type
- `crates/assay-types/src/worktree.rs` — WorktreeMetadata type

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R001–R002: Prerequisites (persistence + rename)
- R003–R009: Harness crate, profile, prompt builder, settings, hooks, adapter, callbacks
- R010–R013: Worktree enhancements (orphan detection, collision prevention, session linkage, tech debt)
- R014–R016: RunManifest type, parsing, forward compatibility
- R017–R019: End-to-end pipeline, MCP exposure, structured errors

## Scope

### In Scope

- GateEvalContext persistence (write-through cache to disk)
- AgentSession → GateEvalContext rename across codebase
- New `assay-harness` leaf crate
- HarnessProfile type in assay-types
- Layered prompt builder and settings merger
- Hook contract definitions
- Claude Code adapter (CLAUDE.md, .mcp.json, settings, hooks.json generation)
- Worktree enhancements (orphan detection, collision prevention, session linkage)
- 15 worktree tech debt issues
- RunManifest type with `[[sessions]]` TOML format
- End-to-end pipeline: manifest → worktree → harness → agent → gate → merge propose
- Pipeline MCP tool(s)
- Structured pipeline errors with stage context

### Out of Scope / Non-Goals

- Multi-agent orchestration (M002)
- Codex/OpenCode adapters (M003)
- AI conflict resolution (M003)
- TUI screens for pipeline monitoring (future)
- tmux-based agent management
- Trait objects for adapter dispatch

## Technical Constraints

- Zero new workspace dependencies unless absolutely necessary
- Zero-trait convention: closures/callbacks for control inversion
- MCP tools are additive only — never modify existing tool signatures
- `deny_unknown_fields` on all persisted types
- Sync core, async surfaces (MCP handlers use `spawn_blocking`)
- Shell out to `git` and `claude` CLI — no library bindings

## Integration Points

- `git` CLI — worktree create/remove, branch management, merge-tree
- `claude` CLI — `--print` mode with `--output-format json` for agent invocation
- Existing MCP server — new tools added alongside existing 18
- Existing worktree module — enhanced with session linkage and orphan detection
- Existing work_session module — integrated into pipeline flow
- Existing evaluator module — pattern reference for harness subprocess management

## Open Questions

- Hook contract: what exact lifecycle events does Claude Code support in hooks.json? — Verify against current Claude Code docs during S03
- Should the pipeline MCP tool be a single `run_manifest` or composed sequence? — Decide during S07 planning based on what feels natural after building the pieces
