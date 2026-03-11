# Phase 37 Verification — Spec Validation

**Status: passed**

All must-haves from Plan 01 and Plan 02 are present in the actual source code. `just ready` passes with 660 tests, 0 failures.

---

## Plan 01 Must-Haves

- [x] **ValidationResult, Diagnostic, Severity types exist in assay-types with serde + schemars derives**
  - `crates/assay-types/src/validation.rs` defines all four types (`ValidationResult`, `Diagnostic`, `Severity`, `DiagnosticSummary`) with `#[derive(Serialize, Deserialize, JsonSchema)]`.

- [x] **Severity has three levels (error, warning, info) where errors block validity**
  - `Severity::Error`, `Severity::Warning`, `Severity::Info` defined. `ValidationResult.valid` is set to `summary.errors == 0` in `build_summary`.

- [x] **Spec and GatesSpec both have an optional `depends` field (Vec<String>)**
  - `Spec.depends: Vec<String>` in `crates/assay-types/src/lib.rs` (line 66).
  - `GatesSpec.depends: Vec<String>` in `crates/assay-types/src/gates_spec.rs` (line 40).
  - Both use `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.

- [x] **spec::validate module converts existing SpecError to Diagnostic without duplicating validation logic**
  - `crates/assay-core/src/spec/validate.rs` calls `super::validate()` and `super::validate_gates_spec()` and converts errors via `spec_errors_to_diagnostics()`. No logic is duplicated.

- [x] **AgentReport criteria without `prompt` field produce a warning diagnostic**
  - `validate_agent_prompts()` in `validate.rs` emits `Severity::Warning` for `kind=AgentReport` criteria with missing or whitespace-only `prompt`. Tests cover: missing prompt, whitespace-only prompt, valid prompt (no warn), non-AgentReport ignored.

- [x] **check_commands validates command binary existence on PATH using `which` crate**
  - `validate_commands()` calls `which::which(binary)` and emits `Severity::Warning` for not-found binaries. The `which = "7"` workspace dep is in root `Cargo.toml`; `which.workspace = true` in `crates/assay-core/Cargo.toml`.

- [x] **Cycle detection uses DFS with three-color marking and reports full cycle path**
  - `detect_cycles()` uses `Color::White/Gray/Black` enum for DFS marking. Cycle messages use format `"circular dependency detected: a -> b -> a"`. Tests cover 2-node cycle, 3-node cycle, no cycle, unknown dependency.

- [x] **`which` crate is added to workspace dependencies**
  - `which = "7"` present in root `Cargo.toml` (line 40). `which.workspace = true` in `crates/assay-core/Cargo.toml` (line 23).

---

## Plan 02 Must-Haves

- [x] **spec_validate MCP tool exists and is registered in the tool router**
  - `pub async fn spec_validate(...)` decorated with `#[tool(...)]` at line 625 of `crates/assay-mcp/src/server.rs`. The struct uses `#[tool_router]` for automatic registration.

- [x] **Tool accepts `name` (required), `check_commands` (optional, default false) parameters**
  - `SpecValidateParams` has `name: String` (required) and `check_commands: bool` with `#[serde(default)]` (defaults to false).

- [x] **Tool returns structured JSON matching ValidationResult schema**
  - Successful path returns `Content::json(result)?` where `result` is a `ValidationResult`. Error paths also return `ValidationResult`-shaped JSON.

- [x] **Tool reuses validate_spec_with_dependencies from assay-core**
  - Line 710: `assay_core::spec::validate::validate_spec_with_dependencies(&entry, params.0.check_commands, &specs_dir)`.

- [x] **Server module doc comment updated from twelve to thirteen tools**
  - Line 3 of `server.rs`: `"Provides the AssayServer which exposes thirteen tools over MCP"` and `spec_validate` is listed in the tool inventory comment.

- [x] **Invalid TOML produces error-severity diagnostics (via raw TOML parse before SpecEntry loading)**
  - `spec_validate` handler catches `AssayError::SpecParse` and `AssayError::GatesSpecParse` and wraps them as `Severity::Error` diagnostics at `location: "toml"`.

- [x] **TOML parse errors use format_toml_error for enriched messages**
  - `load_spec_entry` (called by `load_spec_entry_with_diagnostics`) uses `crate::config::format_toml_error(&content, &e)` for both legacy specs (line 327) and gates specs (line 350), including line number and caret display.

- [x] **Tool handles both legacy and directory-based specs**
  - `validate_spec()` in `validate.rs` matches on both `SpecEntry::Legacy` and `SpecEntry::Directory` variants. The server's error handling covers both `SpecParse`/`SpecValidation` (legacy) and `GatesSpecParse`/`GatesSpecValidation` (directory).

---

## Test Coverage

- `crates/assay-core/src/spec/validate.rs`: 12 unit tests covering all major code paths (agent prompt checks, command validation, cycle detection, full validate_spec integration).
- `crates/assay-types/src/gates_spec.rs`: roundtrip tests verify `depends` field serializes/deserializes correctly.
- `crates/assay-core/src/spec/mod.rs`: tests for `load_spec_entry_with_diagnostics`, duplicate criterion names, TOML parse errors with enriched messages.
- No MCP-level integration tests for `spec_validate` specifically, but this was not listed as a must-have.

---

## Build & Test Results

```
just ready → All checks passed (660 tests, 0 failures, 0 warnings in code)
```

---

## Gaps

None. All must-haves are implemented and tested.
