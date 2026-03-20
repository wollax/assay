# S03: Guided Authoring Wizard

**Goal:** `assay plan` generates a valid milestone TOML + `gates.toml` per chunk from interactive prompts; `milestone_create` and `spec_create` MCP tools expose the same logic programmatically so agent callers (Claude Code, Codex) can drive authoring without a TTY.
**Demo:** Run `create_from_inputs()` with a `WizardInputs` struct in a test, confirm a `.assay/milestones/<slug>.toml` file appears and two `.assay/specs/<chunk>/gates.toml` files appear; load each with the existing `milestone_load` / `assay gate run` path and confirm they parse cleanly. Then call the MCP `milestone_create` tool against a temp project dir and verify it returns the new milestone slug; call `spec_create` and confirm `gates.toml` appears and the milestone's `chunks` list is updated.

## Must-Haves

- `assay-core::wizard` module: `WizardInputs`, `ChunkInput`, `CriterionInput`, `WizardResult`, `slugify()`, `create_from_inputs()`, `create_milestone_from_params()`, `create_spec_from_params()`
- `create_from_inputs()` writes atomic milestone TOML via `milestone_save()` and atomic `gates.toml` per chunk; sets `milestone` + `order` on each `GatesSpec`
- `create_spec_from_params()` rejects duplicate specs (idempotency guard) and patches the milestone's `chunks` Vec when `milestone_slug` is provided
- `slugify()` lowercases, replaces non-alphanumeric runs with `-`, trims hyphens; result validated via `validate_path_component`
- Slug uniqueness enforced in `create_from_inputs()`: returns error if milestone already exists
- `dialoguer` wired into workspace and `assay-cli`; `assay plan` checks `stdin().is_terminal()` before calling any dialoguer prompt
- `assay plan` non-TTY path exits with a user-friendly message pointing to `milestone_create` MCP tool
- MCP `milestone_create` tool registered in router with `chunks: Vec<ChunkParams>` parameter
- MCP `spec_create` tool registered in router; optionally patches milestone when `milestone_slug` provided
- Integration tests in `crates/assay-core/tests/wizard.rs` cover: round-trip (files created + parseable), slug collision rejection, non-existent milestone error in `create_spec_from_params`
- MCP tool tests: `milestone_create_tool_in_router`, `spec_create_tool_in_router`, `milestone_create_writes_milestone_toml`, `spec_create_writes_gates_toml`, `spec_create_rejects_duplicate`
- `just ready` green; no regression in existing 1308+ tests

## Proof Level

- This slice proves: integration
- Real runtime required: no (wizard core + MCP paths are fully automated; TTY path is UAT only)
- Human/UAT required: yes — manual `assay plan` invocation in a real terminal to verify dialoguer prompts render correctly and generated files pass `assay gate run`

## Verification

```
# Wizard core integration tests
cargo test -p assay-core --features assay-types/orchestrate --test wizard

# MCP tool tests
cargo test -p assay-mcp -- milestone_create
cargo test -p assay-mcp -- spec_create

# CLI compilation + non-TTY guard test
cargo test -p assay-cli -- plan

# Full workspace green
cargo test --workspace
just ready
```

## Observability / Diagnostics

- Runtime signals: `create_from_inputs()` returns `WizardResult { milestone_path, spec_paths: Vec<PathBuf> }` on success — caller can print each created path; `domain_error(&e)` returns `isError: true` with `AssayError::Io` message including file path on any failure
- Inspection surfaces: `assay milestone list` (from S01) shows the generated milestone; `assay spec list` shows the generated chunk specs; `assay gate run <chunk>` validates the generated `gates.toml`
- Failure visibility: slug collision returns `AssayError::Io` with `"milestone '<slug>' already exists"` message; non-TTY returns exit code 1 with `"assay plan requires an interactive terminal. Use the milestone_create MCP tool instead."`; `spec_create` duplicate returns `domain_error` with `"spec directory '<slug>' already exists"`
- Redaction constraints: none (wizard inputs are user-provided spec names, no secrets)

## Integration Closure

- Upstream surfaces consumed: `assay_core::milestone::{milestone_load, milestone_save}` (S01), `assay_core::history::validate_path_component` (S01), `assay_types::{Milestone, ChunkRef, GatesSpec, MilestoneStatus}` (S01), `spec.rs` `gates.toml` template format (pre-existing)
- New wiring introduced in this slice: `pub mod wizard` in `assay-core/src/lib.rs`; `plan` variant + dispatch in `assay-cli/src/main.rs`; `milestone_create` + `spec_create` `#[tool]` methods on `AssayServer`; `dialoguer` in workspace deps
- What remains before the milestone is truly usable end-to-end: S04 (gate-gated PR creation), S05 (Claude Code plugin skills that call `milestone_create` + `spec_create`), S06 (Codex plugin)

## Tasks

- [ ] **T01: Write failing wizard core + MCP integration tests** `est:45m`
  - Why: Defines the wizard API contract before implementation; test-first ensures the implementation covers all required behaviors; failing tests are the objective stopping condition
  - Files: `crates/assay-core/tests/wizard.rs`, `crates/assay-mcp/src/server.rs` (test section only)
  - Do: Write `crates/assay-core/tests/wizard.rs` with 5 tests: `wizard_create_from_inputs_writes_files`, `wizard_create_from_inputs_sets_milestone_and_order_on_specs`, `wizard_slug_collision_returns_error`, `wizard_create_spec_patches_milestone`, `wizard_create_spec_rejects_nonexistent_milestone`. Use `tempfile::TempDir` for isolation. Import `assay_core::wizard::{create_from_inputs, create_spec_from_params, WizardInputs, ChunkInput, CriterionInput}` (these don't exist yet — tests will fail to compile). In `server.rs` test section, add 5 tests: `milestone_create_tool_in_router`, `spec_create_tool_in_router`, `milestone_create_writes_milestone_toml`, `spec_create_writes_gates_toml`, `spec_create_rejects_duplicate`. All tests must fail (compile errors or test failures) after this task — the module doesn't exist yet.
  - Verify: `cargo test -p assay-core --features assay-types/orchestrate --test wizard 2>&1 | grep "error\[E"` shows compile errors; `cargo test -p assay-mcp -- milestone_create 2>&1 | grep "error\[E\|FAILED"` shows failures
  - Done when: Test files exist with all required test functions; both test targets fail to compile due to missing `wizard` module

- [ ] **T02: Implement `assay-core::wizard` module** `est:60m`
  - Why: The pure-function wizard core is the heart of S03 — both the CLI and MCP tools are thin wrappers over it; implementing it makes T01's wizard tests pass
  - Files: `crates/assay-core/src/wizard.rs`, `crates/assay-core/src/lib.rs`
  - Do: Create `crates/assay-core/src/wizard.rs`. Define: `CriterionInput { name: String, description: String, cmd: Option<String> }`, `ChunkInput { name: String, criteria: Vec<CriterionInput> }` (slug auto-derived via `slugify(name)`), `WizardInputs { name: String, description: Option<String>, chunks: Vec<ChunkInput> }` (milestone slug auto-derived via `slugify(name)`), `WizardResult { milestone_path: PathBuf, spec_paths: Vec<PathBuf> }`. Implement `pub fn slugify(s: &str) -> String` (lowercase, `[^a-z0-9]+` → `-`, trim hyphens). Implement `pub fn create_from_inputs(inputs: &WizardInputs, assay_dir: &Path, specs_dir: &Path) -> Result<WizardResult>`: call `validate_path_component` on derived slug, check milestone doesn't already exist (return `AssayError::Io` if collision), build `Milestone` with `Utc::now()` for both timestamps and `MilestoneStatus::Draft`, build one `ChunkRef` per chunk, call `milestone_save()`, then per chunk build `GatesSpec` with `milestone: Some(slug)` and `order: Some(i as u32)`, use `toml::to_string_pretty()`, write `gates.toml` atomically via `NamedTempFile`. Implement `pub fn create_milestone_from_params(slug: &str, name: &str, description: Option<&str>, chunks: Vec<(String, u32)>, assay_dir: &Path) -> Result<Milestone>` (creates milestone TOML only). Implement `pub fn create_spec_from_params(slug: &str, name: &str, milestone_slug: Option<&str>, order: Option<u32>, criteria: Vec<CriterionInput>, specs_dir: &Path, assay_dir: &Path) -> Result<PathBuf>` (rejects existing spec dir, writes `gates.toml`, optionally patches milestone). Add `pub mod wizard;` to `lib.rs`.
  - Verify: `cargo test -p assay-core --features assay-types/orchestrate --test wizard` — all 5 tests pass; `cargo test -p assay-core --features assay-types/orchestrate` (all existing core tests still pass)
  - Done when: All 5 wizard integration tests pass; `cargo test -p assay-core --features assay-types/orchestrate` is green

- [ ] **T03: Add `assay plan` CLI command with dialoguer** `est:45m`
  - Why: `assay plan` is the primary human-facing entry point for the wizard; TTY guard prevents hangs in non-interactive environments
  - Files: `Cargo.toml` (workspace), `crates/assay-cli/Cargo.toml`, `crates/assay-cli/src/commands/plan.rs`, `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/main.rs`
  - Do: Add `dialoguer = "0.12.0"` to `[workspace.dependencies]` in root `Cargo.toml`. Add `dialoguer.workspace = true` to `[dependencies]` in `crates/assay-cli/Cargo.toml`. Create `crates/assay-cli/src/commands/plan.rs` with `pub(crate) fn handle() -> anyhow::Result<i32>`. At the top of `handle()`, call `if !std::io::stdin().is_terminal() { eprintln!("assay plan requires an interactive terminal. Use the milestone_create MCP tool for non-interactive authoring."); return Ok(1); }`. Then use `dialoguer::Input` to collect milestone name, optional description, chunk count (1–7 via `dialoguer::Select` with options "1".."7"), per-chunk name + description + criterion name/description/cmd (repeat until user declines). Build `WizardInputs`, call `assay_core::wizard::create_from_inputs(&inputs, &assay_dir, &specs_dir)`, print created file paths with `println!("  Created milestone '{slug}'")` + `println!("    created {}", path.display())` per file. Add `pub mod plan;` to `commands/mod.rs`. Add `Plan` variant (no sub-commands) to the `Command` enum in `main.rs` with doc comment `"Run the guided authoring wizard to create a milestone + chunk specs"`. Add dispatch arm `Some(Command::Plan) => commands::plan::handle()`. Add a `#[test] fn plan_command_non_tty() { ... }` in `plan.rs` that calls `handle()` only if `!std::io::stdin().is_terminal()` — since tests run non-interactively, verify it returns `Ok(1)`.
  - Verify: `cargo test -p assay-cli -- plan` — `plan_command_non_tty` passes; `cargo build -p assay-cli` compiles
  - Done when: `cargo test -p assay-cli -- plan` passes; `cargo build -p assay-cli` green; `just lint` passes

- [ ] **T04: Implement `milestone_create` and `spec_create` MCP tools** `est:45m`
  - Why: MCP tools are the programmatic entry point for agent callers; closes S03's requirement coverage for R042 and makes T01's MCP tests pass
  - Files: `crates/assay-mcp/src/server.rs`
  - Do: Add four structs with `#[derive(Deserialize, JsonSchema)]`: `ChunkParams { slug: String, name: String, order: u32 }`, `CriterionParams { name: String, description: String, cmd: Option<String> }`, `MilestoneCreateParams { slug: String, name: String, description: Option<String>, chunks: Vec<ChunkParams> }`, `SpecCreateParams { slug: String, name: String, milestone_slug: Option<String>, order: Option<u32>, criteria: Vec<CriterionParams> }`. Add `milestone_create()` method with `#[tool(description = "Create a milestone TOML in .assay/milestones/ from structured params...")]`; body: `resolve_cwd()`, `cwd.join(".assay")`, convert `ChunkParams` to `(String, u32)` tuples, call `assay_core::wizard::create_milestone_from_params(...)`, serialize result slug as JSON string, return `domain_error` on failure. Add `spec_create()` method with `#[tool(description = "Create a chunk gates.toml in .assay/specs/<slug>/...")]`; body: `resolve_cwd()`, config load for `specs_dir`, convert `CriterionParams` to `assay_core::wizard::CriterionInput`, call `assay_core::wizard::create_spec_from_params(...)`, return created path as JSON string or `domain_error`. Both methods use `spawn_blocking` wrapping (same pattern as `cycle_advance`). In the test section, add the 5 tests written in T01 that previously failed; each test uses `create_project()` helper + `std::env::set_current_dir()` pattern from existing tests.
  - Verify: `cargo test -p assay-mcp -- milestone_create` — all 3 `milestone_create` tests pass; `cargo test -p assay-mcp -- spec_create` — all 2 `spec_create` tests pass; `cargo test --workspace` — 1308+ tests green; `just ready` green
  - Done when: All 5 new MCP tests pass; `just ready` is green with no regressions

## Files Likely Touched

- `crates/assay-core/src/wizard.rs` — new
- `crates/assay-core/src/lib.rs` — `pub mod wizard`
- `crates/assay-core/tests/wizard.rs` — new (5 integration tests)
- `crates/assay-mcp/src/server.rs` — `MilestoneCreateParams`, `SpecCreateParams`, `ChunkParams`, `CriterionParams`, `milestone_create()`, `spec_create()`, 5 tests
- `crates/assay-cli/src/commands/plan.rs` — new
- `crates/assay-cli/src/commands/mod.rs` — `pub mod plan`
- `crates/assay-cli/src/main.rs` — `Plan` variant + dispatch
- `Cargo.toml` (workspace) — `dialoguer = "0.12.0"`
- `crates/assay-cli/Cargo.toml` — `dialoguer.workspace = true`
