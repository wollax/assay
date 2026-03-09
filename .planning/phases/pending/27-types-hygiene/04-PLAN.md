---
phase: 27
plan: 4
wave: 2
depends_on: [1]
files_modified:
  - crates/assay-types/src/criterion.rs
  - crates/assay-types/src/gates_spec.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-core/src/gate/mod.rs
  - crates/assay-core/src/spec/mod.rs
autonomous: true
source_issue: null
must_haves:
  truths:
    - "GateCriterion is a type alias for Criterion (backward compatible)"
    - "Criterion has a requirements field with #[serde(default, skip_serializing_if = 'Vec::is_empty')]"
    - "Existing TOML/JSON without requirements field deserializes correctly"
    - "CriterionResult naming reviewed and confirmed appropriate"
    - "Schema registry entries updated for merged type"
  artifacts:
    - path: "crates/assay-types/src/criterion.rs"
      provides: "Criterion with added requirements field"
    - path: "crates/assay-types/src/gates_spec.rs"
      provides: "GateCriterion as type alias for Criterion"
    - path: "crates/assay-core/src/gate/mod.rs"
      provides: "Simplified to_criterion() (now trivial clone or removed)"
  key_links:
    - from: "criterion.rs (Criterion with requirements)"
      to: "gates_spec.rs (type alias GateCriterion = Criterion)"
      via: "GateCriterion becomes alias after Criterion gains requirements field"
    - from: "gates_spec.rs (type alias)"
      to: "assay-core gate/mod.rs and spec/mod.rs"
      via: "All GateCriterion usage sites continue to work via type alias"
    - from: "criterion.rs (Criterion)"
      to: "gate_run.rs (CriterionResult)"
      via: "CriterionResult naming reviewed — it names a result, not a criterion; name is appropriate"
---

<objective>
Merge GateCriterion into Criterion by adding a `requirements` field to Criterion, then make GateCriterion a type alias. This eliminates structural duplication (TYPE-05) while maintaining backward compatibility via serde defaults. Also review CriterionResult naming per the locked requirement.
</objective>

<context>
@crates/assay-types/src/criterion.rs
@crates/assay-types/src/gates_spec.rs
@crates/assay-types/src/gate_run.rs
@crates/assay-types/src/lib.rs
@crates/assay-core/src/gate/mod.rs
@crates/assay-core/src/spec/mod.rs
</context>

<task type="auto">
  <name>Task 1: Add requirements field to Criterion and create GateCriterion type alias</name>
  <files>
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-types/src/lib.rs
  </files>
  <action>
    1. In `criterion.rs`, add a `requirements` field to `Criterion`:
       ```rust
       /// Requirement IDs this criterion traces to (e.g., `["REQ-FUNC-001"]`).
       #[serde(default, skip_serializing_if = "Vec::is_empty")]
       pub requirements: Vec<String>,
       ```
       Place it as the last field, after `prompt`.

    2. In `gates_spec.rs`:
       - Remove the `GateCriterion` struct definition entirely.
       - Replace it with a type alias: `pub type GateCriterion = crate::Criterion;`
       - Keep the doc comment explaining it's an alias for backward compatibility.
       - Remove the `gate-criterion` schema registry entry (it's now identical to `criterion`).
       - Update imports: remove `CriterionKind` and `Enforcement` imports if no longer needed by this file (GatesSpec still uses GateSection).

    3. In `lib.rs`:
       - Verify `GateCriterion` is still re-exported from `gates_spec` (the type alias should work).

    4. Update ALL test code that constructs `Criterion` to include `requirements: vec![]` (or update existing tests that already use the struct literal syntax).

    5. Update ALL test code that constructs `GateCriterion` — these now construct `Criterion` via the alias, so the field set must match Criterion exactly.

    6. Verify backward compatibility: TOML/JSON without a `requirements` field must still deserialize to Criterion with an empty vec (ensured by `#[serde(default)]`).
  </action>
  <verify>
    cargo test -p assay-types 2>&1 | tail -30
    cargo build -p assay-types 2>&1 | tail -10
  </verify>
  <done>
    - Criterion has a `requirements: Vec<String>` field with serde defaults
    - GateCriterion is `pub type GateCriterion = Criterion`
    - All assay-types tests pass
    - Existing TOML without `requirements` deserializes correctly
  </done>
</task>

<task type="auto">
  <name>Task 2: Update assay-core to work with merged Criterion type</name>
  <files>
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-core/src/spec/mod.rs
  </files>
  <action>
    1. In `gate/mod.rs`:
       - Simplify `to_criterion()`: since GateCriterion IS Criterion now, `to_criterion()` can just clone the input. Consider removing it entirely and using `.clone()` at call sites, OR keep it as a trivial wrapper if it aids readability.
       - Update any `GateCriterion` struct literal construction in tests to use the full Criterion field set (including `requirements: vec![]`).

    2. In `spec/mod.rs`:
       - Update any `GateCriterion` struct literal construction in tests to include `requirements: vec![]`.
       - The `is_executable` closure that takes `&GateCriterion` continues to work since it's now `&Criterion`.

    3. Review `CriterionResult` naming (locked requirement):
       - `CriterionResult` in `gate_run.rs` pairs a criterion name with its evaluation result. The name accurately describes "the result of evaluating a criterion." Confirm this is appropriate and document the review in a code comment if helpful.
  </action>
  <verify>
    cargo test --workspace 2>&1 | tail -30
    cargo clippy --workspace -- -D warnings 2>&1 | head -30
  </verify>
  <done>
    - `to_criterion()` is simplified or removed
    - All workspace tests pass
    - All GateCriterion usages in assay-core compile via type alias
    - CriterionResult naming reviewed and confirmed
  </done>
</task>

<task type="auto">
  <name>Task 3: Update schema registry and verify schema output</name>
  <files>
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-types/src/criterion.rs
  </files>
  <action>
    1. Verify the `criterion` schema registry entry now includes `requirements` as an optional array field.
    2. Remove the duplicate `gate-criterion` schema registry entry from `gates_spec.rs` (if not already done in Task 1).
    3. Run the schema generation to verify output is correct:
       ```bash
       cargo run -p assay-cli -- schema criterion
       ```
    4. Verify the `gates-spec` schema still works correctly (GatesSpec.criteria is now Vec<Criterion>).
  </action>
  <verify>
    cargo run -p assay-cli -- schema criterion 2>&1 | grep -c "requirements"
    cargo test --workspace 2>&1 | tail -20
  </verify>
  <done>
    - Schema for `criterion` includes optional `requirements` array
    - No duplicate `gate-criterion` schema entry
    - `gates-spec` schema references merged Criterion type
  </done>
</task>

<verification>
```bash
just ready
```
</verification>

<success_criteria>
- [ ] `GateCriterion` is a type alias for `Criterion`
- [ ] `Criterion` has `requirements: Vec<String>` with serde default
- [ ] Backward-compatible: TOML/JSON without `requirements` still deserializes
- [ ] `to_criterion()` simplified or removed
- [ ] `CriterionResult` naming reviewed and documented
- [ ] Schema registry updated (no duplicate entries)
- [ ] `just ready` passes
</success_criteria>
