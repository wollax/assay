# Phase 37: Spec Validation - Research

**Researched:** 2026-03-11
**Confidence:** HIGH (codebase-verified patterns, well-established libraries)

## Standard Stack

| Need | Use | Version | Confidence |
|------|-----|---------|------------|
| TOML parsing | `toml` | 0.8 (already in workspace) | HIGH |
| Cycle detection | Hand-rolled DFS | N/A | HIGH |
| Command PATH lookup | `which` crate | latest | HIGH |
| Serialization | `serde` + `serde_json` | workspace | HIGH |
| Schema generation | `schemars` | workspace | HIGH |
| MCP server | `rmcp` | 0.17 (workspace) | HIGH |

### Why NOT petgraph for cycle detection

petgraph (HIGH confidence, well-documented) provides `is_cyclic_directed` and `toposort` with `Cycle<NodeId>` error reporting. However, the spec dependency graph is trivially small (tens of nodes at most), and we need the **full cycle path** in the error message (e.g., "A -> B -> C -> A"). petgraph's `toposort` only returns a single `Cycle(node_id)` -- not the full path. Extracting the full path requires either:
- Custom DFS visitor callbacks (still hand-rolled logic)
- Building an SCC and post-processing

A hand-rolled DFS with explicit path tracking is simpler, zero dependencies, and gives us the exact diagnostic we need. The graph is built from `scan()` results -- at most a few dozen specs.

### Why the `which` crate

The `which` crate is the de facto Rust standard for cross-platform executable lookup on PATH. It handles:
- Unix: searches PATH, checks execute permissions
- Windows: searches PATH with PATHEXT extensions (.exe, .cmd, etc.)
- Returns `which::Error` on not-found

Add to workspace `Cargo.toml`: `which = "7"` (or latest). Only `assay-core` needs the dependency (behind the `check_commands` feature or unconditionally -- it's tiny).

## Architecture Patterns

### Where code lives

Follow existing crate boundaries exactly:

| Component | Crate | File |
|-----------|-------|------|
| `ValidationResult`, `Diagnostic`, `Severity` types | `assay-types` | `src/validation.rs` (new) |
| `spec_validate()` domain logic | `assay-core` | `src/spec/validate.rs` (new submodule) or extend `src/spec/mod.rs` |
| `spec_validate` MCP tool handler | `assay-mcp` | `src/server.rs` (extend existing) |

### ValidationResult type (assay-types)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResult {
    pub valid: bool,           // false when any error-severity diagnostic exists
    pub spec_name: String,     // which spec was validated
    pub diagnostics: Vec<Diagnostic>,
    pub summary: ValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,          // e.g., "E001", "W001", "I001"
    pub message: String,
    pub field: Option<String>, // structural path like "criteria[2].name"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}
```

### MCP tool handler pattern

Follow the exact pattern of existing tools in `server.rs`:

1. Define `SpecValidateParams` struct with `Deserialize + JsonSchema`
2. Add `#[tool(description = "...")]` method on `AssayServer`
3. Load config via `load_config()`, resolve specs dir
4. Call domain logic in `assay-core`
5. Serialize `ValidationResult` to JSON
6. Return `CallToolResult::success(vec![Content::text(json)])`
7. Domain errors become `CallToolResult::error` via `domain_error()`

```rust
#[derive(Deserialize, JsonSchema)]
pub struct SpecValidateParams {
    /// Spec name to validate.
    #[schemars(description = "Spec name (filename without .toml, e.g. 'auth-flow')")]
    pub name: String,

    /// Validate command existence on PATH (default: false).
    #[schemars(description = "Check that commands referenced in criteria exist on PATH (default: false)")]
    #[serde(default)]
    pub check_commands: bool,

    /// Treat warnings as errors (default: false).
    #[schemars(description = "Strict mode: promote warnings to errors (default: false)")]
    #[serde(default)]
    pub strict: bool,
}
```

### Validation logic architecture

The domain function in `assay-core` should:

1. **Read raw TOML string** (not use `load()` which already validates and returns early on parse errors)
2. **Try TOML parse** -- on failure, return a single `Diagnostic` with the TOML error (including line/column from `toml::de::Error`)
3. **If parse succeeds**, run semantic checks collecting all diagnostics:
   - Criterion name uniqueness (ERROR)
   - AgentReport prompt presence (ERROR)
   - AgentReport + cmd/path mutual exclusion (ERROR)
   - Empty name (ERROR)
   - Empty criteria list (severity: Claude's discretion)
   - cmd + path both set (WARNING)
   - Unrecognized fields -- note: `#[serde(deny_unknown_fields)]` already rejects these at parse time, so this is handled by step 2
4. **If `check_commands`**, validate each criterion's `cmd` field first token against PATH
5. **Apply strict mode** -- if `strict`, promote all warnings to errors
6. **Compute `valid`** = no error-severity diagnostics remain

### Cross-spec dependency validation (SPEC-04)

The `Spec` and `GatesSpec` types do **not** currently have a `depends` field. This needs to be added:

```rust
// In assay-types Spec and/or GatesSpec:
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub depends: Vec<String>,
```

Cycle detection algorithm:
1. Build adjacency list from all specs' `depends` fields (via `scan()`)
2. Run DFS with three-color marking (white/gray/black)
3. When a gray node is re-encountered, extract the cycle path from the DFS stack
4. Return the full path in the diagnostic: "Dependency cycle: A -> B -> C -> A"

This should be a separate function that takes the full `ScanResult` (or a map of spec-name -> depends), not a single-spec operation. However, it can be invoked from `spec_validate` for a single spec by loading all specs, checking if the validated spec participates in any cycle.

### Reuse of existing validation

The existing `validate()`, `validate_gates_spec()`, and `validate_feature_spec()` functions already check:
- Empty name
- Empty criteria
- Duplicate criterion names
- cmd/path mutual exclusion
- AgentReport + cmd/path incompatibility
- At least one required executable criterion

The new `spec_validate` tool should **reuse** these functions by calling them and converting their `Vec<SpecError>` results into `Vec<Diagnostic>` with appropriate severity levels. Do NOT duplicate the validation logic.

Approach: call existing `validate()` / `validate_gates_spec()` to get `Vec<SpecError>`, then map each to a `Diagnostic`. Add **new** checks on top:
- AgentReport missing prompt (not currently checked -- existing validation only checks cmd/path incompatibility)
- Command existence on PATH (new, behind `check_commands`)
- Cross-spec dependency cycles (new)
- Unrecognized outcome values in future fields

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| PATH lookup / executable detection | `which` crate |
| TOML parsing / error messages | `toml` crate's `de::Error` (already used) |
| JSON serialization | `serde_json` (already used) |
| Schema generation for MCP | `schemars` derive (already used) |
| TOML error formatting with line/column | `crate::config::format_toml_error()` (already exists in assay-core) |

## Common Pitfalls

### 1. Duplicating validation logic
The existing `validate()` / `validate_gates_spec()` / `validate_feature_spec()` functions already check many of the same things SPEC-02 requires. The new tool must wrap and reuse them, not reimplement. Converting `SpecError` -> `Diagnostic` is the bridge.

### 2. Early return on first error
The existing `load()` function returns early on TOML parse failure (never reaching semantic validation). The `spec_validate` tool must handle this differently: a TOML parse error should produce a diagnostic and return (since semantic validation cannot proceed on unparseable input), but this is a design choice, not a bug.

### 3. `deny_unknown_fields` handling
All spec types use `#[serde(deny_unknown_fields)]`. This means unrecognized fields already cause TOML parse failures. If the CONTEXT.md says "unrecognized fields are warnings", we'd need to parse *without* `deny_unknown_fields` first, detect extras, then parse with it. **Alternative**: accept that `deny_unknown_fields` makes extras an error at parse time (which is more correct) and document that unrecognized fields are parse errors. This is simpler and consistent with existing behavior.

**Recommendation**: Keep `deny_unknown_fields` behavior (parse errors for unknown fields). The CONTEXT.md says "unrecognized/extra fields in spec TOML are warnings -- catches typos, blocks in strict mode." To implement this as warnings, we'd need a two-pass parse: first with a permissive struct (no `deny_unknown_fields`), then compare keys. This is complex. Given Claude has discretion here, recommend treating them as errors (existing behavior) since `deny_unknown_fields` is already enforced everywhere.

### 4. Cycle detection must handle missing dependencies gracefully
A spec may declare `depends = ["nonexistent-spec"]`. This should produce its own diagnostic (warning or error per discretion) before/separate from cycle detection. The cycle detection should only consider edges where both endpoints exist.

### 5. Command checking platform differences
On Unix, `which` checks execute permission. On Windows, it checks PATHEXT. The `which` crate handles both. Do not hand-roll `std::fs::metadata` + permission checks.

### 6. Command extraction from criterion `cmd` field
Commands like `cargo test --workspace -- -D warnings` need only the first token (`cargo`) checked against PATH. Use `cmd.split_whitespace().next()` -- this already exists as `extract_binary()` in `gate/mod.rs`.

### 7. Schema registry
All new types in `assay-types` must submit to the schema registry via `inventory::submit!` -- this is the established pattern.

### 8. MCP tool count in module doc
The server module doc comment lists tool count ("twelve tools"). This must be updated when adding `spec_validate`.

## Code Examples

### Converting existing SpecError to Diagnostic

```rust
fn spec_errors_to_diagnostics(errors: Vec<SpecError>) -> Vec<Diagnostic> {
    errors.into_iter().map(|e| Diagnostic {
        severity: Severity::Error,
        code: classify_spec_error(&e),  // map field patterns to codes
        message: e.message,
        field: Some(e.field),
    }).collect()
}
```

### Cycle detection with path reporting

```rust
fn detect_cycles(deps: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();  // permanently done
    let mut on_stack = HashSet::new(); // currently in DFS path
    let mut path = Vec::new();         // current DFS path

    for node in deps.keys() {
        if !visited.contains(node.as_str()) {
            dfs(node, deps, &mut visited, &mut on_stack, &mut path, &mut cycles);
        }
    }
    cycles
}

fn dfs(
    node: &str,
    deps: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    on_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    on_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(neighbors) = deps.get(node) {
        for neighbor in neighbors {
            if on_stack.contains(neighbor.as_str()) {
                // Found cycle -- extract from path
                let start = path.iter().position(|n| n == neighbor).unwrap();
                let mut cycle: Vec<String> = path[start..].to_vec();
                cycle.push(neighbor.clone()); // close the cycle
                cycles.push(cycle);
            } else if !visited.contains(neighbor.as_str()) {
                dfs(neighbor, deps, visited, on_stack, path, cycles);
            }
        }
    }

    path.pop();
    on_stack.remove(node);
}
```

### Command existence check

```rust
fn check_command_exists(cmd_str: &str) -> Option<Diagnostic> {
    let binary = cmd_str.split_whitespace().next()?;
    match which::which(binary) {
        Ok(_) => None,
        Err(_) => Some(Diagnostic {
            severity: Severity::Warning, // or Error -- Claude's discretion
            code: "W010".to_string(),
            message: format!("command '{}' not found on PATH", binary),
            field: None, // set by caller with criterion index
        }),
    }
}
```

### AgentReport prompt validation (not currently checked)

```rust
// Add to validation: AgentReport criteria must have a prompt
if criterion.kind == Some(CriterionKind::AgentReport) && criterion.prompt.is_none() {
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        code: "E005".to_string(),
        message: format!(
            "criterion '{}' has kind=AgentReport but no prompt field",
            criterion.name
        ),
        field: Some(format!("criteria[{}].prompt", i)),
    });
}
```

## Key Codebase References

| What | Where |
|------|-------|
| Spec type (legacy flat) | `crates/assay-types/src/lib.rs` lines 47-64 |
| GatesSpec type (directory) | `crates/assay-types/src/gates_spec.rs` |
| Criterion type | `crates/assay-types/src/criterion.rs` |
| FeatureSpec type | `crates/assay-types/src/feature_spec.rs` |
| Existing validate() | `crates/assay-core/src/spec/mod.rs` ~line 202 |
| validate_criteria() | `crates/assay-core/src/spec/mod.rs` ~line 235 |
| validate_gates_spec() | `crates/assay-core/src/spec/mod.rs` ~line 546 |
| validate_feature_spec() | `crates/assay-core/src/spec/mod.rs` ~line 487 |
| scan() for all specs | `crates/assay-core/src/spec/mod.rs` ~line 622 |
| extract_binary() | `crates/assay-core/src/gate/mod.rs` ~line 48 |
| format_toml_error() | `crates/assay-core/src/config/mod.rs` |
| MCP server + tool pattern | `crates/assay-mcp/src/server.rs` |
| domain_error() helper | `crates/assay-mcp/src/server.rs` ~line 1315 |
| AssayError variants | `crates/assay-core/src/error.rs` |
| SpecError type | `crates/assay-core/src/spec/mod.rs` ~line 127 |
| Workspace deps | `Cargo.toml` (root) |

## Open Questions (for planner)

1. **`depends` field location**: Should `depends` go on `Spec` (legacy), `GatesSpec` (directory), or both? Recommendation: both, for consistency. But if SPEC-04 only applies to directory-based specs, it could go on `GatesSpec` only.

2. **Scope of SPEC-04**: The validate tool gets a single spec name. For cycle detection, it needs to load ALL specs to build the dependency graph. This means `spec_validate` with cycle detection is heavier than a single-spec check. Consider: always do cycle detection when the spec has `depends`, skip when it doesn't.

3. **Strict mode default**: CONTEXT.md says "default allows warnings without blocking; `strict: true` promotes warnings to errors." This is clear -- `strict` defaults to `false`.

---

*Phase: 37-spec-validation*
*Research completed: 2026-03-11*
