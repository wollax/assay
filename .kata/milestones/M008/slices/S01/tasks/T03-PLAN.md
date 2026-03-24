---
estimated_steps: 7
estimated_files: 3
---

# T03: CLI flags + MCP params + wiring

**Slice:** S01 — Advanced PR creation with labels, reviewers, and templates
**Milestone:** M008

## Description

Add `--label` and `--reviewer` repeatable CLI flags to `assay pr create` and optional `labels`/`reviewers` params to the `pr_create` MCP tool. Wire both surfaces to pass through to `pr_create_if_gates_pass()` using the `extra_labels`/`extra_reviewers` extend semantics from T02. Write CLI and MCP tests.

## Steps

1. Read `crates/assay-cli/src/commands/pr.rs` to understand the existing `PrCommand::Create` struct and `pr_create_cmd` function
2. Add `--label` as a repeatable `Vec<String>` clap arg and `--reviewer` as a repeatable `Vec<String>` clap arg to `PrCommand::Create`
3. Update `pr_create_cmd` to pass these as `extra_labels` and `extra_reviewers` to `pr_create_if_gates_pass()`
4. Read `crates/assay-mcp/src/server.rs` to understand `PrCreateParams` and the `pr_create` handler
5. Add `labels: Option<Vec<String>>` and `reviewers: Option<Vec<String>>` to `PrCreateParams` with `#[serde(default)]`
6. Update the `pr_create` MCP handler to pass these as `extra_labels` and `extra_reviewers` (defaulting None to empty vec)
7. Write tests: CLI test verifying `--label X --label Y --reviewer Z` are accepted and forwarded; MCP test verifying `labels` and `reviewers` params deserialize and are forwarded. Run `just ready` to confirm everything passes.

## Must-Haves

- [ ] `assay pr create <slug> --label X --label Y --reviewer Z` is accepted by clap (repeatable flags)
- [ ] CLI labels/reviewers are passed to `pr_create_if_gates_pass()` as `extra_labels`/`extra_reviewers`
- [ ] `PrCreateParams` has `labels: Option<Vec<String>>` and `reviewers: Option<Vec<String>>` with `#[serde(default)]`
- [ ] MCP handler passes labels/reviewers through to `pr_create_if_gates_pass()`
- [ ] CLI unit test verifies flag parsing
- [ ] MCP test verifies param deserialization (additive — existing `pr_create` presence test still passes)
- [ ] `just ready` passes (fmt, lint, test, deny)

## Verification

- `just ready` — full quality gate passes
- `cargo test -p assay-cli` — CLI tests pass including new flag test
- `cargo test -p assay-mcp` — MCP tests pass including new param test
- `assay pr create --help` shows `--label` and `--reviewer` flags in output

## Observability Impact

- None — CLI and MCP are thin wiring layers; observability is in the core function (T02)

## Inputs

- `crates/assay-cli/src/commands/pr.rs` — existing `PrCommand::Create` with milestone, title, body
- `crates/assay-mcp/src/server.rs` — existing `PrCreateParams` with milestone_slug, title, body
- `crates/assay-core/src/pr.rs` — updated `pr_create_if_gates_pass()` with `extra_labels`/`extra_reviewers` params (from T02)

## Expected Output

- `crates/assay-cli/src/commands/pr.rs` — `PrCommand::Create` with `label` and `reviewer` Vec<String> fields + updated `pr_create_cmd`
- `crates/assay-mcp/src/server.rs` — `PrCreateParams` with `labels` and `reviewers` optional fields + updated handler
- CLI and MCP tests proving the wiring works
