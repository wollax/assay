# Phase 37 Plan 02: MCP Tool Wiring Summary

**Wire spec_validate into the MCP server as a structured validation tool**

## Frontmatter

- **Phase:** 37-spec-validation
- **Plan:** 02
- **Subsystem:** mcp
- **Tags:** mcp, tool-wiring, validation, error-handling
- **Completed:** 2026-03-11
- **Duration:** ~8 minutes

### Dependency Graph

- **Requires:** Plan 01 (ValidationResult types, validate_spec_with_dependencies function)
- **Provides:** spec_validate MCP tool accessible to agents
- **Affects:** None (terminal plan in phase 37)

### Key Files

- **Modified:**
  - `crates/assay-mcp/src/server.rs` — Added SpecValidateParams struct and spec_validate tool handler

## Task Completion

### Task 1: spec_validate MCP Tool

| Item | Status |
|------|--------|
| Module doc comment updated (twelve -> thirteen) | Done |
| spec_validate added to tool list in doc comment | Done |
| SpecValidateParams struct with name + check_commands | Done |
| check_commands defaults to false via #[serde(default)] | Done |
| Tool handler follows existing Parameters<T> pattern | Done |
| TOML parse errors caught and returned as ValidationResult | Done |
| SpecValidation/GatesSpecValidation errors caught and converted | Done |
| SpecNotFound errors use existing domain_error pattern | Done |
| Valid specs delegated to validate_spec_with_dependencies | Done |
| Cargo.lock updated with `which` dependency from Plan 01 | Done |
| `just ready` passes (fmt, clippy, 660 tests, cargo-deny) | Done |

**Commit:** `78ac1a4` — `feat(37-02): wire spec_validate MCP tool`

## Deviations

### Deviation 1: Plan used non-existent API patterns (auto-fixed)

The plan referenced `#[tool(aggr)]` parameter annotation and `self.specs_dir()` method, neither of which exist in the codebase. Adapted to use the actual patterns: `Parameters<T>` wrapper and `resolve_cwd()` + `load_config()` + manual specs_dir construction, matching all existing tool handlers.

### Deviation 2: Cargo.lock included in commit (auto-fixed)

Plan 01 added the `which` workspace dependency but did not commit Cargo.lock. Included it in this commit since the lockfile is a tracked build artifact.

## Decisions

- Used fully qualified paths (`assay_types::ValidationResult`, etc.) in the handler rather than adding imports, keeping the use statement minimal and matching existing style where other handlers use `Content::text(serde_json::to_string(...))`.
- The handler catches `FeatureSpecParse`/`FeatureSpecValidation` errors implicitly via the `Err(other)` fallthrough to `domain_error()`, since those are less common and don't need structured ValidationResult conversion.

## Verification

- `cargo check -p assay-mcp` — compiles without errors
- `cargo clippy --workspace -- -D warnings` — no warnings
- `cargo test --workspace` — 660 passed, 3 ignored
- `just ready` — all checks passed (fmt-check, lint, test, deny)
