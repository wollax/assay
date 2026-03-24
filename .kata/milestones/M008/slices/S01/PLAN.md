# S01: Advanced PR creation with labels, reviewers, and templates

**Goal:** Extend `assay pr create` to pass labels, reviewers, and a template-rendered body to `gh`, configured in milestone TOML with CLI/MCP overrides.
**Demo:** User adds `pr_labels = ["ready-for-review"]` and `pr_reviewers = ["teammate"]` to milestone TOML; `assay pr create` creates the PR with those labels and reviewer assigned. Proven by integration tests with mock `gh` binary.

## Must-Haves
- Milestone TOML round-trips with `pr_labels`, `pr_reviewers`, `pr_body_template` fields (backward-compatible — existing TOML without these fields loads without error)
- `pr_create_if_gates_pass()` passes `--label` and `--reviewer` flags to `gh pr create` from milestone fields
- `pr_body_template` with `{milestone_name}`, `{milestone_slug}`, `{chunk_list}`, `{gate_summary}` placeholders renders correctly
- CLI `--label` and `--reviewer` flags extend (not replace) TOML values
- MCP `pr_create` tool accepts optional `labels` and `reviewers` params that extend TOML values
- Schema snapshot updated for Milestone type
- Integration tests with mock `gh` binary verify args are passed correctly
- `just ready` passes

## Tasks

- [x] **T01: Milestone type extension + TOML round-trip**
  Add `pr_labels`, `pr_reviewers`, `pr_body_template` to `Milestone` in assay-types with serde(default, skip_serializing_if). Update schema snapshot. Write TOML round-trip tests proving backward compatibility.

- [x] **T02: PR body template rendering + core PR function update**
  Add template rendering (str::replace on 4 placeholders) and update `pr_create_if_gates_pass()` to read labels/reviewers/template from the loaded milestone and pass them as `--label`/`--reviewer`/`--body` args to `gh`. Write integration tests with mock `gh` binary.

- [x] **T03: CLI flags + MCP params + wiring**
  Add `--label` and `--reviewer` repeatable CLI flags to `assay pr create`. Add `labels` and `reviewers` optional params to `PrCreateParams` in assay-mcp. Wire both to pass through to `pr_create_if_gates_pass()` with extend semantics. Write CLI and MCP tests.

## Files Likely Touched
- `crates/assay-types/src/milestone.rs` — new fields
- `crates/assay-core/src/pr.rs` — template rendering + arg construction
- `crates/assay-cli/src/commands/pr.rs` — CLI flags
- `crates/assay-mcp/src/server.rs` — MCP params
- `crates/assay-core/tests/pr.rs` — integration tests
- `crates/assay-types/src/snapshots/` — schema snapshot
