---
id: T02
parent: S01
milestone: M008
provides:
  - render_pr_body_template() with 4 placeholder substitutions ({milestone_name}, {milestone_slug}, {chunk_list}, {gate_summary})
  - pr_create_if_gates_pass() reads pr_labels/pr_reviewers from milestone TOML and passes --label/--reviewer to gh
  - pr_create_if_gates_pass() renders pr_body_template when set and passes as --body
  - extra_labels/extra_reviewers parameters for CLI/MCP override (extend semantics)
  - ChunkGateSummary type for gate summary data passed to template rendering
key_files:
  - crates/assay-core/src/pr.rs
  - crates/assay-core/tests/pr.rs
key_decisions:
  - "Caller-provided body takes precedence over pr_body_template when both exist"
  - "Gate summaries collected by re-evaluating gates during PR creation (reuses existing evaluate_all_gates)"
  - "Mock gh uses NUL-separated arg capture to handle multiline body values"
patterns_established:
  - "NUL-separated arg capture in mock gh scripts for multiline value testing"
observability_surfaces:
  - "gh stderr pass-through on failure (invalid reviewer surfaces gh error directly)"
duration: 25min
verification_result: passed
completed_at: 2026-03-23T20:00:00Z
blocker_discovered: false
---

# T02: PR body template rendering + core PR function update

**Added render_pr_body_template() with 4 placeholders and extended pr_create_if_gates_pass() to pass --label/--reviewer/--body args to gh from milestone TOML, with extra_labels/extra_reviewers extend semantics for CLI/MCP overrides**

## What Happened

Added `render_pr_body_template(template, milestone, gate_summaries)` as a pure function doing `str::replace` on 4 placeholders: `{milestone_name}`, `{milestone_slug}`, `{chunk_list}` (bulleted chunk list from milestone.chunks), and `{gate_summary}` (pass/fail counts from ChunkGateSummary). Unknown placeholders pass through verbatim.

Extended `pr_create_if_gates_pass()` signature with `extra_labels: &[String]` and `extra_reviewers: &[String]` parameters. The function now reads `pr_labels` and `pr_reviewers` from the loaded milestone TOML, chains them with the extra_* params (extend semantics — union, not replace), and appends `--label` / `--reviewer` flags to the `gh pr create` args. If `pr_body_template` is set on the milestone, it renders it and uses as `--body` unless the caller explicitly provides a `body` param (which takes precedence).

Gate summaries for template rendering are collected by re-evaluating all chunk gates during PR creation — this reuses the same `evaluate_all_gates` call already happening for the gate check, but a second pass is needed because the gate check early-returns on failure while template rendering needs all chunk results.

Updated all existing callers across assay-cli, assay-mcp, and test files to pass `&[], &[]` for the new parameters.

## Verification

- `cargo test -p assay-core --lib -- pr::tests` — 3 unit tests pass (all placeholders, unknown passthrough, empty template)
- `cargo test -p assay-core --test pr` — 12 integration tests pass (9 existing + 3 new: labels/reviewers, body template, caller body override)
- `cargo test -p assay-core -p assay-cli -p assay-mcp -p assay-harness -p assay-tui` — all 1100+ tests pass across all crates, zero failures

## Diagnostics

- `gh` stderr is surfaced directly in error messages when reviewer usernames are invalid or `gh` fails
- Template rendering is a pure function — inspect output by calling `render_pr_body_template()` directly in tests

## Deviations

- Added `ChunkGateSummary` as a new public type in assay-core::pr (not in the plan) — needed to pass gate results to the template renderer
- Gate summaries are re-evaluated rather than reusing the gate-check results, because the gate check short-circuits on failure while template rendering needs all results. Minor performance cost (~100ms for typical projects) but avoids complex result threading.
- Mock `gh` script uses NUL-separated arg capture instead of newline-separated — required because PR body templates contain newlines that would break line-by-line parsing.

## Known Issues

- Pre-existing: `schema_roundtrip` test in assay-types doesn't compile without the `orchestrate` feature — not caused by this task, exists prior to M008.

## Files Created/Modified

- `crates/assay-core/src/pr.rs` — added render_pr_body_template(), ChunkGateSummary, updated pr_create_if_gates_pass() signature and arg building
- `crates/assay-core/tests/pr.rs` — 3 new integration tests (labels/reviewers, body template, caller body override), NUL-separated arg capture helper
- `crates/assay-core/src/wizard.rs` — added pr_labels/pr_reviewers/pr_body_template: None to Milestone constructors
- `crates/assay-cli/src/commands/pr.rs` — updated caller to pass &[], &[]
- `crates/assay-mcp/src/server.rs` — updated caller to pass &[], &[]
- `crates/assay-core/tests/wizard.rs` — added new fields to Milestone constructors
- `crates/assay-core/tests/cycle.rs` — added new fields to Milestone constructors
- `crates/assay-core/tests/milestone_io.rs` — added new fields to Milestone constructors
