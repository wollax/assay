---
estimated_steps: 8
estimated_files: 4
---

# T02: PR body template rendering + core PR function update

**Slice:** S01 — Advanced PR creation with labels, reviewers, and templates
**Milestone:** M008

## Description

Add PR body template rendering with 4 placeholders and update `pr_create_if_gates_pass()` to read labels, reviewers, and body template from the loaded milestone, constructing the appropriate `--label`, `--reviewer`, and `--body` arguments for `gh pr create`. Write integration tests with a mock `gh` binary that captures and verifies the args.

## Steps

1. Read `crates/assay-core/src/pr.rs` to understand `pr_create_if_gates_pass()` — specifically how it builds `gh` args and the existing `body` parameter
2. Add a `render_pr_body_template(template, milestone, chunks_with_gates)` free function in `assay-core::pr` that does `str::replace` for `{milestone_name}`, `{milestone_slug}`, `{chunk_list}` (bulleted chunk names), and `{gate_summary}` (pass/fail counts per chunk). If no template is set and no body is provided, omit `--body` entirely.
3. Update `pr_create_if_gates_pass()` to: (a) read `milestone.pr_labels` and append `--label <L>` for each label; (b) read `milestone.pr_reviewers` and append `--reviewer <R>` for each reviewer; (c) if `milestone.pr_body_template` is Some, render it and use as body (caller-provided `body` param takes precedence if both are set)
4. Add a new parameter to `pr_create_if_gates_pass` for extra labels and reviewers from CLI/MCP overrides: `extra_labels: &[String]`, `extra_reviewers: &[String]` — these extend the TOML values
5. Write unit tests for `render_pr_body_template` covering: all placeholders replaced, missing placeholder is passed through verbatim, empty template returns empty string
6. Write integration test `test_pr_create_passes_labels_and_reviewers` using mock `gh` binary pattern from existing tests/pr.rs — mock binary writes its received args to a file, test asserts `--label ready-for-review --reviewer teammate` are present
7. Write integration test `test_pr_create_renders_body_template` — milestone has `pr_body_template` set, assert the rendered body is passed to `gh` as `--body`
8. Run `cargo test -p assay-core` to verify all existing + new tests pass

## Must-Haves

- [ ] `render_pr_body_template()` correctly substitutes `{milestone_name}`, `{milestone_slug}`, `{chunk_list}`, `{gate_summary}`
- [ ] `pr_create_if_gates_pass()` reads `pr_labels` from milestone and passes `--label` flags to `gh`
- [ ] `pr_create_if_gates_pass()` reads `pr_reviewers` from milestone and passes `--reviewer` flags to `gh`
- [ ] `pr_create_if_gates_pass()` renders `pr_body_template` when set and passes as `--body`
- [ ] Caller-provided `body` param takes precedence over `pr_body_template` when both exist
- [ ] `extra_labels` and `extra_reviewers` parameters extend TOML values (union, not replace)
- [ ] Integration tests with mock `gh` binary verify correct args
- [ ] `cargo test -p assay-core` passes

## Verification

- `cargo test -p assay-core` — all tests pass
- Integration test confirms mock `gh` received `--label` and `--reviewer` args
- Unit test confirms template rendering produces expected output for all 4 placeholders

## Observability Impact

- Signals added/changed: error messages from `gh` when reviewer doesn't exist are surfaced as-is (pass-through per S01-CONTEXT)
- How a future agent inspects this: `pr_create_if_gates_pass` error messages include the full `gh` stderr
- Failure state exposed: invalid reviewer → `gh` non-zero exit with descriptive stderr

## Inputs

- `crates/assay-core/src/pr.rs` — existing `pr_create_if_gates_pass()` with `title` and `body` params
- `crates/assay-types/src/milestone.rs` — Milestone with new `pr_labels`, `pr_reviewers`, `pr_body_template` fields (from T01)
- `crates/assay-core/tests/pr.rs` — existing mock `gh` test pattern

## Expected Output

- `crates/assay-core/src/pr.rs` — updated `pr_create_if_gates_pass()` + new `render_pr_body_template()`
- `crates/assay-core/tests/pr.rs` — new integration tests for labels/reviewers/template
