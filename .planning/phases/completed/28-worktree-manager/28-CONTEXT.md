# Phase 28: Worktree Manager - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement git worktree lifecycle management — create, list, status, cleanup — with CLI subcommands, MCP tools, and configurable paths. Worktrees are always associated with a spec. This phase does NOT include orchestration logic, agent launching, or merge-back pipelines.

</domain>

<decisions>
## Implementation Decisions

### Worktree directory layout
- Base path is **configurable** with a **sibling directory default** (e.g., `../project-worktrees/`)
- Configuration lives in `.assay/config.toml` under a `[worktree]` section (e.g., `base_dir = "..."`)
- CLI flag `--worktree-dir` and env var `ASSAY_WORKTREE_DIR` can override the config per-invocation
- Precedence: CLI flag > env var > config file > default

### Cleanup behavior
- Uncommitted changes trigger an **interactive confirmation prompt**; non-interactive mode fails safely
- Cleanup **deletes the associated branch** along with the worktree directory
- Bulk cleanup supported via `--all` flag, with confirmation prompt listing all worktrees to be removed
- `--force` flag available to bypass confirmation (for CI/scripting)

### Spec-to-worktree mapping
- Worktree creation **requires a valid spec** — fails if the spec doesn't exist
- One worktree per spec (duplicate handling is Claude's discretion)
- Base branch defaults to main/default, overridable with `--base <branch>`
- Branch naming follows `assay/<spec-slug>` pattern

### CLI & MCP output shape
- All CLI subcommands support `--json` for machine-readable output
- `worktree create` prints a human-friendly message by default, `--json` gives structured output with path field
- MCP tool surface design is Claude's discretion (1:1 mirror vs consolidated)

### Claude's Discretion
- Worktree directory naming scheme within the base path (slug format, prefixing)
- Spec slug derivation strategy (from filename vs title)
- Duplicate worktree handling (error vs idempotent return)
- `worktree list` default display format (compact table vs detailed cards)
- MCP tool granularity (individual tools vs consolidated)
- Git ref pruning strategy (always vs only on bulk cleanup)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 28-worktree-manager*
*Context gathered: 2026-03-09*
