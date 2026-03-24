---
id: S01
parent: M008
milestone: M008
provides:
  - Milestone.pr_labels Option<Vec<String>> — configurable PR labels in TOML
  - Milestone.pr_reviewers Option<Vec<String>> — configurable PR reviewers in TOML
  - Milestone.pr_body_template Option<String> — template-rendered PR body with 4 placeholders
  - render_pr_body_template() for {milestone_name}, {milestone_slug}, {chunk_list}, {gate_summary}
  - pr_create_if_gates_pass() passes --label/--reviewer/--body to gh from TOML + overrides
  - --label/--reviewer repeatable CLI flags on assay pr create
  - PrCreateParams.labels/reviewers MCP params with extend semantics
  - ChunkGateSummary type for gate result aggregation
  - Updated Milestone schema snapshot
requires:
  - slice: none
    provides: standalone — extends existing Milestone type and pr.rs
affects:
  - S02
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-core/src/pr.rs
  - crates/assay-cli/src/commands/pr.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-core/tests/pr.rs
key_decisions:
  - "D117: New Milestone TOML fields use D092 serde(default, skip_serializing_if) pattern"
  - "D121: Caller-provided body takes precedence over pr_body_template"
  - "CLI/MCP labels/reviewers extend TOML values (union, not replace)"
patterns_established:
  - "NUL-separated arg capture in mock gh scripts for multiline body testing"
  - "extra_labels/extra_reviewers pass-through pattern from CLI/MCP to core"
observability_surfaces:
  - gh stderr pass-through on failure (invalid reviewer surfaces gh error directly)
drill_down_paths:
  - .kata/milestones/M008/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M008/slices/S01/tasks/T03-SUMMARY.md
duration: 50min
verification_result: passed
completed_at: 2026-03-23T20:30:00Z
---

# S01: Advanced PR creation with labels, reviewers, and templates

**Extended assay pr create with TOML-configurable labels, reviewers, and template-rendered PR body, with CLI/MCP override extend semantics proven by 12 integration tests**

## What Happened

T01 extended the Milestone struct with three new optional fields (`pr_labels`, `pr_reviewers`, `pr_body_template`) using the established D092 backward-compatibility pattern. Schema snapshot updated, TOML round-trip tests confirm existing files without these fields load as None.

T02 added `render_pr_body_template()` — a pure function doing `str::replace` on 4 placeholders (`{milestone_name}`, `{milestone_slug}`, `{chunk_list}`, `{gate_summary}`). Extended `pr_create_if_gates_pass()` to read labels/reviewers/template from the loaded milestone and pass them as `--label`/`--reviewer`/`--body` args to `gh`. Introduced `extra_labels`/`extra_reviewers` parameters with extend semantics (union of TOML + caller values). Added `ChunkGateSummary` type for gate result aggregation in templates. 3 new integration tests with mock `gh` binary verify labels, template rendering, and caller body override.

T03 wired the CLI and MCP surfaces: added repeatable `--label`/`--reviewer` clap flags to `PrCommand::Create` and `labels`/`reviewers` optional params to `PrCreateParams` in assay-mcp. Both forward to the core function's extra_labels/extra_reviewers. 2 CLI parsing tests and 1 MCP deserialization test added.

## Verification

- `cargo test -p assay-types --lib -- milestone` — 9 tests pass (backward compat + new round-trip)
- `cargo test -p assay-core --test pr` — 12 integration tests pass (labels, reviewers, body template, caller override, existing tests)
- `cargo test -p assay-cli --bin assay -- pr` — 4 tests pass (including new flag parsing tests)
- `cargo test -p assay-mcp --lib -- pr_create` — 2 tests pass (router + param deserialization)
- `cargo clippy --workspace --all-targets -- -D warnings` — passes clean
- `cargo fmt --check` — passes clean
- Schema snapshot up to date

## Requirements Advanced

- R058 (Advanced PR workflow) — PR creation now supports labels, reviewers, and body templates from TOML with CLI/MCP overrides. TUI PR status panel remains for S02.

## Requirements Validated

- None fully validated by this slice alone — R058 spans S01 + S02.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- T02 added `ChunkGateSummary` type (not in plan) — needed to pass structured gate results to the template renderer.
- T02 uses NUL-separated arg capture in mock gh (not in plan) — required because PR body templates contain newlines that break line-by-line parsing.
- T02 re-evaluates gates for template rendering rather than reusing the gate-check results — the gate check short-circuits on failure while template rendering needs all results. Minor performance cost (~100ms typical).
- T03 added `#[allow(clippy::too_many_arguments)]` on `pr_create_if_gates_pass()` — the 8-param signature from T02 exceeds clippy's default 7.

## Known Limitations

- Pre-existing: `schema_roundtrip` test in assay-types doesn't compile without the `orchestrate` feature — not caused by S01.
- `just ready` times out on mesh integration tests — pre-existing, unrelated to S01 changes.
- Gate summaries are re-evaluated (second pass) during template rendering rather than being threaded from the gate check — minor inefficiency, not user-visible.

## Follow-ups

- None — S02 consumes S01's outputs for the TUI PR status panel as planned.

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — 3 new fields on Milestone + round-trip test
- `crates/assay-types/src/snapshots/` — updated Milestone schema snapshot
- `crates/assay-core/src/pr.rs` — render_pr_body_template(), ChunkGateSummary, extended pr_create_if_gates_pass()
- `crates/assay-core/tests/pr.rs` — 3 new integration tests (labels/reviewers, body template, caller body override)
- `crates/assay-cli/src/commands/pr.rs` — --label/--reviewer repeatable flags, 2 new tests
- `crates/assay-mcp/src/server.rs` — labels/reviewers on PrCreateParams, 1 new test
- `crates/assay-core/src/wizard.rs` — new fields in Milestone constructors
- `crates/assay-core/tests/wizard.rs` — new fields in Milestone constructors
- `crates/assay-core/tests/cycle.rs` — new fields in Milestone constructors
- `crates/assay-core/tests/milestone_io.rs` — new fields in Milestone constructors

## Forward Intelligence

### What the next slice should know
- `pr_create_if_gates_pass()` now has 8 params including `extra_labels`/`extra_reviewers` — the `#[allow(clippy::too_many_arguments)]` is already in place.
- `Milestone.pr_number` and `Milestone.pr_url` (pre-existing fields) are set during PR creation — S02 should read these to decide which milestones to poll for PR status.
- The mock `gh` binary pattern in `crates/assay-core/tests/pr.rs` uses NUL-separated arg capture for multiline values — reuse this pattern if S02 needs to test `gh pr view` output.

### What's fragile
- The 8-parameter signature on `pr_create_if_gates_pass()` is at the edge of maintainability — if S02 needs to add more params, consider a `PrCreateOptions` struct.

### Authoritative diagnostics
- `cargo test -p assay-core --test pr` is the authoritative test for all PR creation behavior — it uses real filesystem + mock `gh` binary.

### What assumptions changed
- None — the plan's approach worked as designed. D092 pattern confirmed suitable for Milestone extension.
