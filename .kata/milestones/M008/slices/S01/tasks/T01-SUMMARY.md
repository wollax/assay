---
id: T01
parent: S01
milestone: M008
provides:
  - Milestone.pr_labels Option<Vec<String>> with serde(default, skip_serializing_if)
  - Milestone.pr_reviewers Option<Vec<String>> with serde(default, skip_serializing_if)
  - Milestone.pr_body_template Option<String> with serde(default, skip_serializing_if)
  - Schema snapshot updated for Milestone type
  - milestone_toml_roundtrip_with_pr_config test proving new fields round-trip
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-types/src/snapshots/
key_decisions:
  - "Used D092/D117 pattern (serde default + skip_serializing_if) for backward compatibility — existing TOML without new fields loads without error"
patterns_established:
  - "Same optional-field-with-serde-default pattern as ProviderConfig (D092)"
observability_surfaces:
  - none — pure type extension
duration: 10min
verification_result: passed
completed_at: 2026-03-23T19:30:00Z
blocker_discovered: false
---

# T01: Milestone type extension + TOML round-trip

**Added pr_labels, pr_reviewers, pr_body_template to Milestone with backward-compatible serde defaults and schema snapshot update**

## What Happened

Extended the Milestone struct in assay-types with three new optional fields: `pr_labels: Option<Vec<String>>`, `pr_reviewers: Option<Vec<String>>`, and `pr_body_template: Option<String>`. All use `#[serde(default, skip_serializing_if = "Option::is_none")]` following the D092 pattern. Updated the insta schema snapshot. Added `milestone_toml_roundtrip_with_pr_config` test that constructs a Milestone with all three fields set, serializes to TOML, deserializes back, and asserts equality. Existing backward-compat tests continue to pass — TOML files without these fields load as None.

## Verification

- `cargo test -p assay-types` — all 9 milestone tests pass including new roundtrip test
- Schema snapshot updated and accepted
- Existing TOML without new fields loads without error (backward compat confirmed)

## Deviations
None.

## Files Created/Modified
- `crates/assay-types/src/milestone.rs` — added 3 fields to Milestone struct + roundtrip test
- `crates/assay-types/src/snapshots/` — updated Milestone schema snapshot
