---
estimated_steps: 5
estimated_files: 4
---

# T03: Add `assay history` CLI subcommand

**Slice:** S04 — Gate History Analytics Engine and CLI
**Milestone:** M008

## Description

Wire the analytics engine into the CLI as a top-level `assay history` subcommand with `--analytics` and `--json` flags. This is the user-facing surface that completes R059. Includes structured text formatting for human-readable output and JSON for machine consumption.

## Steps

1. Create `crates/assay-cli/src/commands/history.rs`:
   - Define `HistoryCommand` enum with subcommand variant `Analytics { json: bool }`
   - Implement `pub fn handle(command: HistoryCommand) -> anyhow::Result<i32>` 
   - For `Analytics`: resolve `project_root()` → `assay_dir()`, call `compute_analytics()`, format output
   - Structured text formatter: header "Gate Failure Frequency", table with columns `Spec`, `Criterion`, `Fails`, `Runs`, `Rate`, `Enforcement`; sorted by fail_count desc; then header "Milestone Velocity", table with `Milestone`, `Chunks`, `Days`, `Rate`; then footer with unreadable_records if > 0
   - JSON formatter: `serde_json::to_string_pretty(&report)` to stdout
   - Use `colors_enabled()` for ANSI coloring (red for high fail rate, green for passing, consistent with existing gate output)
2. Add `pub mod history;` to `crates/assay-cli/src/commands/mod.rs`
3. Add `History` variant to the `Command` enum in `crates/assay-cli/src/main.rs`:
   - Top-level command: `History { #[command(subcommand)] command: commands::history::HistoryCommand }`
   - Add dispatch in `run()`: `Some(Command::History { command }) => commands::history::handle(command)`
   - Add help text examples
4. Write CLI tests in `history.rs` `#[cfg(test)]` module:
   - `test_analytics_text_output_shape` — synthetic data → text output contains expected headers and table rows
   - `test_analytics_json_output_valid` — synthetic data → output parses as valid `AnalyticsReport` JSON
   - `test_analytics_no_project_shows_error` — non-project dir → error message, exit code 1
   - `test_analytics_empty_project` — project with no history → empty tables (not error)
5. Run `just ready` to confirm full workspace passes

## Must-Haves

- [ ] `assay history analytics` CLI command exists as a top-level subcommand
- [ ] Structured text output includes failure frequency table and milestone velocity table
- [ ] `--json` flag produces valid JSON that deserializes to `AnalyticsReport`
- [ ] Unreadable records count shown in text footer when > 0
- [ ] Error handling: non-project directory → helpful error message with exit code 1
- [ ] `just ready` passes with zero warnings

## Verification

- `cargo test -p assay-cli -- history` — CLI tests pass
- `just ready` — full workspace green
- Manual: `cd /tmp && assay history analytics` → graceful error (no project)

## Observability Impact

- Signals added/changed: CLI output surfaces `unreadable_records` count to users; `--json` provides machine-readable analytics for automation
- How a future agent inspects this: `assay history analytics --json` → parse JSON → check fields
- Failure state exposed: Non-project error with exit code 1; unreadable records count in output

## Inputs

- `crates/assay-core/src/history/analytics.rs` — `compute_analytics()` function from T02
- `crates/assay-cli/src/commands/mod.rs` — existing command module structure
- `crates/assay-cli/src/main.rs` — existing `Command` enum to extend
- Existing CLI patterns: `commands/gate.rs` (table formatting), `commands/pr.rs` (subcommand structure), `commands/mod.rs` (shared helpers)

## Expected Output

- `crates/assay-cli/src/commands/history.rs` — new file: HistoryCommand enum + handle() + text/JSON formatters + tests
- `crates/assay-cli/src/commands/mod.rs` — modified: `pub mod history` added
- `crates/assay-cli/src/main.rs` — modified: `History` command variant + dispatch + help text
- `just ready` passing
