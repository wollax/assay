# Phase 68: MCP Surface - Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Five new MCP tools expose agent-driven gate composition — `gate_wizard`, `criteria_list`, `criteria_get`, `criteria_create`, and `spec_resolve` — each delegating validation to `assay-core::wizard` and `assay-core::spec::compose`. All tools follow the established MCP handler pattern (parameter structs with `Deserialize + JsonSchema`, async handlers returning `CallToolResult`). No core logic changes — MCP tools are thin wrappers.

Out of scope: TUI wizard (Phase 69), changes to existing MCP tools, deprecation warnings on `spec_create`.

</domain>

<decisions>
## Implementation Decisions

### gate_wizard vs spec_create coexistence
- `gate_wizard` supersedes `spec_create` as THE tool for creating/editing gates — it's a strict superset with composability fields (`extends`, `include`, `preconditions`) plus edit mode
- `spec_create` stays for backward compatibility — no deprecation warning, no removal. Just stop recommending it in new docs/skills
- `gate_wizard` uses `CriterionInput` objects only (name + description + optional cmd) — no bare string criteria (no `CriterionOrString` union)

### gate_wizard tool design
- MCP params: reuse `GateWizardInput` directly as the parameter struct — schema auto-generated from the existing `assay-types` type, zero wrapper overhead
- Response: return path + full serialized `GatesSpec` — matches `GateWizardOutput` already built in Phase 67. Agent can verify what was written without a follow-up `spec_get` call
- Delegates to `apply_gate_wizard()` — no reimplemented validation

### spec_resolve tool design
- Params: spec name only (string) — consistent with `spec_get` / `gate_run` pattern. No inline `GatesSpec` input; agents use `gate_wizard` first, then `spec_resolve` to inspect
- Response: full `ResolvedGate` struct with per-criterion `CriterionSource` annotations (Own/Parent/Library). No pre-resolved raw `GatesSpec` — agent has `spec_get` for that
- Include shadow warnings in the response — when own criterion overrides parent/library by name, surface it in a warnings field. Lightweight since `resolve()` already detects these
- Error on missing refs — if `extends` or `include` references a missing gate/library, return an MCP error (consistent with `compose::resolve()` fail-fast semantics). No partial results

### criteria_list tool design
- Response: slug + criterion count + description (if present) per entry — compact, enough for agents to decide which to `criteria_get`
- Delegates to `compose::scan_libraries()`

### criteria_get tool design
- Response: full deserialized `CriteriaLibrary` object (name, description, version, tags, criteria array) — consistent with `spec_get` returning the full spec
- Delegates to `compose::load_library_by_slug()`

### criteria_create tool design
- MCP params: reuse `CriteriaWizardInput` directly — same pattern as `gate_wizard`
- Response: path + full `CriteriaLibrary` — matches `CriteriaWizardOutput` from Phase 67
- Delegates to `apply_criteria_wizard()`

### Error surfaces
- Fuzzy slug suggestions on not-found errors — reuse `enriched_error_display` pattern for `criteria_get` and `spec_resolve`. Agents self-correct without a separate list call
- Single error message for validation errors (fail-fast) — first validation error as prose, matching existing `AssayError` display. No structured field-level error objects
- Universal optional `warnings: Vec<String>` field on all five tool responses — consistent with existing MCP tools (e.g., `gate_run`). Only populated when there's something to report (shadows, empty includes, etc.)

### Claude's Discretion
- Whether `criteria_list` summary struct is a new named type in `server.rs` or an inline struct
- How shadow detection is surfaced in spec_resolve — extract from resolve() or re-derive from the ResolvedGate
- Exact warning messages and formatting
- Whether to add tool registration tests (the existing `*_tool_in_router` pattern) — recommended to follow convention
- Internal code organization within `server.rs` (tool ordering, grouping with existing tools)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `GateWizardInput` / `GateWizardOutput` (`assay-types/src/wizard_input.rs`): MCP params type with `JsonSchema` derive — use directly as tool params
- `CriteriaWizardInput` / `CriteriaWizardOutput` (`assay-types/src/wizard_input.rs`): Same pattern for criteria_create
- `apply_gate_wizard()` (`assay-core/src/wizard/gate.rs:25`): Core entry point for gate creation/edit
- `apply_criteria_wizard()` (`assay-core/src/wizard/criteria.rs:26`): Core entry point for library creation
- `compose::scan_libraries()` (`assay-core/src/spec/compose.rs:136`): Returns `Vec<CriteriaLibrary>` for criteria_list
- `compose::load_library_by_slug()` (`assay-core/src/spec/compose.rs:168`): Loads single library for criteria_get
- `compose::resolve()` (`assay-core/src/spec/compose.rs:243`): Closure-based resolution for spec_resolve
- `enriched_error_display` (`assay-core/src/gate/mod.rs:474`): Fuzzy slug matching for not-found errors
- Existing MCP param structs (`assay-mcp/src/server.rs`): Pattern reference — `Deserialize + JsonSchema`, `#[schemars(description)]` on fields

### Established Patterns
- MCP handler: `pub async fn tool_name(&self, params: Parameters<Params>) -> Result<CallToolResult, McpError>`
- Domain errors → `CallToolResult` with `isError: true`; protocol errors → `McpError`
- `resolve_cwd()` + `load_config()` + `assay_dir`/`specs_dir` from config at handler start
- `tool_router!` macro registers all tools; `list_all()` returns tool list for tests
- Response serialized as JSON text content in `CallToolResult`
- Tool registration tests: `#[tokio::test] async fn tool_name_tool_in_router()` pattern

### Integration Points
- `crates/assay-mcp/src/server.rs`: Five new handler methods + five new param structs (or reused types) + `tool_router!` registration
- `crates/assay-mcp/src/server.rs` module doc comment: Update tool list at top of file
- No changes to `assay-core` or `assay-types` — everything needed exists from Phases 65 + 67

</code_context>

<specifics>
## Specific Ideas

- All five tools are thin wrappers: resolve paths from config, deserialize params, call core function, serialize response. The MCP surface should be the thinnest surface of the three (CLI, MCP, TUI).
- The `warnings` field pattern already exists in the codebase (gate_run responses) — follow that exact shape.
- `spec_resolve` is the most interesting tool: it calls `resolve()` with closure-based loaders, then checks for shadow overrides to populate warnings. This is the only tool that does more than delegate-and-serialize.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 68-mcp-surface*
*Context gathered: 2026-04-12*
