---
estimated_steps: 5
estimated_files: 5
---

# T03: Build `assay harness` CLI subcommand with generate/install/update/diff

**Slice:** S05 — Harness CLI & Scope Enforcement
**Milestone:** M002

## Description

Create the `assay harness` CLI subcommand family (generate, install, update, diff) that dispatches to all three adapters (claude-code, codex, opencode). The `generate` command loads a spec, builds a HarnessProfile, injects scope prompt layer from ManifestSession scope fields, then calls the matching adapter's `generate_config()`. `install` writes config to the project root via `write_config()`. `update` regenerates and overwrites (same as install). `diff` compares generated config against existing files and prints a changed/added/removed summary. Wire into the existing CLI command registry.

## Steps

1. Create `crates/assay-cli/src/commands/harness.rs` with clap `HarnessCommand` enum:
   - `Generate { adapter: String, spec: Option<String>, workflow: Option<String>, output_dir: Option<String> }`
   - `Install { adapter: String, spec: Option<String> }`
   - `Update { adapter: String, spec: Option<String> }`
   - `Diff { adapter: String, spec: Option<String> }`
   Add `pub fn handle(command: HarnessCommand) -> anyhow::Result<i32>` with match dispatch.
2. Implement `handle_generate()`: resolve adapter name ("claude-code"|"codex"|"opencode"), optionally load spec from .assay/specs/, build HarnessProfile (via `build_harness_profile()` if spec provided, or minimal default), inject scope prompt via `generate_scope_prompt()` as PromptLayer if file_scope is non-empty, dispatch to adapter's `generate_config()`. Print config summary to stdout (file names + content). If --output-dir provided, call `write_config()` to that dir.
3. Implement `handle_install()`: generate config then call `write_config()` to project root. Print files written. Implement `handle_update()` as alias (same behavior — regenerate + overwrite).
4. Implement `handle_diff()`: generate new config, read existing files from project root, compare. Print summary: files that would be added, changed, or removed. Report file names only, not content (per redaction constraint). Return exit code 0 if no changes, 1 if changes detected.
5. Wire into CLI: add `pub mod harness;` to `commands/mod.rs`. Add `Harness { command: HarnessCommand }` variant to `Command` enum in `main.rs` with help text and examples. Add match arm calling `commands::harness::handle(command)`. Write integration tests in harness.rs verifying adapter dispatch (at minimum: generate for each adapter produces non-empty output, unknown adapter errors, diff with no existing files shows all-added). Run `just ready`.

## Must-Haves

- [ ] `assay harness generate claude-code` dispatches to claude adapter
- [ ] `assay harness generate codex` dispatches to codex adapter
- [ ] `assay harness generate opencode` dispatches to opencode adapter
- [ ] `assay harness install <adapter>` writes config to project root
- [ ] `assay harness update <adapter>` regenerates and overwrites
- [ ] `assay harness diff <adapter>` shows file-level change summary without content
- [ ] Unknown adapter name produces actionable error message
- [ ] Scope prompt injection: if ManifestSession has file_scope, scope PromptLayer is injected before generate_config
- [ ] `just ready` passes

## Verification

- `cargo build -p assay-cli` — compiles without errors
- `cargo test -p assay-cli -- harness` — CLI integration tests pass
- `just ready` — full suite green
- Manual spot check: `cargo run -p assay-cli -- harness generate claude-code` shows help or minimal output

## Observability Impact

- Signals added/changed: CLI prints file-level summaries (written/changed/added/removed) to stderr for human consumption
- How a future agent inspects this: `assay harness diff <adapter>` returns exit code 0/1 — machine-checkable; output lists affected files
- Failure state exposed: anyhow errors with context (e.g., "Unknown adapter 'foo'. Valid adapters: claude-code, codex, opencode")

## Inputs

- `crates/assay-harness/src/{claude,codex,opencode}.rs` — generate_config/write_config functions from S04
- `crates/assay-harness/src/scope.rs` — generate_scope_prompt from T02
- `crates/assay-core/src/pipeline.rs` — build_harness_profile() for profile construction
- `crates/assay-cli/src/commands/worktree.rs` — template for clap subcommand pattern

## Expected Output

- `crates/assay-cli/src/commands/harness.rs` — new: HarnessCommand enum, handle(), generate/install/update/diff handlers, integration tests
- `crates/assay-cli/src/commands/mod.rs` — pub mod harness added
- `crates/assay-cli/src/main.rs` — Harness variant in Command enum with match arm
