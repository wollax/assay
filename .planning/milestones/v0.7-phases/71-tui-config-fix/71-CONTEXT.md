# Phase 71: TUI Config Fix - Context

**Gathered:** 2026-04-13
**Status:** Ready for planning

<domain>
## Phase Boundary

TUI reads `config.specs_dir` instead of hardcoding `root.join(".assay").join("specs")`, so projects with non-default specs directories work correctly. Pure bug fix — no new features or UI changes.

</domain>

<decisions>
## Implementation Decisions

### Config fallback
- When `self.config` is `None` (no config.toml), fall back to default `"specs/"` silently — same behavior as CLI
- No warning or error needed for missing config; the default value already matches historical behavior

### slash.rs config threading
- Pass resolved `specs_dir: &Path` into `execute_slash_cmd` rather than the full `Config` struct
- Keeps function signature minimal — caller (app.rs) resolves specs_dir from config before calling
- Matches the principle of resolving config values at the surface boundary, not deep in dispatch

### Claude's Discretion
- Whether to extract a helper method for specs_dir resolution on App or inline it at each call site
- Test approach for verifying the fix (existing test patterns in gate_wizard_app.rs and slash_commands.rs)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — mechanical fix following the established CLI pattern (`assay_dir.join(&config.specs_dir)`).

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `self.config: Option<assay_types::Config>` already loaded on App struct (app.rs:192)
- `default_specs_dir()` in assay-types returns `"specs/"` — ensures backward compat
- CLI pattern in gate.rs, spec.rs: `assay_dir.join(&config.specs_dir)` — proven, tested

### Established Patterns
- CLI loads config via `assay_core::config::load(root)` and resolves `specs_dir` at each command handler
- TUI loads config once at App::new() and stores as `Option<Config>`
- `execute_slash_cmd` in slash.rs is a pure dispatch function taking minimal params

### Integration Points
- app.rs: ~10 sites where `assay_dir.join("specs")` must become config-aware
- slash.rs: `execute_slash_cmd` signature needs `specs_dir` parameter added
- Tests: gate_wizard_app.rs, slash_commands.rs, spec_browser.rs — may need adjustment for new param

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 71-tui-config-fix*
*Context gathered: 2026-04-13*
