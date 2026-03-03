# Phase 9: CLI Surface Completion - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Polish the CLI surface so all subcommands work end-to-end, help text is complete with examples, and `plugin.json` metadata is accurate. The CLI subcommands themselves were built incrementally in Phases 5-8. This phase adds the finishing layer: rich help, consistent errors, status display on bare invocation, and plugin manifest finalization.

Requirement: PLG-01 (`plugin.json` manifest with name, version, description, author).

</domain>

<decisions>
## Implementation Decisions

### Help text quality
- Rich help with usage examples (like `cargo` or `gh` help style)
- Examples should cover both human CLI usage and agent/JSON usage (both audiences)
- Every subcommand and flag gets a clear description plus inline examples
- No specific style reference — Claude picks a clean, functional format

### plugin.json metadata
- Author: `wollax`
- Description: "Agentic development kit with spec-driven workflows" (matches CLI about text)
- Include optional fields: `homepage` (repository URL) and `license`
- Version must auto-sync from Cargo.toml workspace version (build step or just recipe)

### Error message consistency
- Error prefix style: Claude's discretion (pick based on Rust CLI conventions)
- Include hint/suggestion lines where genuinely useful (e.g., "hint: run `assay spec list` to see available specs")
- Exit codes: Claude's discretion (pick what's useful for CI/agent integration)
- JSON error formatting when `--json` is used: Claude's discretion

### Version and status display
- `assay --version` format: Claude's discretion (pick what's most useful for a dev tool)
- Bare `assay` (no args) in an initialized project: show status summary (Claude designs the layout based on available info)
- Bare `assay` (no args) outside a project: hint to init ("Not an Assay project. Run `assay init` to get started.") then show help
- About text placement: Claude's discretion

### Claude's Discretion
- Error prefix convention (`error:` vs `assay:` vs none)
- Exit code granularity (0/1 vs semantic codes)
- JSON error formatting approach
- Version display format (bare version vs version + build info)
- Status display layout and content
- About text placement in status/help

</decisions>

<specifics>
## Specific Ideas

- Status display when running bare `assay` in an initialized project — quick project health at a glance
- When NOT in an assay project, hint to `assay init` before showing help — guide new users
- Help examples should demonstrate the dual nature of the tool (human + agent audiences)
- Error hints should be like rustc's `help:` lines — actionable suggestions, not just error descriptions

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 09-cli-surface-completion*
*Context gathered: 2026-03-02*
