---
id: T03
parent: S04
milestone: M005
provides:
  - "`crates/assay-cli/src/commands/pr.rs` — `PrCommand`, `handle`, `pr_create_cmd`, 2 unit tests"
  - "`assay pr create <milestone>` CLI subcommand wired into main.rs with `Pr` variant and dispatch arm"
  - "`PrCreateParams` struct and `pr_create` MCP tool method in `server.rs` following cycle_advance spawn_blocking pattern"
  - "`pr_create_tool_in_router` presence test in assay-mcp"
  - "`pr_create_if_gates_pass` signature extended with `title: &str` and `body: Option<&str>` params"
key_files:
  - crates/assay-cli/src/commands/pr.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-core/src/pr.rs
  - crates/assay-core/tests/pr.rs
key_decisions:
  - "Extended pr_create_if_gates_pass to accept title and body params — T03 task plan called for CLI to pass effective_title; core needed to accept it rather than derive title from milestone name"
  - "CLI pr_create_cmd constructs effective_title as title.unwrap_or_else(|| format!(\"feat: {milestone}\")) before calling core — consistent with D072 pattern"
patterns_established:
  - "New CLI subcommand group pattern: create commands/<name>.rs with #[derive(Subcommand)] enum, handle fn, add pub mod <name>; to mod.rs, add variant+dispatch to main.rs"
observability_surfaces:
  - "CLI stdout: 'PR created: #N — <url>' on success; stderr: 'Error: <msg>' on failure — exit 0/1"
  - "MCP pr_create returns {pr_number, pr_url} JSON on success; CallToolResult with isError on failure"
  - "assay pr create --help shows <MILESTONE>, --title, --body"
duration: 25min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T03: Wire CLI `assay pr create` and MCP `pr_create` tool

**CLI `assay pr create <milestone>` and MCP `pr_create` tool wired to `pr_create_if_gates_pass`; all 11 tests green, `just ready` clean.**

## What Happened

Created `crates/assay-cli/src/commands/pr.rs` with `PrCommand::Create { milestone, title, body }`, `handle`, and `pr_create_cmd`. The function constructs `effective_title = title.unwrap_or_else(|| format!("feat: {milestone}"))` then delegates to `assay_core::pr::pr_create_if_gates_pass`. On success it prints `"PR created: #{n} — {url}"` (exit 0); on error it writes `eprintln!("Error: {e}")` and returns exit 1 (D072 pattern).

Added `pub mod pr;` to `commands/mod.rs` and a `Command::Pr { command: commands::pr::PrCommand }` variant with `after_long_help` examples to `main.rs`, plus the dispatch arm `Some(Command::Pr { command }) => commands::pr::handle(command)`.

Added `PrCreateParams` (milestone_slug, title, body) and the `pr_create` async method to `server.rs` following the `cycle_advance` spawn_blocking pattern exactly. Added `pr_create_tool_in_router` presence test. Updated the module-level doc comment to list `pr_create`.

**Deviation:** `pr_create_if_gates_pass` in T02 only accepted 4 params and derived title from `milestone.name`. The task plan required the CLI to pass an `effective_title`, so the core signature was extended to accept `title: &str` and `body: Option<&str>`. The `--json number,url` gh args list was updated to include `--body` when provided. All 8 integration tests in `tests/pr.rs` updated to pass the new args.

Also removed unused `PrCreateResult` import from `tests/pr.rs` (caught by clippy `-D warnings`).

## Verification

- `cargo test -p assay-cli -- pr` → 2 passed (`pr_create_cmd_exits_1_no_assay_dir`, `pr_create_cmd_exits_1_already_created`)
- `cargo test -p assay-mcp -- pr_create` → 1 passed (`pr_create_tool_in_router`)
- `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 passed (all integration tests)
- `cargo test --workspace` → all green
- `just ready` → "All checks passed."
- `assay pr create --help` → shows `<MILESTONE>` positional arg, `--title`, `--body` options

## Diagnostics

- `assay pr create <slug>` exit 0 = PR created; exit 1 = failure (stderr has reason)
- MCP `pr_create` response: `isError: false` + `{pr_number, pr_url}` on success; `isError: true` on domain failure
- `cat .assay/milestones/<slug>.toml` shows `pr_number` and `pr_url` fields after successful creation

## Deviations

`pr_create_if_gates_pass` signature extended from 4 to 6 params (`title: &str`, `body: Option<&str>`). T02 derived title from `milestone.name`; T03 task plan required the caller to pass `effective_title`. Updated all 8 call sites in `tests/pr.rs` to pass the new args.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/pr.rs` — new: PrCommand, handle, pr_create_cmd, 2 unit tests
- `crates/assay-cli/src/commands/mod.rs` — `pub mod pr;` added
- `crates/assay-cli/src/main.rs` — `Command::Pr` variant + dispatch arm added
- `crates/assay-mcp/src/server.rs` — PrCreateParams, pr_create method, pr_create_tool_in_router test, doc comment updated
- `crates/assay-core/src/pr.rs` — pr_create_if_gates_pass signature extended with title/body params
- `crates/assay-core/tests/pr.rs` — all 8 call sites updated for new signature; unused import removed
