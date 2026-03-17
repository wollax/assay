---
id: T03
parent: S05
milestone: M002
provides:
  - "assay harness generate|install|update|diff CLI subcommands dispatching to claude-code, codex, opencode adapters"
  - "Scope prompt injection into HarnessProfile before adapter dispatch"
  - "File-level diff comparison (added/changed/removed) with exit code 0/1"
key_files:
  - crates/assay-cli/src/commands/harness.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
key_decisions:
  - "Update is an alias for install (same regenerate+overwrite behavior) — simplifies implementation without losing semantics"
  - "Diff prints to stderr, returns exit code 1 if changes detected (0 if none) — machine-checkable"
  - "File content never printed in diff output (redaction constraint for MCP secrets)"
patterns_established:
  - "GeneratedConfig enum wraps adapter-specific config types with unified files()/write() interface"
  - "inject_scope_layer() adds PromptLayer with kind=System, priority=-100 for scope enforcement"
  - "Adapter validation via VALID_ADAPTERS const with actionable error messages listing valid options"
observability_surfaces:
  - "assay harness diff <adapter> — exit code 0 (no changes) or 1 (changes detected), prints added/changed/removed file names to stderr"
  - "assay harness generate/install — prints file count and paths to stderr"
  - "Unknown adapter errors include valid adapter list for self-correction"
duration: 10m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Build `assay harness` CLI subcommand with generate/install/update/diff

**Added `assay harness generate|install|update|diff` CLI subcommands dispatching to all three adapters with scope prompt injection and file-level diff comparison.**

## What Happened

Created `HarnessCommand` clap enum with Generate, Install, Update, and Diff sub-subcommands in `crates/assay-cli/src/commands/harness.rs`. Each command validates the adapter name, builds a `HarnessProfile` (from spec or default), optionally injects a scope prompt layer via `generate_scope_prompt()`, then dispatches to the matching adapter's `generate_config()`.

Key implementation details:
- `GeneratedConfig` enum wraps `ClaudeConfig`, `CodexConfig`, `OpenCodeConfig` with unified `files()` and `write()` methods
- `handle_generate()` prints config to stdout with optional `--output-dir` for file writing
- `handle_install()` writes config to project root; `handle_update()` aliases to the same behavior
- `handle_diff()` compares generated config against existing files, reports added/changed/removed, returns exit code 0 (no changes) or 1 (changes detected)
- `find_existing_adapter_files()` checks known file paths per adapter for removal detection
- Wired into CLI as `Harness { command }` variant in the `Command` enum with help text and examples

11 unit/integration tests cover: adapter validation, config generation for all three adapters, diff detection (all-added, changed files), scope prompt injection (present and absent), install writes to dir, and existing file discovery.

## Verification

- `cargo build -p assay-cli` — compiles without errors ✅
- `cargo test -p assay-cli -- harness` — 11 tests pass ✅
- `cargo run -p assay-cli -- harness generate claude-code` — produces .mcp.json and .claude/settings.json ✅
- `just ready` — full suite green (fmt, lint, test, deny) ✅

Slice-level verification (all pass — this is the final task):
- `cargo test -p assay-types -- scope` — 2 tests pass ✅
- `cargo test -p assay-types -- schema_snapshots` — pass ✅
- `cargo test -p assay-harness -- scope` — 9 tests pass ✅
- `cargo test -p assay-cli -- harness` — 11 tests pass ✅
- `just ready` — all checks pass ✅

## Diagnostics

- `assay harness diff <adapter>` — prints changed/added/removed files to stderr, returns exit code 0/1 for machine consumption
- `assay harness generate <adapter>` — prints file names and byte counts to stderr, content to stdout
- Unknown adapter errors: `"Unknown adapter 'foo'. Valid adapters: claude-code, codex, opencode"`
- Spec not found errors include the resolved path for debugging

## Deviations

- Applied `cargo fmt` to fix formatting issues caught by `just ready` — no functional changes

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/harness.rs` — HarnessCommand enum, handle(), generate/install/update/diff handlers, GeneratedConfig abstraction, 11 integration tests
- `crates/assay-cli/src/commands/mod.rs` — `pub mod harness` already present
- `crates/assay-cli/src/main.rs` — Harness variant in Command enum already wired
