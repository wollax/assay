# Phase 9 Verification: CLI Surface Completion

**Date:** 2026-03-02
**Verifier:** kata-verifier
**Status:** passed

## Must-Have Verification

### Plan 01 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `assay --help` shows all subcommands with clear descriptions and usage examples | PASS | `#[command(after_long_help = "...")]` on all four top-level commands (`init`, `mcp`, `spec`, `gate`) and their subcommands; `about = "Agentic development kit with spec-driven workflows"` on root command. main.rs lines 9-94. |
| 2 | `assay` with no args inside an initialized project shows project status (name, spec inventory) | PASS | `None` arm in `main()` (line 675-684) checks for `.assay` dir and calls `show_status()` (lines 557-609), which prints `assay {version} -- {project_name}` and spec inventory with criteria counts. |
| 3 | `assay` with no args outside a project hints to run `assay init` then shows help | PASS | `None` arm (line 679-683): `eprintln!("Not an Assay project. Run \`assay init\` to get started.")` then `Cli::command().print_help()`. |
| 4 | NO_COLOR detection uses `var_os` (handles non-UTF-8 values correctly) | PASS | `fn colors_enabled()` at line 171: `std::env::var_os("NO_COLOR").is_none()`. Uses `var_os`, not `var`. |
| 5 | MCP server error displays human-readable message (Display, not Debug) | PASS | Line 656: `eprintln!("Error: {e}");` — uses `{}` (Display format), not `{:?}` (Debug format). |
| 6 | Init handler uses `project_root()` instead of inline `current_dir()` | PASS | Line 636: `let root = project_root();`. The only occurrence of `current_dir()` in the file is inside `project_root()` itself (line 217). No inline usage elsewhere. |
| 7 | Spec list aligns slugs consistently regardless of description presence | PASS | `handle_spec_list()` (lines 332-382) computes `name_width` from all slugs before printing, then uses `{:<width$}` padding for both branches (with and without description). |
| 8 | ANSI escape byte count is a named constant, not a magic number | PASS | Line 6: `const ANSI_COLOR_OVERHEAD: usize = 9;` with doc comment explaining the composition. Used at line 315: `type_w = type_width + ANSI_COLOR_OVERHEAD`. |
| 9 | Installed binary is named `assay` (not `assay-cli`) | PASS | `crates/assay-cli/Cargo.toml` lines 9-11: `[[bin]]`, `name = "assay"`, `path = "src/main.rs"`. |

### Plan 02 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 10 | `plugin.json` contains name, version, description, author, homepage, and license fields | PASS | `plugins/claude-code/.claude-plugin/plugin.json` has all six fields: `name`, `version`, `description`, `author`, `homepage`, `license`. |
| 11 | `plugin.json` description matches CLI about text ('Agentic development kit with spec-driven workflows') | PASS | plugin.json: `"description": "Agentic development kit with spec-driven workflows"`. CLI about (main.rs line 12): `about = "Agentic development kit with spec-driven workflows"`. Exact match. |
| 12 | `plugin.json` version matches workspace Cargo.toml version | PASS | plugin.json: `"version": "0.1.0"`. workspace Cargo.toml line 6: `version = "0.1.0"`. Match confirmed; also confirmed by `just ready` output: "Plugin versions match workspace (0.1.0)." |
| 13 | `just sync-plugin-version` updates plugin.json version from Cargo.toml | PASS | justfile lines 67-77: `sync-plugin-version` recipe reads workspace version via `grep`+`sed`, then uses `jq` to update `.version` in `plugins/claude-code/.claude-plugin/plugin.json`. |
| 14 | `just ready` catches plugin.json version drift | PASS | justfile line 32: `ready: fmt-check lint test deny check-plugin-version`. `check-plugin-version` (lines 80-94) compares workspace version to plugin.json version and exits 1 on mismatch. |

## Artifacts Verified

| Artifact | Exists | Correct | Notes |
|----------|--------|---------|-------|
| `crates/assay-cli/src/main.rs` | Yes | Yes | All handlers, constants, and CLI structure present and correct. |
| `crates/assay-cli/Cargo.toml` | Yes | Yes | Binary name `assay` declared, workspace deps only. |
| `plugins/claude-code/.claude-plugin/plugin.json` | Yes | Yes | All 6 required fields present; version and description match workspace/CLI. |
| `justfile` | Yes | Yes | `sync-plugin-version`, `check-plugin-version`, and `ready` recipes all present and correct. |

## Key Links Verified

| From | To | Pattern | Found |
|------|-----|---------|-------|
| `main.rs` `None` arm | `show_status()` | `.assay` dir check → status display | Yes (lines 675-684) |
| `main.rs` `Init` arm | `project_root()` | uses helper, not inline `current_dir()` | Yes (line 636) |
| `main.rs` `Mcp::Serve` | `assay_mcp::serve()` | delegates to library crate | Yes (line 655) |
| `main.rs` MCP error | `eprintln!("Error: {e}")` | Display format | Yes (line 656) |
| `colors_enabled()` | `std::env::var_os` | non-UTF-8 safe | Yes (line 171) |
| `ANSI_COLOR_OVERHEAD` | `const` declaration | named constant | Yes (line 6) |
| `check-plugin-version` | `just ready` | dependency declared | Yes (justfile line 32) |
| `plugin.json` description | CLI `about` text | exact string match | Yes |
| `plugin.json` version | workspace `Cargo.toml` version | exact version match `0.1.0` | Yes |

## Test Suite

- `just ready`: PASS
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets -- -D warnings`: PASS
  - `cargo test --workspace`: PASS (95 tests: 70 assay-core, 16 assay-mcp, 9 assay-types schema roundtrip, 9 schema snapshots, 15 schema validation, 0 failures)
  - `cargo deny check`: PASS (warnings only — duplicate transitive deps and unused license allowances, no errors)
  - `check-plugin-version`: PASS ("Plugin versions match workspace (0.1.0).")

## Summary

Phase 9 (CLI Surface Completion) is fully implemented. All 14 Plan 01 and Plan 02 must-haves are satisfied by direct source inspection:

- The CLI surface is complete with all four subcommand groups (`init`, `mcp`, `spec`, `gate`) wired to `assay-core` and `assay-mcp`.
- The bare `assay` invocation correctly branches on project detection: status view inside a project, help hint outside.
- All code quality requirements are met: `var_os` for `NO_COLOR`, Display format for MCP errors, `project_root()` abstraction, named ANSI constant, slug alignment.
- The binary is correctly named `assay` in `Cargo.toml`.
- `plugin.json` satisfies PLG-01 with all required fields, version and description in sync with the workspace.
- `just ready` enforces version drift detection via `check-plugin-version` and passes cleanly.
