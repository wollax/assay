# S01: Scaffold, Manifest & Dry-Run CLI

**Goal:** Gut the v0.1.0 orchestration code, replace it with the new infrastructure-layer foundation: a TOML job manifest parser/validator, RuntimeProvider trait, new error types, SmeltConfig loader, and a `smelt run --dry-run` CLI command that validates manifests and prints the execution plan.
**Demo:** `smelt run manifest.toml --dry-run` parses a complete job manifest, validates all fields, resolves credential sources from environment, and prints the execution plan — or rejects malformed input with clear errors.

## Must-Haves

- `smelt run manifest.toml --dry-run` prints a structured execution plan (job name, sessions, image, resources, credential status)
- `smelt run bad-manifest.toml --dry-run` exits with a clear error message for each class of validation failure (missing fields, invalid image ref, duplicate session names, circular dependencies, unknown fields)
- The manifest schema supports all top-level sections: `[job]`, `[environment]`, `[credentials]`, `[[session]]`, `[merge]`
- `#[serde(deny_unknown_fields)]` on all manifest structs — strict parsing catches schema mismatches
- `RuntimeProvider` trait is defined with async methods: `provision()`, `exec()`, `collect()`, `teardown()`
- All v0.1.0 modules deleted (orchestrate/, session/, merge/, worktree/, ai/, summary/, init.rs) — git/ module retained and compiles standalone
- `cargo build` and `cargo test` pass with zero warnings
- An example manifest at `examples/job-manifest.toml` demonstrates the full schema

## Proof Level

- This slice proves: contract
- Real runtime required: no (dry-run only — no Docker needed)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-core` — all manifest parsing, validation, and config tests pass
- `cargo test -p smelt-cli` — dry-run integration tests pass
- `cargo build --workspace` — zero errors, zero warnings
- `cargo run -- run examples/job-manifest.toml --dry-run` — prints structured execution plan
- `cargo run -- run examples/bad-manifest.toml --dry-run` — exits with clear validation error

## Observability / Diagnostics

- Runtime signals: Structured tracing events at `info` level for manifest load/validate/plan-print phases; `error` level for validation failures with field-level detail
- Inspection surfaces: `smelt run --dry-run` is the primary diagnostic surface — shows exactly what Smelt would do without doing it
- Failure visibility: Validation errors report the exact field path and constraint violated (e.g., "session[1].timeout: must be > 0")
- Redaction constraints: Credential values are never printed in dry-run output; only credential source (e.g., "env:ANTHROPIC_API_KEY → resolved") is shown

## Integration Closure

- Upstream surfaces consumed: None (first slice)
- New wiring introduced in this slice: `smelt run` CLI entrypoint, manifest types consumed by all downstream slices, RuntimeProvider trait implemented by S02
- What remains before the milestone is truly usable end-to-end: DockerProvider implementation (S02), repo mounting (S03), result collection (S04), monitoring (S05), full integration (S06)

## Tasks

- [x] **T01: Gut v0.1.0 modules and stabilize workspace** `est:30m`
  - Why: The v0.1.0 orchestration code (~9400 lines) must be removed before new code can take its place. The git/ module is retained as reusable infrastructure.
  - Files: `crates/smelt-core/src/lib.rs`, `crates/smelt-core/Cargo.toml`, `crates/smelt-cli/src/main.rs`, `crates/smelt-cli/src/commands/`, `Cargo.toml`
  - Do: Delete all v0.1.0 modules in smelt-core (orchestrate/, session/, merge/, worktree/, ai/, summary/, init.rs). Rewrite lib.rs with only `pub mod git; pub mod error;`. Remove all CLI subcommands except a stub `run` command. Remove unused workspace deps (genai, petgraph, globset, similar, dialoguer, libc, indicatif, comfy-table, which). Fix git/mod.rs imports — remove `WorktreeInfo` dependency, inline or remove worktree-related methods from GitOps trait. Update smelt-cli Cargo.toml to match.
  - Verify: `cargo build --workspace` compiles with zero errors; `cargo test --workspace` passes (git module tests still work)
  - Done when: Workspace compiles clean, only git/ and error.rs remain in smelt-core, CLI has a stub `run` subcommand

- [x] **T02: Job manifest types with strict validation** `est:45m`
  - Why: The manifest is Smelt's primary input contract — every downstream slice consumes it. Strict parsing with deny_unknown_fields catches integration bugs early.
  - Files: `crates/smelt-core/src/manifest.rs`, `crates/smelt-core/src/lib.rs`, `examples/job-manifest.toml`, `examples/bad-manifest.toml`
  - Do: Create manifest.rs with serde structs: `JobManifest`, `JobMeta` (`[job]` — name, repo, base_ref), `Environment` (`[environment]` — runtime, image, resources map), `CredentialConfig` (`[credentials]` — provider, model, optional env overrides), `SessionDef` (`[[session]]` — name, spec, harness, timeout, depends_on), `MergeConfig` (`[merge]` — strategy, order, ai_resolution, target). All structs get `#[serde(deny_unknown_fields)]`. Add `JobManifest::load(path)` and `JobManifest::validate()` methods. Validation checks: unique session names, valid depends_on references (no cycles, no self-refs), required fields present, timeout > 0, image not empty. Write the example manifests. Write unit tests for valid parsing, each validation rule, and deny_unknown_fields rejection.
  - Verify: `cargo test -p smelt-core -- manifest` — all tests pass
  - Done when: `JobManifest::load()` + `validate()` accept valid TOML and reject each class of invalid input with specific error messages

- [x] **T03: RuntimeProvider trait, error types, and SmeltConfig** `est:30m`
  - Why: The RuntimeProvider trait is the extension point for S02 (Docker), and the error enum is consumed by every module. SmeltConfig provides project-level defaults.
  - Files: `crates/smelt-core/src/provider.rs`, `crates/smelt-core/src/error.rs`, `crates/smelt-core/src/config.rs`, `crates/smelt-core/src/lib.rs`
  - Do: Rewrite error.rs with new SmeltError variants: Manifest { field, message }, Provider { operation, message, source? }, Credential { provider, message }, Config { path, message }, Io (keep existing pattern), Git* (keep existing variants). Define RuntimeProvider trait in provider.rs with async methods: `provision(&self, manifest: &JobManifest) -> Result<ContainerId>`, `exec(&self, container: &ContainerId, command: &[String]) -> Result<ExecHandle>`, `collect(&self, container: &ContainerId, manifest: &JobManifest) -> Result<CollectResult>`, `teardown(&self, container: &ContainerId) -> Result<()>`. Use opaque `ContainerId(String)` and `ExecHandle` types. Create config.rs with `SmeltConfig` loading from `.smelt/config.toml` (default_image, credential_sources). Update lib.rs exports.
  - Verify: `cargo build -p smelt-core` — compiles with zero warnings; trait is object-safe or has documented Send + Sync bounds
  - Done when: RuntimeProvider trait compiles and is importable; SmeltError covers all planned error categories; SmeltConfig loads from TOML with sensible defaults

- [x] **T04: `smelt run --dry-run` CLI and execution plan printer** `est:30m`
  - Why: This is the user-facing surface for S01 — the demo. It proves the manifest pipeline works end-to-end from CLI invocation to structured output.
  - Files: `crates/smelt-cli/src/main.rs`, `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/src/commands/mod.rs`
  - Do: Create `run` subcommand with clap: `smelt run <manifest> [--dry-run]`. In dry-run mode: load manifest, validate, resolve credential sources (check env vars, report found/missing), print structured execution plan (job name, repo, image, resources, sessions with deps/timeouts, merge config, credential status). Without `--dry-run`: print "Docker execution not yet implemented" and exit 1 (placeholder for S02). Format output with section headers and aligned fields. Credential values are never printed — only source and resolved/missing status.
  - Verify: `cargo run -- run examples/job-manifest.toml --dry-run` prints the plan; `cargo run -- run examples/bad-manifest.toml --dry-run` exits with validation error; `cargo test -p smelt-cli` integration tests pass
  - Done when: `smelt run --dry-run` is the working entry point, output is human-readable and shows all manifest sections, invalid manifests produce clear errors

## Files Likely Touched

- `crates/smelt-core/src/lib.rs` — rewritten (new module declarations)
- `crates/smelt-core/src/manifest.rs` — new (job manifest types)
- `crates/smelt-core/src/provider.rs` — new (RuntimeProvider trait)
- `crates/smelt-core/src/error.rs` — rewritten (new error variants)
- `crates/smelt-core/src/config.rs` — new (SmeltConfig)
- `crates/smelt-core/src/git/mod.rs` — modified (remove worktree imports)
- `crates/smelt-core/src/git/cli.rs` — modified (remove worktree methods)
- `crates/smelt-core/Cargo.toml` — updated (remove unused deps)
- `crates/smelt-cli/src/main.rs` — rewritten (new CLI structure)
- `crates/smelt-cli/src/commands/run.rs` — new
- `crates/smelt-cli/src/commands/mod.rs` — rewritten
- `crates/smelt-cli/Cargo.toml` — updated
- `Cargo.toml` — updated (workspace deps cleanup)
- `examples/job-manifest.toml` — new (replaces agent-manifest.toml)
- `examples/bad-manifest.toml` — new (invalid manifest for testing)
