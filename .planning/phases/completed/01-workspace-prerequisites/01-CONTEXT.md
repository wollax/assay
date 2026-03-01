# Phase 1: Workspace Prerequisites - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate the mandatory schemars 0.8→1.x blocker and add all new workspace dependencies so every downstream phase can build cleanly. Scaffold the `assay-mcp` library crate. No business logic, no new features — pure infrastructure.

</domain>

<decisions>
## Implementation Decisions

### schemars migration strategy
- Pin schemars to the exact 1.x version that rmcp requires — no caret range
- Fix breaking derives/attributes inline during the upgrade (no separate audit step)
- Minimal feature usage — derive macros only, no advanced 1.x features like `transform` until needed
- Zero-breakage tolerance: every existing `JsonSchema` derive must compile without changes beyond the version bump itself

### assay-mcp crate design
- Claude's discretion on all design decisions for the scaffold:
  - Whether assay-mcp depends on assay-core directly or only assay-types
  - Whether to re-export rmcp types or keep them internal
  - Whether to start with just `lib.rs` or include module stubs
  - Whether tokio is a workspace dependency or scoped to assay-mcp only

### Workspace dependency policy
- Claude's discretion on version pinning strategy per crate (exact vs caret based on maturity)
- Claude's discretion on feature flag selection per crate
- Claude's discretion on cargo-deny updates (handle as needed to keep `just deny` green)
- Claude's discretion on TOML crate choice (`toml` vs `toml_edit`) based on downstream needs

### Claude's Discretion
- assay-mcp internal structure and dependency graph placement
- Version pinning strategy per new dependency
- Feature flags for rmcp, tokio, tracing
- TOML crate selection
- cargo-deny configuration updates

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The only hard constraint is that schemars must be pinned to the exact version rmcp needs, and existing derives must not break.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-workspace-prerequisites*
*Context gathered: 2026-02-28*
