# Phase 68: MCP Surface - Research

**Researched:** 2026-04-12
**Domain:** MCP tool handlers in Rust (`assay-mcp/src/server.rs`), rmcp framework patterns
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- `gate_wizard` supersedes `spec_create` as the composability-aware gate tool but `spec_create` stays for backward compatibility — no deprecation, no removal
- `gate_wizard` uses `CriterionInput` objects only (name + description + optional cmd) — no bare string criteria, no `CriterionOrString` union
- MCP params for `gate_wizard`: reuse `GateWizardInput` directly as the parameter struct
- `gate_wizard` response: path + full serialized `GatesSpec` (matches `GateWizardOutput`)
- `gate_wizard` delegates to `apply_gate_wizard()`
- `spec_resolve` params: spec name only (string), consistent with `spec_get` / `gate_run` pattern
- `spec_resolve` response: full `ResolvedGate` struct with per-criterion `CriterionSource` annotations
- `spec_resolve` includes shadow warnings in a `warnings` field
- `spec_resolve` errors on missing refs (fail-fast, no partial results)
- `criteria_list` response: slug + criterion count + description per entry
- `criteria_list` delegates to `compose::scan_libraries()`
- `criteria_get` response: full `CriteriaLibrary` object
- `criteria_get` delegates to `compose::load_library_by_slug()`
- `criteria_create` MCP params: reuse `CriteriaWizardInput` directly
- `criteria_create` response: path + full `CriteriaLibrary` (matches `CriteriaWizardOutput`)
- `criteria_create` delegates to `apply_criteria_wizard()`
- Fuzzy slug suggestions on not-found errors for `criteria_get` and `spec_resolve`
- Single error message for validation errors (fail-fast prose, matching existing `AssayError` display)
- Universal optional `warnings: Vec<String>` field on all five tool responses

### Claude's Discretion
- Whether `criteria_list` summary struct is a new named type in `server.rs` or an inline struct
- How shadow detection is surfaced in `spec_resolve` — extract from `resolve()` or re-derive from the `ResolvedGate`
- Exact warning messages and formatting
- Whether to add tool registration tests (the existing `*_tool_in_router` pattern) — recommended to follow convention
- Internal code organization within `server.rs` (tool ordering, grouping with existing tools)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WIZM-01 | Agent can drive gate wizard via `gate_wizard` MCP tool | `GateWizardInput` type exists in `assay-types` with `JsonSchema` derive; `apply_gate_wizard()` is the core entry point; established handler pattern applies directly |
| WIZM-02 | Agent can discover criteria libraries via `criteria_list` and `criteria_get` MCP tools | `scan_libraries()` and `load_library_by_slug()` exist in `assay-core::spec::compose`; `CriteriaLibrary` type serializes cleanly; fuzzy-match suggestion built in to `load_library_by_slug` |
| WIZM-03 | Agent can resolve a spec's effective criteria via `spec_resolve` MCP tool | `compose::resolve()` accepts closures and returns `ResolvedGate`; shadow detection requires post-process step on the result; load path via `load_spec_entry` + spec dir pattern |
| CLIB-04 | Agent can create criteria libraries programmatically via `criteria_create` MCP tool | `CriteriaWizardInput` type exists with `JsonSchema` derive; `apply_criteria_wizard()` is the core entry point; identical pattern to `gate_wizard` |
</phase_requirements>

## Summary

Phase 68 adds five MCP tools to `crates/assay-mcp/src/server.rs`. Every required core function and type was built in Phases 65 and 67 — this phase is exclusively a thin MCP surface layer. No new types are needed in `assay-types` or `assay-core`.

The established handler pattern is `#[tool(description = "...")]` on `pub async fn tool_name(&self, params: Parameters<ParamsType>) -> Result<CallToolResult, McpError>`, with `domain_error(&e)` for all `AssayError` failures. The `#[tool_router]` macro on the `impl AssayServer` block auto-registers every method annotated with `#[tool]` — adding a new tool is a purely additive change. All I/O operations (`apply_gate_wizard`, `apply_criteria_wizard`, `scan_libraries`, `load_library_by_slug`, `compose::resolve`) must be wrapped in `tokio::task::spawn_blocking` following the established D007 convention.

The most complex tool is `spec_resolve`: it needs to construct closures for `compose::resolve()` that load from `specs_dir` and `assay_dir`, then derive shadow warnings by scanning for criterion names that appear in multiple sources and are overridden. The four remaining tools are straight delegate-and-serialize wrappers.

**Primary recommendation:** Implement all five tools in a single additive pass on `server.rs` — param structs (or reuse of existing types), `#[tool]`-annotated handlers, response structs with `warnings` field, `spawn_blocking` wrapping, and `*_tool_in_router` registration tests. No core changes required.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rmcp` | workspace | MCP tool registration (`#[tool]`, `#[tool_router]`, `Parameters`, `CallToolResult`) | Only MCP framework in the workspace |
| `serde` + `serde_json` | workspace | Struct serialization to JSON text content | Used for all existing tool responses |
| `schemars` | workspace | `JsonSchema` derive for tool parameter structs | Required by `rmcp` for tool schema generation |
| `tokio` | workspace | Async runtime + `spawn_blocking` for sync I/O | All handlers are `async`; I/O is sync |
| `serial_test` | workspace (dev) | Test serialization for tests that mutate `std::env::current_dir` | All handler integration tests use `#[serial]` |
| `tempfile` | workspace (dev) | Temp directories for integration tests | Used by all handler integration tests |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `assay_core::spec::compose` | workspace | `scan_libraries`, `load_library_by_slug`, `resolve` | All five tools delegate here |
| `assay_core::wizard` | workspace | `apply_gate_wizard`, `apply_criteria_wizard` | `gate_wizard` and `criteria_create` |
| `assay_types` | workspace | `GateWizardInput`, `GateWizardOutput`, `CriteriaWizardInput`, `CriteriaWizardOutput`, `CriteriaLibrary`, `ResolvedGate` | Direct param reuse + response types |

**No new Cargo dependencies are needed.** All required types and functions are already in scope.

## Architecture Patterns

### Recommended Project Structure

All changes are in one file: `crates/assay-mcp/src/server.rs`.

```
server.rs
├── // module doc — update tool list at top (5 new entries)
├── // ── Parameter structs ─ (existing section)
│   ├── SpecResolveParams (new — name: String only)
│   ├── CriteriaGetParams (new — slug: String only)
│   └── CriteriaListParams (none — no params; unit struct or no-params handler)
├── // ── Response structs ── (existing implied)
│   ├── GateWizardResponse (or reuse GateWizardOutput directly)
│   ├── CriteriaCreateResponse (or reuse CriteriaWizardOutput directly)
│   ├── CriteriaListResponse { entries: Vec<CriteriaListEntry>, warnings: Vec<String> }
│   ├── CriteriaListEntry { slug, criterion_count, description } (new or inline)
│   ├── CriteriaGetResponse (= CriteriaLibrary with warnings wrapper, or direct)
│   └── SpecResolveResponse { resolved: ResolvedGate, warnings: Vec<String> }
├── #[tool_router] impl AssayServer
│   ├── gate_wizard() — reuses GateWizardInput, returns GateWizardOutput + warnings
│   ├── criteria_list() — no params, returns CriteriaListResponse
│   ├── criteria_get() — CriteriaGetParams, returns CriteriaLibrary + warnings
│   ├── criteria_create() — reuses CriteriaWizardInput, returns CriteriaWizardOutput + warnings
│   └── spec_resolve() — SpecResolveParams, returns ResolvedGate + warnings
└── mod tests
    ├── gate_wizard_tool_in_router
    ├── criteria_list_tool_in_router
    ├── criteria_get_tool_in_router
    ├── criteria_create_tool_in_router
    ├── spec_resolve_tool_in_router
    └── integration tests per tool
```

### Pattern 1: Standard MCP Handler Structure

**What:** Every tool follows identical boilerplate: resolve cwd, load config, derive assay_dir/specs_dir, wrap I/O in `spawn_blocking`, match Ok/Err.

**When to use:** All five new handlers.

```rust
// Source: existing handlers in server.rs (e.g., milestone_create at line 4254)
#[tool(description = "...")]
pub async fn tool_name(
    &self,
    params: Parameters<ParamsType>,
) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = match load_config(&cwd) {
        Ok(c) => c,
        Err(err_result) => return Ok(err_result),
    };
    let assay_dir = cwd.join(".assay");
    let specs_dir = cwd.join(".assay").join(&config.specs_dir);

    let input = params.0;
    let result = tokio::task::spawn_blocking(move || {
        // call assay-core function
    })
    .await
    .map_err(|e| McpError::internal_error(format!("tool_name panicked: {e}"), None))?;

    match result {
        Ok(output) => {
            let json = serde_json::to_string(&response_struct)
                .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
        Err(e) => Ok(domain_error(&e)),
    }
}
```

### Pattern 2: Response Struct with Optional warnings

**What:** All five response structs include `warnings: Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Warnings are populated before serialization; if none apply, the field is absent from JSON.

**When to use:** All five new tool response types.

```rust
// Source: GateRunResponse (line 829), GateReportResponse (line 869), WorktreeListResponse (line 1220)
#[derive(Serialize)]
struct ToolNameResponse {
    // ... primary fields ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}
```

### Pattern 3: Reusing assay-types Input Structs as MCP Params

**What:** `GateWizardInput` and `CriteriaWizardInput` already derive `Deserialize + JsonSchema` — they are used directly as `Parameters<GateWizardInput>`. No wrapper struct needed.

**When to use:** `gate_wizard` and `criteria_create`.

```rust
// Source: GateWizardInput in assay-types/src/wizard_input.rs (line 28)
// GateWizardInput derives: Deserialize, JsonSchema, Serialize, Clone, Debug
pub async fn gate_wizard(
    &self,
    params: Parameters<GateWizardInput>,
) -> Result<CallToolResult, McpError> { ... }
```

### Pattern 4: spec_resolve Shadow Detection

**What:** After calling `compose::resolve()`, scan `resolved.criteria` for names that appear more than once across sources (by checking if the same name existed in parent/library sources). Since `resolve()` already applies own-wins dedup, shadow detection means: for each `ResolvedCriterion` with source `Own`, check if an earlier entry with the same name was eliminated (i.e., reconstruct by calling resolve mentally or detect by checking GatesSpec fields). The practical approach: compare `gate.criteria` names against parent/library slugs via the loaded spec's `extends` and `include` fields, or re-derive from the `ResolvedGate` by checking if any `Own` criterion has a name matching a shadowed `Parent`/`Library` entry.

**Simpler approach (recommended):** `resolve()` returns only surviving criteria with their final source. Shadow detection requires knowing what was discarded. The most direct path: before calling `resolve()`, load the raw parent and library criteria names, then after resolution, for each `Own` criterion, check if its name appears in parent or library names — if so, it's a shadow. This avoids re-calling resolve.

**When to use:** `spec_resolve` handler only.

### Pattern 5: No-Params Tool

**What:** `criteria_list` takes no parameters. The rmcp `#[tool]` macro supports `pub async fn criteria_list(&self) -> Result<CallToolResult, McpError>` with no `params` argument.

**When to use:** `criteria_list`.

```rust
// Source: spec_list at line 1322 — same pattern (no params)
#[tool(description = "...")]
pub async fn criteria_list(&self) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = match load_config(&cwd) { ... };
    let assay_dir = cwd.join(".assay");
    let result = tokio::task::spawn_blocking(move || {
        assay_core::spec::compose::scan_libraries(&assay_dir)
    })
    ...
}
```

### Pattern 6: Tool Registration Test

**What:** Every tool gets a `#[tokio::test] #[serial] async fn tool_name_tool_in_router()` test asserting the tool name appears in `server.tool_router.list_all()`.

**When to use:** All five new tools (per CONTEXT.md recommendation).

```rust
// Source: spec_create_tool_in_router at line 9013, milestone_create_tool_in_router at line 9001
#[tokio::test]
#[serial]
async fn gate_wizard_tool_in_router() {
    let server = AssayServer::new();
    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(tool_names.contains(&"gate_wizard"), "...");
}
```

### Anti-Patterns to Avoid

- **Inline business logic in handlers:** All validation, file I/O, and slug checking belong in `assay-core`. Handlers are thin wrappers — if you find yourself writing a loop or match over criteria, step back.
- **Missing `spawn_blocking`:** All five tools touch the filesystem. Calling sync functions directly in an async handler violates the async runtime contract. Always wrap in `spawn_blocking`.
- **Omitting `spawn_blocking` panic handling:** All `spawn_blocking` calls must `.map_err(|e| McpError::internal_error(...))` — this is the pattern for capturing task panics.
- **Using `domain_error` for protocol errors:** `domain_error` is for `AssayError` (agent can see and self-correct). Infrastructure failures (serialization, CWD resolution) use `McpError::internal_error`.
- **Adding `deny_unknown_fields` to response types:** Per Phase 65 decision, output types (responses) do NOT use `deny_unknown_fields` for forward-compatibility. Only input types use it.
- **Deriving `Deserialize` on response structs:** Response structs are serialize-only. Per Phase 67 decision: "Wizard output types derive Serialize+JsonSchema only — no Deserialize needed."

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Slug validation | Custom regex or char checking | `compose::validate_slug()` | Already battle-tested with path traversal prevention; error type is `AssayError::InvalidSlug` |
| Fuzzy slug suggestions | Levenshtein distance from scratch | `assay_core::spec::find_fuzzy_match()` (used by `load_library_by_slug`) | Built-in to `load_library_by_slug` — not-found errors already include suggestions |
| Atomic file writes | `File::create` + write | `apply_gate_wizard()` / `apply_criteria_wizard()` / `compose::save_library()` | Already uses `NamedTempFile` → `sync_all` → `persist` for crash safety |
| Criteria merging / own-wins | Custom HashMap accumulation | `compose::resolve()` | Handles parent load, library load, cycle detection, and reverse-dedup in one call |
| JSON schema generation | Manual JSON schema string | `#[derive(JsonSchema)]` on param structs | `rmcp` `#[tool_router]` macro reads schemas from derives at compile time |

**Key insight:** Every hard problem (slug validation, fuzzy match, atomic writes, merge semantics, cycle detection) is already solved in `assay-core`. The MCP handlers exist solely to translate between the MCP wire format and these core functions.

## Common Pitfalls

### Pitfall 1: spec_resolve Loading the Wrong Entry Type

**What goes wrong:** `load_spec_entry` returns a `SpecEntry` which is either `Directory { gates, .. }` or `Legacy { spec, .. }`. `compose::resolve()` takes a `&GatesSpec`. Legacy specs don't have a `GatesSpec` (they are the old flat `.toml` format). Calling resolve on a Legacy entry would require conversion, and Legacy specs have no `extends`/`include`/`preconditions` fields.

**Why it happens:** `load_spec_entry_mcp` returns `SpecEntry` — easy to unwrap the wrong variant.

**How to avoid:** For `spec_resolve`, only `SpecEntry::Directory` has `GatesSpec` with composability fields. Pattern match: if `Legacy`, return a `domain_error` explaining the spec is a legacy format and doesn't support composability resolution. If `Directory`, extract `gates` and proceed.

**Warning signs:** Compilation will fail if you try to pass a `Spec` (legacy type) to `compose::resolve()` — but the logic error is silently accepting a Legacy spec and returning an empty/wrong result.

### Pitfall 2: forgetting assay_dir vs specs_dir

**What goes wrong:** `apply_criteria_wizard(&input, assay_dir)` takes `assay_dir` (the `.assay/` root). `apply_gate_wizard(&input, _assay_dir, specs_dir)` takes both. `compose::scan_libraries(assay_dir)` and `compose::load_library_by_slug(assay_dir, slug)` take `assay_dir`. `compose::resolve()` closures need `specs_dir` for gate loading and `assay_dir` for library loading.

**Why it happens:** The naming is consistent in core but callers must know which path each function expects.

**How to avoid:** Always derive both at handler start: `let assay_dir = cwd.join(".assay")` and `let specs_dir = cwd.join(".assay").join(&config.specs_dir)`.

### Pitfall 3: Shadow Detection in spec_resolve

**What goes wrong:** `compose::resolve()` returns only the winning criteria — it doesn't report what was shadowed. Naively iterating the `ResolvedGate` result gives no shadowing information.

**Why it happens:** The dedup algorithm (reverse-dedup, own-wins) discards duplicates silently.

**How to avoid:** Before calling `compose::resolve()`, load the parent's criteria names and library criteria names independently (or inspect the loaded GatesSpec's `extends`/`include` fields). After resolution, for each `ResolvedCriterion { source: Own, criterion }`, check if `criterion.name` matches any name from the pre-loaded parent or library sets. These are the shadow warnings. This requires loading parent/library data twice if composability fields are non-empty — acceptable since it only happens when `extends` or `include` is set.

**Alternative:** Since `compose::resolve()` internally loads parent and library data via closures, a lightweight alternative is: check `gate.extends.is_some() || !gate.include.is_empty()`, then look at `resolved.criteria` — any `CriterionSource::Own` whose name would have collided. The simplest implementation: pre-collect all parent+library criterion names before calling resolve (via direct `load_spec_entry` + `load_library_by_slug` calls), then post-check the resolved Own criteria for name collisions.

### Pitfall 4: Module Doc Comment Not Updated

**What goes wrong:** The module doc comment at the top of `server.rs` (lines 1–31) lists all tools. Missing entries cause confusion for future developers and MCP server inspection.

**Why it happens:** Easy to forget as the last step.

**How to avoid:** Add five entries to the list as part of the implementation. The planner should make this an explicit task step.

### Pitfall 5: Not Using `#[serial]` on Handler Integration Tests

**What goes wrong:** Handler integration tests set `std::env::current_dir` via `std::env::set_current_dir()`. Without `#[serial]`, parallel test execution causes race conditions where tests see the wrong CWD.

**Why it happens:** All existing handler tests use `#[serial]` from `serial_test` crate — easy to forget when adding new tests.

**How to avoid:** Every `#[tokio::test]` that calls `std::env::set_current_dir` must also have `#[serial]`.

## Code Examples

Verified patterns from existing `server.rs`:

### Tool with Reused Input Type (gate_wizard pattern)
```rust
// Based on: milestone_create handler (line 4254) + spec_create handler (line 4313)
#[tool(
    description = "Create or edit a gate spec (gates.toml) from structured parameters. ..."
)]
pub async fn gate_wizard(
    &self,
    params: Parameters<GateWizardInput>,
) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = match load_config(&cwd) {
        Ok(c) => c,
        Err(err_result) => return Ok(err_result),
    };
    let assay_dir = cwd.join(".assay");
    let specs_dir = cwd.join(".assay").join(&config.specs_dir);
    let input = params.0;

    let result = tokio::task::spawn_blocking(move || {
        assay_core::wizard::apply_gate_wizard(&input, &assay_dir, &specs_dir)
    })
    .await
    .map_err(|e| McpError::internal_error(format!("gate_wizard panicked: {e}"), None))?;

    match result {
        Ok(output) => {
            // GateWizardOutput has path + spec (GatesSpec)
            // Wrap in a response struct with optional warnings
            let response = GateWizardResponse {
                path: output.path.display().to_string(),
                spec: output.spec,
                warnings: vec![],  // no warnings from apply_gate_wizard currently
            };
            let json = serde_json::to_string(&response)
                .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
        Err(e) => Ok(domain_error(&e)),
    }
}
```

### No-Params Tool (criteria_list pattern)
```rust
// Based on: spec_list handler (line 1322)
#[tool(description = "List all available criteria libraries. ...")]
pub async fn criteria_list(&self) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = match load_config(&cwd) {
        Ok(c) => c,
        Err(err_result) => return Ok(err_result),
    };
    let assay_dir = cwd.join(".assay");

    let result = tokio::task::spawn_blocking(move || {
        assay_core::spec::compose::scan_libraries(&assay_dir)
    })
    .await
    .map_err(|e| McpError::internal_error(format!("criteria_list panicked: {e}"), None))?;

    match result {
        Ok(libs) => {
            let entries: Vec<CriteriaListEntry> = libs
                .iter()
                .map(|lib| CriteriaListEntry {
                    slug: lib.name.clone(),
                    criterion_count: lib.criteria.len(),
                    description: if lib.description.is_empty() { None } else { Some(lib.description.clone()) },
                })
                .collect();
            let response = CriteriaListResponse { entries, warnings: vec![] };
            let json = serde_json::to_string(&response)
                .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
        Err(e) => Ok(domain_error(&e)),
    }
}
```

### spec_resolve Closure Construction
```rust
// compose::resolve signature (compose.rs line 243):
// pub fn resolve(gate: &GatesSpec, gate_slug: &str,
//   load_gate: impl Fn(&str) -> Result<GatesSpec>,
//   load_library: impl Fn(&str) -> Result<CriteriaLibrary>) -> Result<ResolvedGate>

// In the spawn_blocking closure:
let resolved = assay_core::spec::compose::resolve(
    &gates,
    &spec_name,
    |slug| {
        let path = specs_dir.join(slug).join("gates.toml");
        assay_core::spec::load_gates(&path)
    },
    |slug| {
        assay_core::spec::compose::load_library_by_slug(&assay_dir, slug)
    },
)?;
```

### Tool Registration Test
```rust
// Based on: spec_create_tool_in_router (line 9013)
#[tokio::test]
#[serial]
async fn gate_wizard_tool_in_router() {
    let server = AssayServer::new();
    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        tool_names.contains(&"gate_wizard"),
        "gate_wizard should be in tool list, got: {tool_names:?}"
    );
}
```

### Integration Test Structure
```rust
// Based on: spec_create_writes_gates_toml (line 9102)
#[tokio::test]
#[serial]
async fn gate_wizard_writes_gates_toml() {
    let dir = create_project(r#"project_name = "wizard-test""#);
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server
        .gate_wizard(Parameters(GateWizardInput {
            slug: "my-gate".to_string(),
            description: Some("test gate".to_string()),
            extends: None,
            include: vec![],
            criteria: vec![CriterionInput {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("cargo build".to_string()),
            }],
            preconditions: None,
            overwrite: false,
        }))
        .await
        .unwrap();

    assert!(!result.is_error.unwrap_or(false), "...");
    // Assert file on disk
    let gates_path = dir.path().join(".assay").join("specs").join("my-gate").join("gates.toml");
    assert!(gates_path.exists());
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `spec_create` for gate creation | `gate_wizard` with composability fields | Phase 68 | Agents can now set `extends`, `include`, `preconditions` |
| Manual criteria construction | `CriteriaWizardInput` → `apply_criteria_wizard` | Phase 67 | Library creation is a single MCP call |

**No deprecated approaches in this phase.** `spec_create` remains available.

## Open Questions

1. **GateWizardOutput vs wrapper response struct for gate_wizard**
   - What we know: `GateWizardOutput` has `path: PathBuf` and `spec: GatesSpec`, both derive `Serialize + JsonSchema`
   - What's unclear: Whether to serialize `GateWizardOutput` directly (PathBuf serializes as a string) or wrap in a response struct that stringifies path and adds `warnings`
   - Recommendation: Create a thin `GateWizardResponse` with `path: String` (via `display().to_string()`) and `warnings: Vec<String>` alongside `spec: GatesSpec`. This matches the pattern of other tools returning structured JSON (not raw type output), and lets warnings be included without modifying `assay-types`.

2. **Shadow detection implementation detail**
   - What we know: `compose::resolve()` does not expose what was shadowed; only the winner is in `ResolvedGate`
   - What's unclear: Whether to pre-load parent/library names before calling `resolve()` (two-pass) or post-derive from GatesSpec fields
   - Recommendation: Pre-load approach — collect parent criteria names (if `gate.extends.is_some()`) and library criteria names (for each `gate.include` slug) before calling `resolve()`, then post-check Own criteria against collected names. This is the most readable and doesn't require modifying `compose::resolve`.

3. **spec_resolve for Legacy specs**
   - What we know: `compose::resolve()` takes `&GatesSpec` (directory format only); Legacy specs have no composability fields
   - What's unclear: Whether to silently return a trivial resolution (no warnings, empty extends/includes) or return a domain error
   - Recommendation: Return a domain error with a clear message: "spec_resolve is only available for directory-format specs with composability fields". Agents can use `spec_get` for legacy specs.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`, `#[tokio::test]`) + `serial_test` |
| Config file | `Cargo.toml` workspace configuration — no separate test config |
| Quick run command | `cargo test -p assay-mcp -- --test-threads=1` |
| Full suite command | `just test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WIZM-01 | `gate_wizard` registered in tool router | unit | `cargo test -p assay-mcp gate_wizard_tool_in_router -- --test-threads=1` | ❌ Wave 0 |
| WIZM-01 | `gate_wizard` writes gates.toml to disk | integration | `cargo test -p assay-mcp gate_wizard_writes_gates_toml -- --test-threads=1` | ❌ Wave 0 |
| WIZM-01 | `gate_wizard` rejects duplicate without overwrite | integration | `cargo test -p assay-mcp gate_wizard_rejects_duplicate -- --test-threads=1` | ❌ Wave 0 |
| WIZM-02 | `criteria_list` registered in tool router | unit | `cargo test -p assay-mcp criteria_list_tool_in_router -- --test-threads=1` | ❌ Wave 0 |
| WIZM-02 | `criteria_list` returns empty list when no libraries exist | integration | `cargo test -p assay-mcp criteria_list_empty_project -- --test-threads=1` | ❌ Wave 0 |
| WIZM-02 | `criteria_get` registered in tool router | unit | `cargo test -p assay-mcp criteria_get_tool_in_router -- --test-threads=1` | ❌ Wave 0 |
| WIZM-02 | `criteria_get` returns full library for valid slug | integration | `cargo test -p assay-mcp criteria_get_returns_library -- --test-threads=1` | ❌ Wave 0 |
| WIZM-02 | `criteria_get` returns domain error for invalid slug | integration | `cargo test -p assay-mcp criteria_get_not_found -- --test-threads=1` | ❌ Wave 0 |
| CLIB-04 | `criteria_create` registered in tool router | unit | `cargo test -p assay-mcp criteria_create_tool_in_router -- --test-threads=1` | ❌ Wave 0 |
| CLIB-04 | `criteria_create` writes library file to disk | integration | `cargo test -p assay-mcp criteria_create_writes_library -- --test-threads=1` | ❌ Wave 0 |
| CLIB-04 | `criteria_create` rejects duplicate without overwrite | integration | `cargo test -p assay-mcp criteria_create_rejects_duplicate -- --test-threads=1` | ❌ Wave 0 |
| WIZM-03 | `spec_resolve` registered in tool router | unit | `cargo test -p assay-mcp spec_resolve_tool_in_router -- --test-threads=1` | ❌ Wave 0 |
| WIZM-03 | `spec_resolve` returns ResolvedGate for valid spec | integration | `cargo test -p assay-mcp spec_resolve_returns_resolved_gate -- --test-threads=1` | ❌ Wave 0 |
| WIZM-03 | `spec_resolve` returns domain error for missing spec | integration | `cargo test -p assay-mcp spec_resolve_not_found -- --test-threads=1` | ❌ Wave 0 |
| WIZM-03 | `spec_resolve` surfaces shadow warnings | integration | `cargo test -p assay-mcp spec_resolve_shadow_warnings -- --test-threads=1` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-mcp -- --test-threads=1`
- **Per wave merge:** `just test`
- **Phase gate:** Full suite green before `/kata:verify-work`

### Wave 0 Gaps
All test functions listed above are new — they will be added inline to `server.rs`'s `mod tests` block as part of implementation. No separate test files are created (all tests live in the single `server.rs` file per convention). No additional framework install needed — `serial_test` and `tempfile` are already workspace dev-dependencies.

- [ ] All test functions in the Phase Requirements → Test Map above — added to `mod tests` in `server.rs`

## Sources

### Primary (HIGH confidence)
- `crates/assay-mcp/src/server.rs` — read directly; all patterns verified against actual code
- `crates/assay-types/src/wizard_input.rs` — read directly; `GateWizardInput`, `CriteriaWizardInput`, output types confirmed
- `crates/assay-core/src/spec/compose.rs` — read directly; `scan_libraries`, `load_library_by_slug`, `resolve` signatures confirmed
- `crates/assay-core/src/wizard/gate.rs` — read directly; `apply_gate_wizard` signature confirmed
- `crates/assay-core/src/wizard/criteria.rs` — read directly; `apply_criteria_wizard` signature confirmed
- `crates/assay-types/src/resolved_gate.rs` — read directly; `ResolvedGate`, `ResolvedCriterion`, `CriterionSource` confirmed
- `crates/assay-types/src/criteria_library.rs` — read directly; `CriteriaLibrary` fields confirmed
- `.planning/phases/68-mcp-surface/68-CONTEXT.md` — read directly; locked decisions and discretion areas

### Secondary (MEDIUM confidence)
None — all findings verified from primary source code.

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all types, functions, and patterns verified by reading the actual source
- Architecture: HIGH — established patterns extracted from existing handlers in the same file
- Pitfalls: HIGH (Pitfall 1–3) / MEDIUM (Pitfall 4–5) — code-derived except the doc comment reminder

**Research date:** 2026-04-12
**Valid until:** 2026-05-12 (stable internal codebase — no external dependencies change)
