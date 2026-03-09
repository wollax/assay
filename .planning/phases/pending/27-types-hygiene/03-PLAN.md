---
phase: 27
plan: 3
wave: 1
depends_on: []
files_modified:
  - crates/assay-types/src/lib.rs
  - crates/assay-types/src/context.rs
  - crates/assay-types/src/enforcement.rs
  - crates/assay-types/src/feature_spec.rs
  - crates/assay-types/src/gate.rs
  - crates/assay-types/src/gate_run.rs
  - crates/assay-types/src/checkpoint.rs
  - crates/assay-types/src/session.rs
  - crates/assay-types/src/schema_registry.rs
autonomous: true
source_issue: null
must_haves:
  truths:
    - "cargo doc --no-deps produces zero 'missing documentation' warnings for assay-types public items"
    - "#![deny(missing_docs)] is active at the crate level in assay-types"
    - "EnforcementSummary fields have doc comments visible in generated docs"
  artifacts:
    - path: "crates/assay-types/src/lib.rs"
      provides: "#![deny(missing_docs)] crate attribute and doc comments on all public items in lib.rs"
    - path: "crates/assay-types/src/enforcement.rs"
      provides: "Doc comments on EnforcementSummary fields (TYPE-06)"
    - path: "crates/assay-types/src/feature_spec.rs"
      provides: "Doc comments on all 27 undocumented enum variants"
    - path: "crates/assay-types/src/context.rs"
      provides: "Doc comments on ContentBlock variant fields"
  key_links:
    - from: "lib.rs (#![deny(missing_docs)])"
      to: "all source files"
      via: "Crate-level deny means ALL public items must have docs or build fails"
---

<objective>
Add doc comments to all public items in assay-types and enable `#![deny(missing_docs)]` at the crate level. This fulfills TYPE-03 and TYPE-06. The deny attribute must be added in the SAME commit as all doc comments to avoid breaking the build.
</objective>

<context>
@crates/assay-types/src/lib.rs
@crates/assay-types/src/context.rs
@crates/assay-types/src/enforcement.rs
@crates/assay-types/src/feature_spec.rs
@crates/assay-types/src/gate.rs
@crates/assay-types/src/gate_run.rs
@crates/assay-types/src/checkpoint.rs
@crates/assay-types/src/session.rs
@crates/assay-types/src/schema_registry.rs
</context>

<task type="auto">
  <name>Task 1: Add doc comments to all undocumented public items</name>
  <files>
    - crates/assay-types/src/lib.rs
    - crates/assay-types/src/context.rs
    - crates/assay-types/src/enforcement.rs
    - crates/assay-types/src/feature_spec.rs
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/src/checkpoint.rs
    - crates/assay-types/src/session.rs
    - crates/assay-types/src/schema_registry.rs
  </files>
  <action>
    First, run `RUSTDOCFLAGS="-W missing_docs" cargo doc --no-deps -p assay-types 2>&1` to get the exact list of missing docs.

    Then add doc comments to every reported item. Expected locations (from research — 47 warnings):

    **lib.rs (~9 items):**
    - Add crate-level doc comment `//! ...` if missing
    - Add doc comments on `Gate` fields: `name`, `passed`
    - Add doc comments on `Review` fields: `spec_name`, `approved`, `comments`
    - Add doc comments on `Workflow` fields: `name`, `specs`, `gates`

    **context.rs (~7 items):**
    - ContentBlock variant fields: `Text::text`, `Thinking::thinking`, `ToolUse::id`, `ToolUse::name`, `ToolUse::input`, `ToolResult::tool_use_id`, `ToolResult::content`
    - Any other undocumented public items

    **enforcement.rs (~4 items):**
    - `EnforcementSummary` fields: `required_passed`, `required_failed`, `advisory_passed`, `advisory_failed` (TYPE-06)

    **feature_spec.rs (~27 items):**
    - All enum variants that lack doc comments across: `SpecStatus`, `Obligation`, `Priority`, `VerificationMethod`, `AcceptanceCriterionType`, `ImpactLevel`, `LikelihoodLevel`
    - Check struct fields as well

    **gate.rs, gate_run.rs, checkpoint.rs, session.rs, schema_registry.rs:**
    - Any remaining undocumented public items (check rustdoc output)

    Keep doc comments concise — one line is sufficient for obvious fields. Use the existing style in the codebase as a guide.
  </action>
  <verify>
    RUSTDOCFLAGS="-W missing_docs" cargo doc --no-deps -p assay-types 2>&1 | grep "warning" | wc -l
  </verify>
  <done>
    - Zero missing_docs warnings from cargo doc
    - All EnforcementSummary fields have doc comments
    - All feature_spec.rs enum variants have doc comments
    - All ContentBlock variant fields have doc comments
  </done>
</task>

<task type="auto">
  <name>Task 2: Enable #![deny(missing_docs)] at crate level</name>
  <files>
    - crates/assay-types/src/lib.rs
  </files>
  <action>
    Add `#![deny(missing_docs)]` at the top of `crates/assay-types/src/lib.rs`, before any `pub mod` declarations.

    This MUST happen after Task 1 is complete (all docs added). If any public item is missing a doc comment, the build will fail immediately.
  </action>
  <verify>
    cargo build -p assay-types 2>&1 | tail -20
    cargo doc --no-deps -p assay-types 2>&1 | tail -20
  </verify>
  <done>
    - `#![deny(missing_docs)]` is present in lib.rs
    - `cargo build -p assay-types` succeeds
    - `cargo doc --no-deps -p assay-types` produces zero warnings
  </done>
</task>

<verification>
```bash
just ready
```
</verification>

<success_criteria>
- [ ] `#![deny(missing_docs)]` present at crate level in assay-types
- [ ] `cargo doc --no-deps -p assay-types` produces zero missing documentation warnings
- [ ] EnforcementSummary fields have doc comments visible in generated docs
- [ ] All feature_spec.rs enum variants documented
- [ ] `just ready` passes
</success_criteria>
