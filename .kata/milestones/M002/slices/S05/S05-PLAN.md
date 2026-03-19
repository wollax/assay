# S05: Harness CLI & Scope Enforcement

**Goal:** `assay harness generate|install|update|diff` CLI subcommands work for all three adapters, scope enforcement via globset detects violations, and multi-agent awareness prompts are injected into harness config.
**Demo:** `assay harness generate claude-code --spec auth` produces scoped config to stdout. `assay harness install claude-code` writes config into the project. `assay harness diff codex` shows what would change. A manifest with `file_scope` and `shared_files` fields produces scope-aware prompts and `check_scope()` returns violations for out-of-scope file changes.

## Must-Haves

- `ScopeConfig` and `ScopeViolation` types in assay-types with serde derives, deny_unknown_fields, inventory registration, and schema snapshots
- `file_scope` and `shared_files` fields on `ManifestSession` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` — backward compatible
- `check_scope()` function in assay-harness using globset for pattern matching, returning `Vec<ScopeViolation>` — advisory, not blocking
- `generate_scope_prompt()` function producing multi-agent awareness markdown, injected as `PromptLayer` with `PromptLayerKind::System` and priority -100
- `assay harness generate <adapter> [--spec <name>]` CLI subcommand dispatching to all three adapters
- `assay harness install <adapter>` writing config files into project root
- `assay harness update <adapter>` regenerating and overwriting managed files
- `assay harness diff <adapter>` showing changed/added/removed file summary without writing
- Updated schema snapshots for `ManifestSession` and `RunManifest` (new fields)

## Proof Level

- This slice proves: contract + integration (CLI integration tests exercise real dispatch, scope enforcement proven by unit tests with globset patterns)
- Real runtime required: no (tests use in-memory profiles and tempdir)
- Human/UAT required: no (snapshot tests lock output format; manual `assay harness generate` with real specs is optional UAT)

## Verification

- `cargo test -p assay-types -- scope` — ScopeViolation round-trip and schema snapshot tests
- `cargo test -p assay-types -- schema_snapshots` — updated ManifestSession/RunManifest snapshots pass
- `cargo test -p assay-harness -- scope` — check_scope and generate_scope_prompt unit tests
- `cargo test -p assay-cli -- harness` — CLI dispatch integration tests (generate/install/diff for each adapter)
- `just ready` — full suite green (fmt, lint, test, deny)

## Observability / Diagnostics

- Runtime signals: `ScopeViolation` structs with file path, violation type, and pattern context — structured and serializable
- Inspection surfaces: `assay harness diff <adapter>` prints changed/added/removed files without applying — agents can check before install
- Failure visibility: CLI commands return structured anyhow errors with context (missing spec, unknown adapter, write failure)
- Redaction constraints: `harness diff` reports file names only, not file contents (avoids leaking secrets from MCP config)

## Integration Closure

- Upstream surfaces consumed: `assay-harness::{claude,codex,opencode}::{generate_config,write_config}`, `assay-core::pipeline::build_harness_profile()`, `ManifestSession` type, `HarnessProfile` type with `prompt_layers`
- New wiring introduced in this slice: `assay harness` CLI subcommand dispatching to all three adapters; scope prompt injection into HarnessProfile before adapter dispatch; `ScopeConfig` fields on ManifestSession
- What remains before the milestone is truly usable end-to-end: S06 wires orchestrator to real CLI entrypoint, adds MCP tools, integrates scope-aware config into orchestrated runs

## Tasks

- [x] **T01: Add scope types to assay-types and ManifestSession fields** `est:20m`
  - Why: Foundation types needed by scope enforcement and CLI — ScopeViolation for enforcement results, file_scope/shared_files on ManifestSession for user authoring
  - Files: `crates/assay-types/src/harness.rs`, `crates/assay-types/src/manifest.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: Add `ScopeViolation` and `ScopeViolationType` types to harness.rs with serde derives and inventory registration. Add `file_scope: Vec<String>` and `shared_files: Vec<String>` to ManifestSession with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Add schema snapshot tests. Run `cargo insta review` to accept updated ManifestSession/RunManifest snapshots.
  - Verify: `cargo test -p assay-types` — all existing + new tests pass, snapshots accepted
  - Done when: ScopeViolation round-trips through JSON, ManifestSession parses with and without scope fields, all schema snapshots locked

- [x] **T02: Implement scope enforcement and multi-agent prompt generation in assay-harness** `est:25m`
  - Why: Core scope logic — check_scope() validates file changes against glob patterns, generate_scope_prompt() produces multi-agent awareness text for injection into harness config
  - Files: `crates/assay-harness/src/scope.rs`, `crates/assay-harness/src/lib.rs`, `crates/assay-harness/Cargo.toml`, `Cargo.toml`
  - Do: Add globset workspace dependency. Create scope.rs with `check_scope(file_scope, shared_files, changed_files) -> Vec<ScopeViolation>` using `GlobSet`, and `generate_scope_prompt(session_name, file_scope, shared_files, all_sessions) -> String`. Add `pub mod scope;` to lib.rs. Write unit tests covering: empty scope (no restrictions), matched files, out-of-scope violations, shared file detection, prompt generation with neighbors.
  - Verify: `cargo test -p assay-harness -- scope` — all scope tests pass
  - Done when: check_scope correctly classifies files against glob patterns; generate_scope_prompt produces concise multi-agent markdown; globset compiles patterns once per invocation

- [x] **T03: Build `assay harness` CLI subcommand with generate/install/update/diff** `est:30m`
  - Why: User-facing CLI surface — dispatches to all three adapters, handles scope prompt injection before generation, and provides install/update/diff lifecycle commands
  - Files: `crates/assay-cli/src/commands/harness.rs`, `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/main.rs`
  - Do: Create harness.rs with clap `HarnessCommand` enum (Generate, Install, Update, Diff sub-subcommands). Generate: load spec → build_harness_profile → inject scope prompt layer → dispatch to adapter generate_config → print to stdout or write with --output-dir. Install: generate + write_config to project root. Update: same as install (regenerate and overwrite). Diff: generate new config, compare with existing files on disk, print changed/added/removed summary. Wire into mod.rs and main.rs Command enum.
  - Verify: `cargo build -p assay-cli` compiles; `cargo test -p assay-cli -- harness` — integration tests for generate/install/diff dispatch
  - Done when: `assay harness generate claude-code`, `assay harness install codex`, `assay harness diff opencode` all route to correct adapters; diff shows file-level changes without content; `just ready` passes

## Files Likely Touched

- `Cargo.toml` (workspace deps: globset)
- `crates/assay-types/src/harness.rs` (ScopeViolation, ScopeViolationType)
- `crates/assay-types/src/manifest.rs` (file_scope, shared_files on ManifestSession)
- `crates/assay-types/tests/schema_snapshots.rs` (new snapshot tests)
- `crates/assay-harness/Cargo.toml` (globset dep)
- `crates/assay-harness/src/lib.rs` (pub mod scope)
- `crates/assay-harness/src/scope.rs` (new: check_scope, generate_scope_prompt)
- `crates/assay-cli/src/commands/harness.rs` (new: HarnessCommand, generate/install/update/diff)
- `crates/assay-cli/src/commands/mod.rs` (pub mod harness)
- `crates/assay-cli/src/main.rs` (Harness variant in Command enum)
