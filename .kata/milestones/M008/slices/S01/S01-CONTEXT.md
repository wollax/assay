---
id: S01
milestone: M008
status: ready
---

# S01: Advanced PR creation with labels, reviewers, and templates — Context

## Goal

Extend `assay pr create` to pass configurable labels, reviewers, and a template-rendered body to `gh pr create`, with configuration in milestone TOML and override capability from CLI flags and MCP tool params.

## Why this Slice

S01 is the foundation for the advanced PR workflow (R058). S02 (TUI PR status panel) depends on the Milestone type having `pr_labels` and `pr_reviewers` fields. Shipping the creation side first means PRs can be created with team-standard labels and reviewers immediately, and the TUI status panel (S02) can read those fields for display.

## Scope

### In Scope

- New `Milestone` fields: `pr_labels: Option<Vec<String>>`, `pr_reviewers: Option<Vec<String>>`, `pr_body_template: Option<String>` — backward-compatible via D092/D117 pattern
- `pr_create_if_gates_pass()` passes `--label` and `--reviewer` flags to `gh pr create` from milestone TOML
- PR body template with placeholder substitution: `{milestone_name}`, `{milestone_slug}`, `{chunk_list}` (bulleted chunk names), `{gate_summary}` (pass/fail counts per chunk)
- CLI `assay pr create` gains `--label` and `--reviewer` flags that **extend** (not replace) TOML values
- MCP `pr_create` tool gains optional `labels` and `reviewers` params that also extend TOML values
- Schema snapshot updated for Milestone type
- Integration tests with mock `gh` binary verifying label/reviewer/body args are passed correctly

### Out of Scope

- PR status polling (S02)
- TUI display of labels/reviewers (S02)
- `{pr_branch}`, `{base_branch}`, `{timestamp}`, `{description}` template placeholders — not in core set
- Pre-validation of reviewer usernames against GitHub API — pass through to `gh` and surface its error (D065-consistent)
- Draft PR support (`--draft` flag)
- PR update/edit after creation

## Constraints

- `Milestone` has `deny_unknown_fields` — new fields must use `serde(default, skip_serializing_if)` per D117
- MCP tools are additive only (D005) — the existing `pr_create` tool signature gains optional params but no existing params change
- CLI follows D072 pattern — domain errors exit with code 1 via eprintln
- `gh` interaction via `std::process::Command` per D065/D008
- Labels from CLI and TOML are merged (extend semantics) — CLI adds to TOML defaults, not replaces
- Reviewers follow the same extend semantics as labels
- Reviewer validation is not done — `gh` errors are surfaced directly to the user

## Integration Points

### Consumes

- `assay-types::Milestone` — extends with 3 new optional fields
- `assay-core::pr::pr_create_if_gates_pass()` — extends signature/internals to read labels/reviewers/template from milestone and build `gh` args
- `assay-core::milestone::milestone_load()` — unchanged, loads the new fields via serde(default)
- `assay-mcp::server::PrCreateParams` — extends with optional labels/reviewers

### Produces

- `Milestone.pr_labels`, `Milestone.pr_reviewers`, `Milestone.pr_body_template` fields in assay-types (used by S02 for display)
- Updated `pr_create_if_gates_pass()` that constructs `--label X --label Y --reviewer A --reviewer B --body <rendered>` args
- Template rendering: simple `str::replace` on 4 placeholders (`{milestone_name}`, `{milestone_slug}`, `{chunk_list}`, `{gate_summary}`)
- CLI `--label` and `--reviewer` repeatable flags
- MCP `labels: Option<Vec<String>>` and `reviewers: Option<Vec<String>>` params on `pr_create`

## Open Questions

- None — all behavioral decisions captured during discuss.
