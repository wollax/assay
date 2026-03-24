---
estimated_steps: 6
estimated_files: 3
---

# T01: Milestone type extension + TOML round-trip

**Slice:** S01 — Advanced PR creation with labels, reviewers, and templates
**Milestone:** M008

## Description

Add three new optional fields to the `Milestone` struct in assay-types: `pr_labels`, `pr_reviewers`, `pr_body_template`. All must follow the D092/D117 backward-compatibility pattern (serde(default, skip_serializing_if)). Update the insta schema snapshot and write TOML round-trip tests proving existing milestones without these fields load without error.

## Steps

1. Read `crates/assay-types/src/milestone.rs` to understand the current Milestone struct and its serde attributes
2. Add `pr_labels: Option<Vec<String>>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` after the existing `pr_url` field
3. Add `pr_reviewers: Option<Vec<String>>` with the same serde attributes
4. Add `pr_body_template: Option<String>` with the same serde attributes
5. Run `cargo test -p assay-types` to verify existing tests still pass (backward compat), then `cargo insta review` to update the Milestone schema snapshot
6. Add a new test `milestone_toml_roundtrip_with_pr_config` that constructs a Milestone with all three fields set, serializes to TOML, deserializes back, and asserts equality. Also verify the existing `milestone_toml_roundtrip_without_optional_fields` test still passes (fields absent → None).

## Must-Haves

- [ ] `Milestone` struct has `pr_labels: Option<Vec<String>>`, `pr_reviewers: Option<Vec<String>>`, `pr_body_template: Option<String>` fields
- [ ] All three fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` (Vec variant uses `skip_serializing_if = "Option::is_none"`)
- [ ] Existing Milestone TOML without these fields loads without error (backward compat test passes)
- [ ] New round-trip test proves pr_labels/pr_reviewers/pr_body_template serialize and deserialize correctly
- [ ] Schema snapshot updated via `cargo insta review`
- [ ] `cargo test -p assay-types` passes with zero failures

## Verification

- `cargo test -p assay-types` — all existing + new tests pass
- `cargo insta test -p assay-types` — snapshots accepted
- `rg "pr_labels" crates/assay-types/src/milestone.rs` confirms field exists with correct serde attrs

## Observability Impact

- None — this is a type extension with no runtime behavior change

## Inputs

- `crates/assay-types/src/milestone.rs` — existing Milestone struct with pr_branch, pr_base, pr_number, pr_url fields
- D117 decision: use D092 pattern (serde(default, skip_serializing_if))
- D092 reference: ProviderConfig addition to Config for the exact pattern to follow

## Expected Output

- `crates/assay-types/src/milestone.rs` — Milestone struct extended with 3 new fields
- `crates/assay-types/src/snapshots/` — updated Milestone schema snapshot
- New test(s) in milestone.rs proving round-trip with and without the new fields
